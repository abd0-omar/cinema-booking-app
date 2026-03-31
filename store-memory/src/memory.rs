use async_trait::async_trait;
use cinema_booking_store::{
    Booking, BookingChangeset, BookingStatus, BookingStore, BookingStoreError,
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
}

impl InMemoryBookingStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(Inner {
                next_id: 1,
                bookings: Vec::new(),
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
}
