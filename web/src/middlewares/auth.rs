use crate::state::SharedAppState;
use axum::body::Body;
use axum::{
    extract::State,
    http::{self, Request, StatusCode},
    middleware::Next,
    response::Response,
};
use tracing::Span;

/// Authenticates an incoming request using a Trailbase-style JWT in `Authorization: Bearer …`.
///
/// On success, inserts [`cinema_booking_auth::Principal`] into request extensions for downstream handlers.
#[tracing::instrument(skip_all, fields(rejection_reason = tracing::field::Empty))]
pub async fn auth(
    State(app_state): State<SharedAppState>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get(http::header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    let auth_header = if let Some(auth_header) = auth_header {
        auth_header
    } else {
        log_rejection_reason("Missing authorization header");
        return Err(StatusCode::UNAUTHORIZED);
    };

    let jwt = parse_bearer_token(auth_header).ok_or_else(|| {
        log_rejection_reason("Malformed bearer token");
        StatusCode::UNAUTHORIZED
    })?;

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

fn parse_bearer_token(header_value: &str) -> Option<&str> {
    let rest = header_value.strip_prefix("Bearer")?;
    let jwt = rest.trim();
    if jwt.is_empty() {
        return None;
    }
    Some(jwt)
}

fn log_rejection_reason(msg: &str) {
    Span::current().record("rejection_reason", msg);
}
