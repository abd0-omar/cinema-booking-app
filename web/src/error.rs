use axum::{http::StatusCode, response::IntoResponse};
use cinema_booking_store_port::BookingStoreError;
use std::fmt::{Debug, Display};

/// Error type that encapsultes anything that can go wrong
/// in this application. Implements [IntoResponse],
/// so that it can be returned directly from a request handler.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Authenticated caller is not allowed to perform this action.
    #[error("Forbidden")]
    Forbidden,
    /// Errors that can occur as a result of a data layer operation.
    #[error("Database error")]
    Database(#[from] cinema_booking_db::Error),
    /// Askama template rendering failed.
    #[error("Template error")]
    Template(#[from] askama::Error),
    /// Redis seat hold / confirm failures (`SeatHoldStore`).
    #[error("Seat hold store error")]
    BookingStore(#[from] BookingStoreError),
    /// Any other error. Handled as an Internal Server Error.
    #[error("Error: {0}")]
    Other(#[from] anyhow::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        match self {
            Error::Forbidden => {
                tracing::info!("Forbidden request");
                StatusCode::FORBIDDEN.into_response()
            }
            Error::Database(cinema_booking_db::Error::NoRecordFound) => {
                StatusCode::NOT_FOUND.into_response()
            }
            Error::Database(cinema_booking_db::Error::ValidationError(e)) => {
                validation_error(e).into_response()
            }
            Error::Database(cinema_booking_db::Error::DbError(e)) => {
                internal_error(e).into_response()
            }
            Error::BookingStore(e) => match e {
                BookingStoreError::Validation(msg) => {
                    tracing::info!(%msg, "Seat hold validation failed");
                    (StatusCode::UNPROCESSABLE_ENTITY, msg).into_response()
                }
                BookingStoreError::NotFound | BookingStoreError::SessionNotFound => {
                    StatusCode::NOT_FOUND.into_response()
                }
                BookingStoreError::SessionOwnershipMismatch => {
                    StatusCode::FORBIDDEN.into_response()
                }
                BookingStoreError::Conflict(msg) => {
                    tracing::info!(%msg, "Seat hold conflict");
                    StatusCode::CONFLICT.into_response()
                }
                BookingStoreError::SeatUnavailable => StatusCode::CONFLICT.into_response(),
                BookingStoreError::Serialization(msg) | BookingStoreError::Internal(msg) => {
                    internal_error(&msg).into_response()
                }
            },
            Error::Template(e) => internal_error(e).into_response(),
            Error::Other(e) => internal_error(e).into_response(),
        }
    }
}

/// Helper function to create an internal error response while
/// taking care to log the error itself.
fn internal_error<E>(e: E) -> StatusCode
where
    // Some "error-like" types (e.g. `anyhow::Error`) don't implement the error trait, therefore
    // we "downgrade" to simply requiring `Debug` and `Display`, the traits
    // we actually need for logging purposes.
    E: Debug + Display,
{
    tracing::error!(err.msg = %e, err.details = ?e, "Internal server error");
    // We don't want to leak internal implementation details to the client
    // via the error response, so we just return an opaque internal server.
    StatusCode::INTERNAL_SERVER_ERROR
}

/// Helper function to create an unprocessable entity error response while
/// taking care to log the error itself.
fn validation_error(e: validator::ValidationErrors) -> (StatusCode, String) {
    tracing::info!(err.msg = %e, err.details = ?e, "Validation failed");
    (StatusCode::UNPROCESSABLE_ENTITY, e.to_string())
}
