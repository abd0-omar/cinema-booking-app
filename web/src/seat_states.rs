//! Merge SQLite bookings and Redis holds into per-seat state for the public seat map.

use crate::error::Error;
use crate::seat_layout::{iter_layout_seats, LayoutSeat};
use cinema_booking_db::entities::bookings::{self, Booking};
use cinema_booking_db::entities::movies::Movie;
use cinema_booking_db::DbPool;
use cinema_booking_store_port::{SeatHoldStore, SeatReservationSession};
use serde::Serialize;
use std::collections::HashMap;

/// UI / JSON state for one seat on the map.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SeatStateKind {
    Available,
    YourHold,
    OtherHold,
    Confirmed,
}

#[derive(Debug, Serialize)]
pub struct SeatStateDto {
    pub seat_uuid: String,
    pub row: i64,
    pub col: i64,
    pub state: SeatStateKind,
}

/// `GET /movies/{slug}/seats` body: movie metadata plus flat seat list.
#[derive(Debug, Serialize)]
pub struct MovieSeatsResponse {
    pub id: i64,
    pub slug: String,
    pub title: String,
    #[serde(rename = "rows")]
    pub row_count: i64,
    #[serde(rename = "seats_per_rows")]
    pub seats_per_row: i64,
    pub seats: Vec<SeatStateDto>,
}

fn state_for_cell(
    cell: &LayoutSeat,
    booking_by_seat: &HashMap<String, &Booking>,
    session_by_seat: &HashMap<String, &SeatReservationSession>,
    viewer_user_id: &str,
) -> SeatStateKind {
    if booking_by_seat.contains_key(&cell.seat_uuid) {
        return SeatStateKind::Confirmed;
    }
    if let Some(session) = session_by_seat.get(&cell.seat_uuid) {
        if !viewer_user_id.is_empty() && session.user_uuid == viewer_user_id {
            return SeatStateKind::YourHold;
        }
        return SeatStateKind::OtherHold;
    }
    SeatStateKind::Available
}

/// Loads the movie, merges SQLite bookings and Redis holds, and returns one row per layout seat.
pub async fn seat_states_for_movie(
    slug: &str,
    viewer_user_id: &str,
    db: &DbPool,
    seat_hold_store: &dyn SeatHoldStore,
) -> Result<MovieSeatsResponse, Error> {
    let movie = cinema_booking_db::entities::movies::load(slug, db).await?;
    merge_movie_seats(&movie, viewer_user_id, db, seat_hold_store).await
}

/// Same as [`seat_states_for_movie`] but reuses an already-loaded [`Movie`].
pub async fn merge_movie_seats(
    movie: &Movie,
    viewer_user_id: &str,
    db: &DbPool,
    seat_hold_store: &dyn SeatHoldStore,
) -> Result<MovieSeatsResponse, Error> {
    let bookings = bookings::list_by_movie_slug(&movie.slug, db).await?;
    let sessions = seat_hold_store
        .list_held_sessions_for_movie(&movie.slug)
        .await?;

    let booking_by_seat: HashMap<String, &Booking> =
        bookings.iter().map(|b| (b.seat_uuid.clone(), b)).collect();

    let session_by_seat: HashMap<String, &SeatReservationSession> =
        sessions.iter().map(|s| (s.seat_uuid.clone(), s)).collect();

    let seats: Vec<SeatStateDto> = iter_layout_seats(movie.row_count, movie.seats_per_row)
        .map(|cell| SeatStateDto {
            seat_uuid: cell.seat_uuid.clone(),
            row: cell.row,
            col: cell.col,
            state: state_for_cell(&cell, &booking_by_seat, &session_by_seat, viewer_user_id),
        })
        .collect();

    Ok(MovieSeatsResponse {
        id: movie.id,
        slug: movie.slug.clone(),
        title: movie.title.clone(),
        row_count: movie.row_count,
        seats_per_row: movie.seats_per_row,
        seats,
    })
}

fn escape_html_attr(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '&' => "&amp;".to_string(),
            '"' => "&quot;".to_string(),
            '<' => "&lt;".to_string(),
            _ => c.to_string(),
        })
        .collect()
}

/// Builds the HTML fragment Datastar patches into `#seatGrid` (include the element with that id).
pub fn seat_grid_element_html(movie: &Movie, seats: &[SeatStateDto]) -> String {
    let m = movie.seats_per_row.max(0) as usize;
    let grid_style = format!("grid-template-columns: repeat({m}, minmax(0, 1fr));");

    let buttons: String = seats
        .iter()
        .map(|s| {
            let bg_class = match s.state {
                SeatStateKind::Available => "bg-base-300",
                SeatStateKind::YourHold => "bg-warning",
                SeatStateKind::OtherHold => "bg-secondary",
                SeatStateKind::Confirmed => "bg-error",
            };
            let label = escape_html_attr(&s.seat_uuid);
            let aria = escape_html_attr(&format!("Seat row {} column {}", s.row, s.col));
            format!(
                r#"<button type="button" class="btn btn-sm min-h-8 w-full px-0 {bg_class} border-0" data-seat="{label}" aria-label="{aria}">{label}</button>"#
            )
        })
        .collect();

    format!(
        r#"<div id="seatGrid" class="grid w-full max-w-xl gap-1" style="{grid_style}" role="group" aria-label="Seat map">{buttons}</div>"#
    )
}

/// Error placeholder when the movie is missing or the stream should show feedback.
pub fn seat_grid_error_html(message: &str) -> String {
    let msg = escape_html_attr(message);
    format!(
        r#"<div id="seatGrid" role="alert" class="alert alert-error w-full max-w-xl">{msg}</div>"#
    )
}
