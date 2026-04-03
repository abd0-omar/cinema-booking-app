use crate::state::SharedAppState;
use axum::body::Body;
use axum::{
    extract::State,
    http::{self, header::HeaderMap, Request, StatusCode},
    middleware::Next,
    response::Response,
};
use tracing::Span;

/// HttpOnly session cookie set by [`crate::views::login_post`] after Trailbase password login.
pub const TB_ACCESS_TOKEN_COOKIE: &str = "tb_access_token";

/// Authenticates an incoming request using `Authorization: Bearer …` or the [`TB_ACCESS_TOKEN_COOKIE`] session cookie.
///
/// On success, inserts [`cinema_booking_auth::Principal`] into request extensions for downstream handlers.
#[tracing::instrument(skip_all, fields(rejection_reason = tracing::field::Empty))]
pub async fn auth(
    State(app_state): State<SharedAppState>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let jwt = match jwt_from_request(req.headers()) {
        Ok(jwt) => jwt,
        Err(reason) => {
            log_rejection_reason(reason);
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    match app_state.access_token_verifier.verify_bearer_token(jwt) {
        Ok(principal) => {
            req.extensions_mut().insert(principal);
            Ok(next.run(req).await)
        }
        Err(_) => {
            log_rejection_reason("Invalid or expired token");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

/// Reads and verifies principal from request headers when present.
///
/// Returns `None` when no auth is provided or verification fails.
pub fn extract_optional_principal_from_headers(
    headers: &HeaderMap,
    app_state: &SharedAppState,
) -> Option<cinema_booking_auth::Principal> {
    let jwt = jwt_from_request(headers).ok()?;
    app_state
        .access_token_verifier
        .verify_bearer_token(jwt)
        .ok()
}

fn jwt_from_request(headers: &HeaderMap) -> Result<&str, &'static str> {
    if let Some(auth_header) = headers
        .get(http::header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
    {
        return parse_bearer_token(auth_header).ok_or("Malformed bearer token");
    }
    session_jwt_from_cookie_header(headers).ok_or("Missing authorization header or session cookie")
}

fn parse_bearer_token(header_value: &str) -> Option<&str> {
    let rest = header_value.strip_prefix("Bearer")?;
    let jwt = rest.trim();
    if jwt.is_empty() {
        return None;
    }
    Some(jwt)
}

fn session_jwt_from_cookie_header(headers: &HeaderMap) -> Option<&str> {
    let raw = headers
        .get(http::header::COOKIE)
        .and_then(|header| header.to_str().ok())?;
    for part in raw.split(';') {
        let part = part.trim();
        let (name, value) = part.split_once('=')?;
        if name == TB_ACCESS_TOKEN_COOKIE {
            let value = value.trim();
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}

fn log_rejection_reason(msg: &str) {
    Span::current().record("rejection_reason", msg);
}
