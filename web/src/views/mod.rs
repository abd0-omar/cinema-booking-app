//! Server-rendered HTML and Datastar (SSE) handlers.

use crate::error::Error;
use crate::middlewares::auth::TB_ACCESS_TOKEN_COOKIE;
use crate::state::SharedAppState;
use crate::templates::{CinemaIndex, LoginPage};
use askama::Template;
use async_stream::stream;
use axum::extract::{Form, Query, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse, Redirect, Response};
use cinema_booking_trailbase::trailbase_client::Error as TrailbaseClientError;
use cookie::time::Duration as CookieDuration;
use cookie::{Cookie, SameSite};
use datastar::{axum::ReadSignals, prelude::PatchElements};
use reqwest::StatusCode as HttpStatus;
use serde::Deserialize;
use std::convert::Infallible;
use std::time::Duration;

const HELLO_MESSAGE: &str = "Hello, world!";

/// `GET /` — Askama-rendered cinema shell.
pub async fn cinema_index() -> Result<Html<String>, Error> {
    let id = uuid::Uuid::new_v4();
    let compact = id.simple().to_string();
    let short = compact.chars().take(12).collect::<String>();
    let page = CinemaIndex {
        user_label: format!("user: {short}"),
    };
    Ok(Html(page.render()?))
}

#[derive(Debug, Deserialize)]
pub struct HelloSignals {
    pub delay: u64,
}

/// Datastar action target: streams [`PatchElements`] that progressively update `#message`.
pub async fn ds_hello_world(ReadSignals(signals): ReadSignals<HelloSignals>) -> impl IntoResponse {
    let delay_ms = signals.delay.max(1);
    let stream = stream! {
        for i in 0..HELLO_MESSAGE.len() {
            let slice = &HELLO_MESSAGE[..=i];
            let elements = format!("<div id=\"message\">{slice}</div>");
            let patch = PatchElements::new(elements);
            yield Ok::<Event, Infallible>(patch.into());
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        }
    };
    Sse::new(stream).keep_alive(KeepAlive::default())
}

#[derive(Debug, Deserialize)]
pub struct LoginQuery {
    pub e: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoginForm {
    pub email: String,
    pub password: String,
}

/// `GET /login` — Trailbase password login form.
pub async fn login_get(Query(q): Query<LoginQuery>) -> Result<Html<String>, Error> {
    let error_message = login_error_display(q.e.as_deref());
    let page = LoginPage { error_message };
    Ok(Html(page.render()?))
}

/// `POST /login` — Proxies to Trailbase `POST /api/auth/v1/login`, sets session cookie on success.
pub async fn login_post(State(state): State<SharedAppState>, Form(form): Form<LoginForm>) -> Response {
    let Some(ref base_url) = state.trailbase_base_url else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "Trailbase base URL is not configured (set trailbase.base_url).",
        )
            .into_response();
    };

    let client = match cinema_booking_trailbase::trailbase_http_client(base_url) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(?e, "trailbase_http_client");
            return redirect_login_error("failed");
        }
    };

    let email = form.email.trim();
    match client.login(email, &form.password).await {
        Ok(None) => {
            let Some(tokens) = client.tokens() else {
                tracing::error!("login succeeded but no tokens on client");
                return redirect_login_error("failed");
            };
            set_session_cookie_redirect(&tokens.auth_token, Redirect::to("/"))
        }
        Ok(Some(_)) => redirect_login_error("mfa"),
        Err(e) => {
            if let TrailbaseClientError::HttpStatus(s) = &e {
                if *s == HttpStatus::UNAUTHORIZED {
                    return redirect_login_error("invalid");
                }
            }
            tracing::warn!(?e, "trailbase login");
            redirect_login_error("failed")
        }
    }
}

/// `POST /logout` — Clears the session cookie.
pub async fn logout_post() -> Response {
    let cookie = Cookie::build((TB_ACCESS_TOKEN_COOKIE, ""))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(CookieDuration::ZERO)
        .build();
    let mut res = Redirect::to("/login").into_response();
    if let Ok(hv) = HeaderValue::from_str(&cookie.to_string()) {
        res.headers_mut().insert(header::SET_COOKIE, hv);
    }
    res
}

fn login_error_display(code: Option<&str>) -> Option<&'static str> {
    match code {
        Some("invalid") => Some("Invalid email or password."),
        Some("mfa") => Some(
            "This account uses MFA. Use a non-MFA user or obtain a token via Trailbase or Hurl.",
        ),
        Some("failed") => Some("Sign-in failed. Try again later."),
        _ => None,
    }
}

fn set_session_cookie_redirect(token: &str, redirect: Redirect) -> Response {
    let cookie = Cookie::build((TB_ACCESS_TOKEN_COOKIE, token.to_string()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(CookieDuration::seconds(60 * 60 * 24 * 7))
        .build();
    let mut res = redirect.into_response();
    if let Ok(hv) = HeaderValue::from_str(&cookie.to_string()) {
        res.headers_mut().insert(header::SET_COOKIE, hv);
    } else {
        tracing::error!("invalid Set-Cookie header value after login");
    }
    res
}

fn redirect_login_error(code: &'static str) -> Response {
    Redirect::to(format!("/login?e={code}").as_str()).into_response()
}
