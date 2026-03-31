use async_trait::async_trait;
use cinema_booking_store_port::{
    Booking, BookingChangeset, BookingStatus, BookingStore, BookingStoreError, SeatHoldChangeset,
    SeatReservationSession, SeatReservationStatus,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use validator::Validate;

/// Thread-safe in-memory booking storage for tests and local tooling.
pub struct InMemoryBookingStore {
    inner: Arc<RwLock<Inner>>,
}

struct Inner {
    next_id: i64,
    bookings: Vec<Booking>,
    reservations: Vec<SeatReservationSession>,
}

impl InMemoryBookingStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(Inner {
                next_id: 1,
                bookings: Vec::new(),
                reservations: Vec::new(),
            })),
        }
    }

    fn active_seat_taken(bookings: &[Booking], movie_uuid: &str, seat_uuid: &str) -> bool {
        bookings.iter().any(|b| {
            b.movie_uuid == movie_uuid
                && b.seat_uuid == seat_uuid
                && b.status != BookingStatus::Cancelled
        })
    }

    fn active_reservation_taken(
        reservations: &[SeatReservationSession],
        movie_uuid: &str,
        seat_uuid: &str,
    ) -> bool {
        reservations.iter().any(|r| {
            r.movie_uuid == movie_uuid
                && r.seat_uuid == seat_uuid
                && matches!(
                    r.status,
                    SeatReservationStatus::Held | SeatReservationStatus::Confirmed
                )
        })
    }
}

impl Default for InMemoryBookingStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BookingStore for InMemoryBookingStore {
    async fn book(&self, changeset: BookingChangeset) -> Result<Booking, BookingStoreError> {
        changeset
            .validate()
            .map_err(|e| BookingStoreError::Validation(e.to_string()))?;

        let movie_uuid = changeset.movie_uuid.clone();
        let seat_uuid = changeset.seat_uuid.clone();

        let mut guard = self.inner.write().await;

        if Self::active_seat_taken(&guard.bookings, &movie_uuid, &seat_uuid) {
            return Err(BookingStoreError::Conflict(
                "seat already booked for this movie".into(),
            ));
        }

        let id = guard.next_id;
        guard.next_id += 1;
        let uuid = Uuid::new_v4().to_string();

        let booking = Booking {
            id,
            uuid: uuid.clone(),
            movie_uuid: changeset.movie_uuid,
            seat_uuid: changeset.seat_uuid,
            user_uuid: changeset.user_uuid,
            status: changeset.status,
        };
        guard.bookings.push(booking.clone());
        Ok(booking)
    }

    async fn list_bookings_by_movie(
        &self,
        movie_uuid: &str,
    ) -> Result<Vec<Booking>, BookingStoreError> {
        let guard = self.inner.read().await;
        Ok(guard
            .bookings
            .iter()
            .filter(|b| b.movie_uuid == movie_uuid)
            .cloned()
            .collect())
    }

    async fn hold(
        &self,
        changeset: SeatHoldChangeset,
    ) -> Result<SeatReservationSession, BookingStoreError> {
        changeset.validate()?;
        let mut guard = self.inner.write().await;
        if Self::active_reservation_taken(
            &guard.reservations,
            &changeset.movie_uuid,
            &changeset.seat_uuid,
        ) {
            return Err(BookingStoreError::SeatUnavailable);
        }

        let session = SeatReservationSession {
            session_uuid: Uuid::new_v4().to_string(),
            movie_uuid: changeset.movie_uuid,
            seat_uuid: changeset.seat_uuid,
            user_uuid: changeset.user_uuid,
            status: SeatReservationStatus::Held,
        };
        guard.reservations.push(session.clone());
        Ok(session)
    }

    async fn confirm(
        &self,
        movie_uuid: &str,
        seat_uuid: &str,
        user_uuid: &str,
    ) -> Result<SeatReservationSession, BookingStoreError> {
        let mut guard = self.inner.write().await;
        let Some(session) = guard
            .reservations
            .iter_mut()
            .find(|s| s.movie_uuid == movie_uuid && s.seat_uuid == seat_uuid)
        else {
            return Err(BookingStoreError::SessionNotFound);
        };

        if session.user_uuid != user_uuid {
            return Err(BookingStoreError::SessionOwnershipMismatch);
        }

        session.status = SeatReservationStatus::Confirmed;
        Ok(session.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_changeset(movie: &str, seat: &str, user: &str) -> BookingChangeset {
        BookingChangeset {
            movie_uuid: movie.into(),
            seat_uuid: seat.into(),
            user_uuid: user.into(),
            status: BookingStatus::Pending,
        }
    }

    #[tokio::test]
    async fn book_list_and_conflict() {
        let store = InMemoryBookingStore::new();
        let cs = sample_changeset("m1", "s1", "u1");

        let created = store.book(cs.clone()).await.unwrap();
        assert_eq!(created.movie_uuid, "m1");

        let listed = store.list_bookings_by_movie("m1").await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].uuid, created.uuid);

        let err = store.book(cs).await.unwrap_err();
        assert!(matches!(err, BookingStoreError::Conflict(_)));
    }

    #[tokio::test]
    async fn hold_confirm_and_conflict() {
        let store = InMemoryBookingStore::new();
        let hold = SeatHoldChangeset {
            movie_uuid: "m1".into(),
            seat_uuid: "s1".into(),
            user_uuid: "u1".into(),
        };

        let created = store.hold(hold.clone()).await.unwrap();
        assert_eq!(created.status, SeatReservationStatus::Held);

        let err = store.hold(hold).await.unwrap_err();
        assert!(matches!(err, BookingStoreError::SeatUnavailable));

        let confirmed = store.confirm("m1", "s1", "u1").await.unwrap();
        assert_eq!(confirmed.status, SeatReservationStatus::Confirmed);
    }
}
