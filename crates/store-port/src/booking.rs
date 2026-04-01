use async_trait::async_trait;
use cinema_booking_db::entities::bookings::{Booking, BookingChangeset};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors returned by booking persistence and seat hold store implementations.
#[derive(Debug, Error)]
pub enum BookingStoreError {
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("no record found")]
    NotFound,
    #[error("{0}")]
    Conflict(String),
    #[error("seat unavailable")]
    SeatUnavailable,
    #[error("seat hold session not found")]
    SessionNotFound,
    #[error("seat hold session ownership mismatch")]
    SessionOwnershipMismatch,
    #[error("serialization failed: {0}")]
    Serialization(String),
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

/// State of a seat reservation session in Redis-backed hold/confirm flows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SeatReservationStatus {
    Held,
    Confirmed,
}

/// Payload stored as the seat lock value for hold/confirm flows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeatReservationSession {
    pub session_uuid: String,
    pub movie_uuid: String,
    pub seat_uuid: String,
    pub user_uuid: String,
    pub status: SeatReservationStatus,
}

/// Input for acquiring a seat hold session.
#[derive(Debug, Clone)]
pub struct SeatHoldChangeset {
    pub movie_uuid: String,
    pub seat_uuid: String,
    pub user_uuid: String,
}

impl SeatHoldChangeset {
    pub fn validate(&self) -> Result<(), BookingStoreError> {
        if self.movie_uuid.trim().is_empty() {
            return Err(BookingStoreError::Validation(
                "movie_uuid must not be empty".to_string(),
            ));
        }
        if self.seat_uuid.trim().is_empty() {
            return Err(BookingStoreError::Validation(
                "seat_uuid must not be empty".to_string(),
            ));
        }
        if self.user_uuid.trim().is_empty() {
            return Err(BookingStoreError::Validation(
                "user_uuid must not be empty".to_string(),
            ));
        }
        Ok(())
    }
}

/// Persists and queries durable seat bookings for movies.
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

/// Manages temporary seat hold sessions (e.g. in Redis) before durable checkout persistence.
#[async_trait]
pub trait SeatHoldStore: Send + Sync {
    /// Acquires a temporary seat hold session.
    ///
    /// Implementations should enforce single-winner semantics for a seat.
    async fn hold(
        &self,
        changeset: SeatHoldChangeset,
    ) -> Result<SeatReservationSession, BookingStoreError>;

    /// Confirms an existing seat hold for a user.
    ///
    /// Implementations should consume/remove the hold on success.
    /// Durable booking persistence is handled by the caller (e.g. SQLite write in checkout flow).
    async fn confirm(
        &self,
        movie_uuid: &str,
        seat_uuid: &str,
        user_uuid: &str,
    ) -> Result<SeatReservationSession, BookingStoreError>;
}
