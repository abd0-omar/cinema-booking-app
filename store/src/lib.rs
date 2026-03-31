//! Store traits for cinema booking persistence backends (database, in-memory, etc.).

pub mod booking;

pub use booking::{BookingStore, BookingStoreError};
pub use cinema_booking_db::entities::bookings::{Booking, BookingChangeset, BookingStatus};
