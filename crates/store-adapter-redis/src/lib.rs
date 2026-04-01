//! Redis implementation of [`cinema_booking_store_port::SeatHoldStore`].

mod redis_store;

pub use redis_store::{RedisSeatHoldStore, RedisSeatHoldStoreConfig};
