//! Server-rendered HTML and Datastar (SSE) handlers.

use crate::error::Error;
use crate::middlewares::auth::TB_ACCESS_TOKEN_COOKIE;
use crate::seat_states::{merge_movie_seats, seat_grid_element_html, seat_grid_error_html};
use crate::state::SharedAppState;
use crate::templates::{CinemaIndex, LoginPage, SignupPage};
use askama::Template;
use async_stream::stream;
use axum::extract::{Form, Query, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse, Redirect, Response};
use cinema_booking_auth_adapter_trailbase::trailbase_client::Error as TrailbaseClientError;
use cinema_booking_db::entities::movies;
use cookie::time::Duration as CookieDuration;
use cookie::{Cookie, SameSite};
use datastar::{axum::ReadSignals, prelude::PatchElements};
use reqwest::StatusCode as HttpStatus;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::time::Duration;

const HELLO_MESSAGE: &str = "Hello, world!";

// Safety cap for long-lived SSE seat-map streams when the client
// does not explicitly provide a `max_ticks` value.
// At the default 2s interval this is ~5 minutes of updates.
const DEFAULT_SEAT_MAP_MAX_TICKS: u64 = 150;

/// `GET /` — Askama-rendered cinema shell.
pub async fn cinema_index(State(state): State<SharedAppState>) -> Result<Html<String>, Error> {
    let id = uuid::Uuid::new_v4();
    let compact = id.simple().to_string();
    let short = compact.chars().take(12).collect::<String>();
    let movies = movies::load_all(&state.db_pool).await?;
    let page = CinemaIndex {
        user_label: format!("user: {short}"),
        viewer_uuid: id.to_string(),
        movies,
    };
    Ok(Html(page.render()?))
}

fn default_seat_map_interval_ms() -> u64 {
    2000
}

#[derive(Debug, Deserialize)]
pub struct SeatMapSignals {
    pub movie_slug: String,
    #[serde(default)]
    pub viewer: Option<String>,
    #[serde(default = "default_seat_map_interval_ms")]
    pub interval_ms: u64,
    #[serde(default)]
    pub max_ticks: Option<u64>,
}

async fn seat_map_grid_html(state: &SharedAppState, movie_slug: &str, viewer: &str) -> String {
    let slug = movie_slug.trim();
    if slug.is_empty() {
        return seat_grid_error_html("Pick a film from the list.");
    }

    let movie = match movies::load(slug, &state.db_pool).await {
        Ok(m) => m,
        Err(cinema_booking_db::Error::NoRecordFound) => {
            return seat_grid_error_html("Film not found.");
        }
        Err(_) => return seat_grid_error_html("Could not load film."),
    };

    let merged = match merge_movie_seats(
        &movie,
        viewer,
        &state.db_pool,
        state.seat_hold_store.as_ref(),
    )
    .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(error = ?e, "seat_map merge failed");
            return seat_grid_error_html("Could not load seat map.");
        }
    };

    seat_grid_element_html(&movie, &merged.seats)
}

/// `GET /seat-map-snapshot` — one-off seat grid HTML for polling.
pub async fn seat_map_snapshot(
    State(state): State<SharedAppState>,
    Query(params): Query<SeatMapSignals>,
) -> Result<Html<String>, Error> {
    let html = seat_map_grid_html(&state, &params.movie_slug, params.viewer.as_deref().unwrap_or_default()).await;
    Ok(Html(html))
}

/// Datastar SSE: patches `#seatGrid` on an interval from merged bookings + Redis holds.
pub async fn ds_seat_map(
    State(state): State<SharedAppState>,
    ReadSignals(signals): ReadSignals<SeatMapSignals>,
) -> impl IntoResponse {
    let state = state.clone();
    let movie_slug = signals.movie_slug;
    let viewer = signals.viewer.unwrap_or_default();
    let interval_ms = signals.interval_ms.max(50);
    let max_ticks = signals.max_ticks.unwrap_or(DEFAULT_SEAT_MAP_MAX_TICKS);

    let stream = stream! {
        let mut tick: u64 = 0;
        loop {
            let html = seat_map_grid_html(&state, &movie_slug, &viewer).await;
            yield Ok::<Event, Infallible>(PatchElements::new(html).into());
            tick += 1;
            if tick >= max_ticks {
                break;
            }
            tokio::time::sleep(Duration::from_millis(interval_ms)).await;
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
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
    pub r: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoginForm {
    pub email: String,
    pub password: String,
}

/// `GET /login` — Trailbase password login form.
pub async fn login_get(Query(q): Query<LoginQuery>) -> Result<Html<String>, Error> {
    let error_message = login_error_display(q.e.as_deref());
    let success_message = login_success_display(q.r.as_deref());
    let page = LoginPage {
        error_message,
        success_message,
    };
    Ok(Html(page.render()?))
}

/// `POST /login` — Proxies to Trailbase `POST /api/auth/v1/login`, sets session cookie on success.
pub async fn login_post(
    State(state): State<SharedAppState>,
    Form(form): Form<LoginForm>,
) -> Response {
    let Some(ref base_url) = state.trailbase_base_url else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "Trailbase base URL is not configured (set trailbase.base_url).",
        )
            .into_response();
    };

    let client = match cinema_booking_auth_adapter_trailbase::trailbase_http_client(base_url) {
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

fn login_success_display(code: Option<&str>) -> Option<&'static str> {
    match code {
        Some("1") => Some(
            "Registration accepted. Sign in when your account is ready (check email if verification is enabled).",
        ),
        _ => None,
    }
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

#[derive(Debug, Deserialize)]
pub struct SignupQuery {
    pub e: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SignupForm {
    pub email: String,
    pub password: String,
    pub password_repeat: String,
}

#[derive(Serialize)]
struct TrailbaseRegisterBody<'a> {
    email: &'a str,
    password: &'a str,
    password_repeat: &'a str,
}

/// `GET /signup` — Trailbase registration form.
pub async fn signup_get(Query(q): Query<SignupQuery>) -> Result<Html<String>, Error> {
    let error_message = signup_error_display(q.e.as_deref());
    let page = SignupPage { error_message };
    Ok(Html(page.render()?))
}

/// `POST /signup` — Proxies to Trailbase `POST /api/auth/v1/register`.
pub async fn signup_post(
    State(state): State<SharedAppState>,
    Form(form): Form<SignupForm>,
) -> Response {
    if form.password != form.password_repeat {
        return redirect_signup_error("mismatch");
    }

    let Some(ref base_url) = state.trailbase_base_url else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "Trailbase base URL is not configured (set trailbase.base_url).",
        )
            .into_response();
    };

    let register_url = format!("{}/api/auth/v1/register", base_url.trim_end_matches('/'));

    let http = match reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(?e, "reqwest client build");
            return redirect_signup_error("failed");
        }
    };

    let email = form.email.trim();
    let body = TrailbaseRegisterBody {
        email,
        password: &form.password,
        password_repeat: &form.password_repeat,
    };

    let response = match http.post(&register_url).json(&body).send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(?e, "trailbase register request");
            return redirect_signup_error("failed");
        }
    };

    let status = response.status();

    if status.is_success() || status == HttpStatus::SEE_OTHER {
        return Redirect::to("/login?r=1").into_response();
    }

    if status == HttpStatus::FAILED_DEPENDENCY {
        return redirect_signup_error("email");
    }

    if matches!(
        status,
        HttpStatus::UNAUTHORIZED | HttpStatus::UNPROCESSABLE_ENTITY | HttpStatus::BAD_REQUEST
    ) {
        return redirect_signup_error("policy");
    }

    tracing::warn!(%status, "trailbase register unexpected status");
    redirect_signup_error("failed")
}

fn signup_error_display(code: Option<&str>) -> Option<&'static str> {
    match code {
        Some("mismatch") => Some("Passwords do not match."),
        Some("email") => Some(
            "Could not send verification email. Check Trailbase mail settings or try again later.",
        ),
        Some("policy") => Some("Password or email did not meet Trailbase requirements."),
        Some("failed") => Some("Registration failed. Try again later."),
        _ => None,
    }
}

fn redirect_signup_error(code: &'static str) -> Response {
    Redirect::to(format!("/signup?e={code}").as_str()).into_response()
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
