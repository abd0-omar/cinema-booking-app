//! Seat grid indexing for auditorium layouts.
//!
//! # Model
//!
//! - `m` = **seats per row** (columns per row), same as `Movie.seats_per_row`.
//! - `row_count` = number of rows (`Movie.row_count`).
//! - **0-based** linear index `idx` runs from `0` to `N - 1` where `N = row_count * m`.
//! - **Flatten:** `idx = i * m + j` with row index `i` and column index `j` (`0 <= i < row_count`, `0 <= j < m`).
//! - **Unflatten:** `i = idx / m`, `j = idx % m` (using integer division / remainder).
//! - Public **seat ids** are 1-based labels `s1` … `sN` where `s{idx+1}` maps to `(i, j) = (idx / m, idx % m)`.
//! - JSON / API **row** and **col** are **1-based:** `row = i + 1`, `col = j + 1`.

/// 0-based grid cell `(row_index, col_index)` from linear index and seats-per-row `m`.
#[must_use]
pub fn from_flat_idx(idx: usize, seats_per_row: usize) -> (usize, usize) {
    if seats_per_row == 0 {
        return (0, 0);
    }
    (idx / seats_per_row, idx % seats_per_row)
}

/// Linear index from 0-based row/column indices and `m` seats per row.
#[must_use]
pub fn flat_idx(row_index: usize, col_index: usize, seats_per_row: usize) -> usize {
    row_index * seats_per_row + col_index
}

/// One seat cell in row-major order for a rectangular auditorium.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutSeat {
    /// 0-based index in `[0, row_count * seats_per_row)`.
    pub flat_idx: usize,
    /// Public API id: `s1` … `sN`.
    pub seat_uuid: String,
    /// 1-based row (auditorium front to back).
    pub row: i64,
    /// 1-based column within the row.
    pub col: i64,
}

/// Iterates every seat in layout order (row-major).
///
/// Empty iterator if `row_count` or `seats_per_row` is non-positive.
pub fn iter_layout_seats(row_count: i64, seats_per_row: i64) -> impl Iterator<Item = LayoutSeat> {
    let rows = row_count.max(0) as usize;
    let m = seats_per_row.max(0) as usize;
    let total = rows.saturating_mul(m);
    (0..total).map(move |idx| {
        let (i, j) = from_flat_idx(idx, m);
        LayoutSeat {
            flat_idx: idx,
            seat_uuid: format!("s{}", idx + 1),
            row: (i + 1) as i64,
            col: (j + 1) as i64,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn s1_is_top_left_1_based_row_col() {
        let first = iter_layout_seats(3, 5).next().unwrap();
        assert_eq!(first.seat_uuid, "s1");
        assert_eq!(first.row, 1);
        assert_eq!(first.col, 1);
    }

    #[test]
    fn flat_and_unflatten_roundtrip() {
        let m = 4_usize;
        assert_eq!(flat_idx(0, 0, m), 0);
        assert_eq!(from_flat_idx(0, m), (0, 0));
        assert_eq!(flat_idx(1, 2, m), 6);
        assert_eq!(from_flat_idx(6, m), (1, 2));
    }

    #[test]
    fn total_seats_matches_dimensions() {
        assert_eq!(iter_layout_seats(2, 3).count(), 6);
    }
}
