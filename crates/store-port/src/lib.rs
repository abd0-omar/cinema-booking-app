//! Store traits for cinema booking persistence backends (database, Redis seat holds, etc.).

pub mod booking;

pub use booking::{
    BookingStore, BookingStoreError, SeatHoldChangeset, SeatHoldStore, SeatReservationSession,
    SeatReservationStatus,
};
pub use cinema_booking_db::entities::bookings::{Booking, BookingChangeset};
