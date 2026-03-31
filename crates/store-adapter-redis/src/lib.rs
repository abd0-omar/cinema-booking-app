//! Redis implementation of [`cinema_booking_store_port::BookingStore`].

mod redis_store;

pub use redis_store::{RedisBookingStore, RedisBookingStoreConfig};
