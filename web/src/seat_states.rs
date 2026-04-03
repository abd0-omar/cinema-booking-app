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

fn row_label(mut row: i64) -> String {
    if row <= 0 {
        return "?".to_string();
    }
    let mut out = String::new();
    while row > 0 {
        let rem = ((row - 1) % 26) as u8;
        out.insert(0, (b'A' + rem) as char);
        row = (row - 1) / 26;
    }
    out
}

/// Builds the HTML fragment Datastar patches into `#seatGrid` (include the element with that id).
pub fn seat_grid_element_html(movie: &Movie, seats: &[SeatStateDto]) -> String {
    let rows = movie.row_count.max(0);
    let cols = movie.seats_per_row.max(0);
    let aisle_after = if cols >= 8 { cols / 2 } else { 0 };

    let mut by_row: HashMap<i64, Vec<&SeatStateDto>> = HashMap::new();
    for seat in seats {
        by_row.entry(seat.row).or_default().push(seat);
    }

    let mut rows_html = String::new();
    for row in 1..=rows {
        let row_name = row_label(row);
        let row_name_html = escape_html_attr(&row_name);
        let mut seat_cells = by_row.remove(&row).unwrap_or_default();
        seat_cells.sort_by_key(|s| s.col);

        let mut seats_html = String::new();
        for seat in seat_cells {
            let (state_class, state_label, inner_class, state_key) = match seat.state {
                SeatStateKind::Available => (
                    "border-base-300/90 bg-base-300/80 shadow-inner shadow-base-content/5 hover:-translate-y-px hover:border-primary/50 hover:bg-base-300 hover:shadow-md",
                    "available",
                    "bg-base-content/15",
                    "available",
                ),
                SeatStateKind::YourHold => (
                    "border-warning/80 bg-warning/85 shadow-inner shadow-warning/20 hover:-translate-y-px hover:border-warning hover:bg-warning hover:shadow-md",
                    "your hold",
                    "bg-warning-content/25",
                    "your_hold",
                ),
                SeatStateKind::OtherHold => (
                    "border-secondary/80 bg-secondary/85 shadow-inner shadow-secondary/20 hover:-translate-y-px hover:border-secondary hover:bg-secondary hover:shadow-md",
                    "held by another viewer",
                    "bg-secondary-content/25",
                    "other_hold",
                ),
                SeatStateKind::Confirmed => (
                    "border-error/80 bg-error/90 shadow-inner shadow-error/25 opacity-95 hover:-translate-y-px hover:border-error hover:bg-error",
                    "booked",
                    "bg-error-content/30",
                    "confirmed",
                ),
            };

            let seat_uuid = escape_html_attr(&seat.seat_uuid);
            let aria = escape_html_attr(&format!(
                "Row {}, Seat {}, {}",
                row_name, seat.col, state_label
            ));
            seats_html.push_str(&format!(
                r#"<button type="button" class="flex h-10 w-10 shrink-0 items-center justify-center rounded-t-md rounded-b-sm border transition-all duration-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/70 focus-visible:ring-offset-2 focus-visible:ring-offset-base-100 {state_class}" data-seat="{seat_uuid}" data-state="{state_key}" aria-label="{aria}"><span class="h-2 w-3 rounded-sm {inner_class} opacity-90" aria-hidden="true"></span></button>"#
            ));

            if aisle_after > 0 && seat.col == aisle_after {
                seats_html.push_str(
                    r#"<span class="mx-1 hidden h-10 w-4 rounded bg-base-300/35 sm:inline-block" aria-hidden="true"></span>"#,
                );
            }
        }

        rows_html.push_str(&format!(
            r#"<div class="flex w-max max-w-full min-w-0 items-stretch gap-2 rounded-xl border border-base-300/50 bg-base-200/40 px-2 py-2 sm:gap-3 sm:px-3"><div class="flex w-9 shrink-0 items-center justify-center self-stretch rounded-lg border border-base-300/60 bg-base-100/90 font-display text-sm font-bold tabular-nums text-base-content/80 shadow-sm" aria-hidden="true">{row_name_html}</div><div class="flex flex-nowrap items-center justify-start gap-1.5 sm:gap-2">{seats_html}</div></div>"#
        ));
    }

    let map_label = escape_html_attr(&format!(
        "{} seat map with {} rows and {} seats per row",
        movie.title, rows, cols
    ));

    format!(
        r#"<div id="seatGrid" class="w-max max-w-full" role="group" aria-label="{map_label}"><div class="w-full rounded-box border border-base-300/60 bg-base-100/70 px-2 py-3 sm:px-4"><div class="flex w-full flex-col items-stretch gap-2 sm:gap-2.5">{rows_html}</div></div></div>"#
    )
}

/// Error placeholder when the movie is missing or the stream should show feedback.
pub fn seat_grid_error_html(message: &str) -> String {
    let msg = escape_html_attr(message);
    format!(
        r#"<div id="seatGrid" role="alert" class="alert alert-error w-full max-w-xl">{msg}</div>"#
    )
}
