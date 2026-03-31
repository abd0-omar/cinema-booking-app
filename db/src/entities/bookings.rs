#[cfg(feature = "test-helpers")]
use fake::{faker::lorem::en::*, Dummy};
use serde::Deserialize;
use serde::Serialize;
use sqlx::Sqlite;
use validator::Validate;

/// Lifecycle state of a booking.
#[derive(
    Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, sqlx::Type,
)]
#[cfg_attr(feature = "test-helpers", derive(Dummy))]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum BookingStatus {
    Pending,
    Confirmed,
    Cancelled,
}

/// A seat booking for a movie, owned by a user.
#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct Booking {
    /// The internal id of the record (aliases rowid for speed).
    pub id: i64,
    /// The external UUID for URLs/APIs.
    pub uuid: String,
    pub movie_uuid: String,
    pub seat_uuid: String,
    pub user_uuid: String,
    pub status: BookingStatus,
}

/// Data for creating or updating a [`Booking`].
#[derive(Deserialize, Validate, Clone)]
#[cfg_attr(feature = "test-helpers", derive(Serialize, Dummy))]
pub struct BookingChangeset {
    #[cfg_attr(feature = "test-helpers", dummy(faker = "Word()"))]
    #[validate(length(min = 1))]
    pub movie_uuid: String,
    #[cfg_attr(feature = "test-helpers", dummy(faker = "Word()"))]
    #[validate(length(min = 1))]
    pub seat_uuid: String,
    #[cfg_attr(feature = "test-helpers", dummy(faker = "Word()"))]
    #[validate(length(min = 1))]
    pub user_uuid: String,
    pub status: BookingStatus,
}

/// Load all [`Booking`]s from the database.
pub async fn load_all(
    executor: impl sqlx::Executor<'_, Database = Sqlite>,
) -> Result<Vec<Booking>, crate::Error> {
    let rows = sqlx::query_as!(
        Booking,
        r#"SELECT id, uuid, movie_uuid, seat_uuid, user_uuid, status as "status: BookingStatus" FROM bookings"#
    )
    .fetch_all(executor)
    .await?;
    Ok(rows)
}

/// Load one [`Booking`] from the database identified by its UUID.
///
/// If no record can be found for the UUID, a [`crate::Error::NoRecordFound`] will be returned.
pub async fn load(
    uuid: &str,
    executor: impl sqlx::Executor<'_, Database = Sqlite>,
) -> Result<Booking, crate::Error> {
    sqlx::query_as!(
        Booking,
        r#"SELECT id as "id!", uuid, movie_uuid, seat_uuid, user_uuid, status as "status: BookingStatus" FROM bookings WHERE uuid = ?1"#,
        uuid
    )
    .fetch_optional(executor)
    .await
    .map_err(crate::Error::DbError)?
    .ok_or(crate::Error::NoRecordFound)
}

/// Load one [`Booking`] from the database identified by its internal ID.
///
/// If no record can be found for the ID, a [`crate::Error::NoRecordFound`] will be returned.
pub async fn load_by_id(
    id: i64,
    executor: impl sqlx::Executor<'_, Database = Sqlite>,
) -> Result<Booking, crate::Error> {
    sqlx::query_as!(
        Booking,
        r#"SELECT id, uuid, movie_uuid, seat_uuid, user_uuid, status as "status: BookingStatus" FROM bookings WHERE id = ?1"#,
        id
    )
    .fetch_optional(executor)
    .await
    .map_err(crate::Error::DbError)?
    .ok_or(crate::Error::NoRecordFound)
}

/// Delete a [`Booking`] from the database identified by its UUID.
///
/// If no record can be found for the UUID, a [`crate::Error::NoRecordFound`] will be returned.
pub async fn delete(
    uuid: &str,
    executor: impl sqlx::Executor<'_, Database = Sqlite>,
) -> Result<(), crate::Error> {
    let result = sqlx::query!("DELETE FROM bookings WHERE uuid = ?1", uuid)
        .execute(executor)
        .await
        .map_err(crate::Error::DbError)?;

    if result.rows_affected() == 0 {
        return Err(crate::Error::NoRecordFound);
    }

    Ok(())
}

/// Create a booking in the database with the data in the passed [`BookingChangeset`].
///
/// If the data in the changeset isn't valid, a [`crate::Error::ValidationError`] will be returned, otherwise the created booking is returned.
pub async fn create(
    booking: BookingChangeset,
    executor: impl sqlx::Executor<'_, Database = Sqlite>,
) -> Result<Booking, crate::Error> {
    booking.validate()?;

    let uuid = uuid::Uuid::new_v4().to_string();

    let result = sqlx::query!(
        "INSERT INTO bookings (uuid, movie_uuid, seat_uuid, user_uuid, status) VALUES (?1, ?2, ?3, ?4, ?5)",
        uuid,
        booking.movie_uuid,
        booking.seat_uuid,
        booking.user_uuid,
        booking.status,
    )
    .execute(executor)
    .await
    .map_err(crate::Error::DbError)?;

    Ok(Booking {
        id: result.last_insert_rowid(),
        uuid,
        movie_uuid: booking.movie_uuid,
        seat_uuid: booking.seat_uuid,
        user_uuid: booking.user_uuid,
        status: booking.status,
    })
}

/// Updates a booking in the database with the data in the passed [`BookingChangeset`].
///
/// If the data in the changeset isn't valid, a [`crate::Error::ValidationError`] will be returned, otherwise the updated [`Booking`] is returned. If no record can be found for the UUID, a [`crate::Error::NoRecordFound`] will be returned.
pub async fn update(
    uuid: &str,
    booking: BookingChangeset,
    db_pool: &crate::DbPool,
) -> Result<Booking, crate::Error> {
    booking.validate()?;

    let result = sqlx::query!(
        "UPDATE bookings SET movie_uuid = ?1, seat_uuid = ?2, user_uuid = ?3, status = ?4 WHERE uuid = ?5",
        booking.movie_uuid,
        booking.seat_uuid,
        booking.user_uuid,
        booking.status,
        uuid
    )
    .execute(db_pool)
    .await
    .map_err(crate::Error::DbError)?;

    if result.rows_affected() == 0 {
        return Err(crate::Error::NoRecordFound);
    }

    sqlx::query_as!(
        Booking,
        r#"SELECT id as "id!", uuid, movie_uuid, seat_uuid, user_uuid, status as "status: BookingStatus" FROM bookings WHERE uuid = ?1"#,
        uuid
    )
    .fetch_one(db_pool)
    .await
    .map_err(crate::Error::DbError)
}
