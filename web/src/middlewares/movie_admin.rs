use axum::body::Body;
use axum::{
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use cinema_booking_auth::{Principal, Role};

/// Requires [`Principal`] from upstream [`crate::middlewares::auth::auth`] with [`Role::Admin`].
///
/// Used for movie mutation routes only; must be layered **inside** `auth` so the principal is present.
pub async fn require_movie_admin(req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    let Some(principal) = req.extensions().get::<Principal>() else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    if principal.role() != Role::Admin {
        tracing::info!("Forbidden request");
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(next.run(req).await)
}
