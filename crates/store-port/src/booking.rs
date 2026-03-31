use async_trait::async_trait;
use cinema_booking_db::entities::bookings::{Booking, BookingChangeset};
use thiserror::Error;

/// Errors returned by [`BookingStore`] implementations.
#[derive(Debug, Error)]
pub enum BookingStoreError {
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("no record found")]
    NotFound,
    #[error("{0}")]
    Conflict(String),
    #[error("internal error: {0}")]
    Internal(String),
}

impl From<cinema_booking_db::Error> for BookingStoreError {
    fn from(value: cinema_booking_db::Error) -> Self {
        match value {
            cinema_booking_db::Error::ValidationError(e) => Self::Validation(e.to_string()),
            cinema_booking_db::Error::NoRecordFound => Self::NotFound,
            cinema_booking_db::Error::DbError(e) => Self::Internal(e.to_string()),
        }
    }
}

/// Persists and queries seat bookings for movies.
#[async_trait]
pub trait BookingStore: Send + Sync {
    /// Creates a booking from a validated changeset; the store assigns identifiers as needed.
    async fn book(&self, changeset: BookingChangeset) -> Result<Booking, BookingStoreError>;

    /// Returns all bookings for the given movie (by its external UUID).
    async fn list_bookings_by_movie(
        &self,
        movie_uuid: &str,
    ) -> Result<Vec<Booking>, BookingStoreError>;
}
