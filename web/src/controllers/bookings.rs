//! JSON API for seat holds (Redis) and durable bookings (SQLite via [`cinema_booking_db`]).
use crate::{error::Error, state::SharedAppState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use cinema_booking_db::entities::bookings::{self, Booking, BookingChangeset};
use cinema_booking_store_port::{SeatHoldChangeset, SeatReservationSession};

/// Starts a TTL-backed seat hold in Redis.
#[axum::debug_handler]
pub async fn hold(
    State(app_state): State<SharedAppState>,
    Json(changeset): Json<SeatHoldChangeset>,
) -> Result<(StatusCode, Json<SeatReservationSession>), Error> {
    let session = app_state.seat_hold_store.hold(changeset).await?;
    Ok((StatusCode::CREATED, Json(session)))
}

/// Confirms the Redis hold and persists a booking in SQLite.
#[axum::debug_handler]
pub async fn checkout(
    State(app_state): State<SharedAppState>,
    Json(body): Json<SeatHoldChangeset>,
) -> Result<(StatusCode, Json<Booking>), Error> {
    let session = app_state
        .seat_hold_store
        .confirm(&body.movie_uuid, &body.seat_uuid, &body.user_uuid)
        .await?;
    let booking = bookings::create(
        BookingChangeset {
            movie_uuid: session.movie_uuid,
            seat_uuid: session.seat_uuid,
            user_uuid: session.user_uuid,
        },
        &app_state.db_pool,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(booking)))
}

/// Lists durable bookings for a movie.
#[axum::debug_handler]
pub async fn list_by_movie(
    State(app_state): State<SharedAppState>,
    Path(movie_uuid): Path<String>,
) -> Result<Json<Vec<Booking>>, Error> {
    let rows = bookings::list_by_movie_uuid(&movie_uuid, &app_state.db_pool).await?;
    Ok(Json(rows))
}
