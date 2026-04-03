//! JSON API for seat holds (Redis) and durable bookings (SQLite via [`cinema_booking_db`]).
use crate::{error::Error, state::SharedAppState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Extension, Json,
};
use cinema_booking_auth::Principal;
use cinema_booking_db::entities::{
    bookings::{self, Booking, BookingChangeset},
    users,
};
use cinema_booking_store_port::{SeatHoldChangeset, SeatReservationSession};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SeatActionPayload {
    pub movie_slug: String,
    pub seat_uuid: String,
}

/// Starts a TTL-backed seat hold in Redis.
#[axum::debug_handler]
pub async fn hold(
    State(app_state): State<SharedAppState>,
    Extension(principal): Extension<Principal>,
    Json(payload): Json<SeatActionPayload>,
) -> Result<(StatusCode, Json<SeatReservationSession>), Error> {
    let changeset = SeatHoldChangeset {
        movie_slug: payload.movie_slug,
        seat_uuid: payload.seat_uuid,
        user_uuid: principal.sub,
    };
    let session = app_state.seat_hold_store.hold(changeset).await?;
    Ok((StatusCode::CREATED, Json(session)))
}

/// Confirms the Redis hold and persists a booking in SQLite.
#[axum::debug_handler]
pub async fn checkout(
    State(app_state): State<SharedAppState>,
    Extension(principal): Extension<Principal>,
    Json(payload): Json<SeatActionPayload>,
) -> Result<(StatusCode, Json<Booking>), Error> {
    let user_uuid = principal.sub;
    users::upsert_for_auth_subject(&user_uuid, &principal.email, "", &app_state.db_pool).await?;
    let session = app_state
        .seat_hold_store
        .confirm(&payload.movie_slug, &payload.seat_uuid, &user_uuid)
        .await?;
    let booking = bookings::create(
        BookingChangeset {
            movie_slug: session.movie_slug,
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
    Path(movie_slug): Path<String>,
) -> Result<Json<Vec<Booking>>, Error> {
    let rows = bookings::list_by_movie_slug(&movie_slug, &app_state.db_pool).await?;
    Ok(Json(rows))
}
