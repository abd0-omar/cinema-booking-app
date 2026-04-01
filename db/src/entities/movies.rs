#[cfg(feature = "test-helpers")]
use fake::{faker::lorem::en::*, Dummy};
use serde::Deserialize;
use serde::Serialize;
use sqlx::Sqlite;
use validator::Validate;

/// A movie with auditorium dimensions (rows × seats per row).
#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct Movie {
    /// The internal id of the record (aliases rowid for speed).
    pub id: i64,
    /// The external UUID for URLs/APIs.
    pub uuid: String,
    pub title: String,
    /// Number of rows in the auditorium (JSON key `rows`).
    #[serde(rename = "rows")]
    pub row_count: i64,
    /// Seats in each row (JSON key `seats_per_rows`).
    #[serde(rename = "seats_per_rows")]
    pub seats_per_row: i64,
}

/// Payload for creating or updating a [`Movie`].
#[derive(Deserialize, Validate, Clone)]
#[cfg_attr(feature = "test-helpers", derive(Serialize, Dummy))]
pub struct MovieChangeset {
    #[cfg_attr(feature = "test-helpers", dummy(faker = "Sentence(3..8)"))]
    #[validate(length(min = 1))]
    pub title: String,
    #[serde(rename = "rows")]
    #[validate(range(min = 1))]
    pub row_count: i64,
    #[serde(rename = "seats_per_rows")]
    #[validate(range(min = 1))]
    pub seats_per_row: i64,
}

/// Load all [`Movie`]s from the database.
pub async fn load_all(
    executor: impl sqlx::Executor<'_, Database = Sqlite>,
) -> Result<Vec<Movie>, crate::Error> {
    let rows = sqlx::query_as!(
        Movie,
        r#"SELECT id, uuid, title, row_count, seats_per_row FROM movies"#
    )
    .fetch_all(executor)
    .await?;
    Ok(rows)
}

/// Load one [`Movie`] by UUID.
pub async fn load(
    uuid: &str,
    executor: impl sqlx::Executor<'_, Database = Sqlite>,
) -> Result<Movie, crate::Error> {
    sqlx::query_as!(
        Movie,
        r#"SELECT id as "id!", uuid, title, row_count, seats_per_row FROM movies WHERE uuid = ?1"#,
        uuid
    )
    .fetch_optional(executor)
    .await
    .map_err(crate::Error::DbError)?
    .ok_or(crate::Error::NoRecordFound)
}

/// Load one [`Movie`] by internal id.
pub async fn load_by_id(
    id: i64,
    executor: impl sqlx::Executor<'_, Database = Sqlite>,
) -> Result<Movie, crate::Error> {
    sqlx::query_as!(
        Movie,
        r#"SELECT id, uuid, title, row_count, seats_per_row FROM movies WHERE id = ?1"#,
        id
    )
    .fetch_optional(executor)
    .await
    .map_err(crate::Error::DbError)?
    .ok_or(crate::Error::NoRecordFound)
}

/// Delete a [`Movie`] by UUID.
pub async fn delete(
    uuid: &str,
    executor: impl sqlx::Executor<'_, Database = Sqlite>,
) -> Result<(), crate::Error> {
    let result = sqlx::query!("DELETE FROM movies WHERE uuid = ?1", uuid)
        .execute(executor)
        .await
        .map_err(crate::Error::DbError)?;

    if result.rows_affected() == 0 {
        return Err(crate::Error::NoRecordFound);
    }

    Ok(())
}

/// Create a [`Movie`] from a changeset (UUID generated here).
pub async fn create(
    movie: MovieChangeset,
    executor: impl sqlx::Executor<'_, Database = Sqlite>,
) -> Result<Movie, crate::Error> {
    movie.validate()?;

    let uuid = uuid::Uuid::new_v4().to_string();

    let result = sqlx::query!(
        "INSERT INTO movies (uuid, title, row_count, seats_per_row) VALUES (?1, ?2, ?3, ?4)",
        uuid,
        movie.title,
        movie.row_count,
        movie.seats_per_row,
    )
    .execute(executor)
    .await
    .map_err(crate::Error::DbError)?;

    Ok(Movie {
        id: result.last_insert_rowid(),
        uuid,
        title: movie.title,
        row_count: movie.row_count,
        seats_per_row: movie.seats_per_row,
    })
}

/// Update a [`Movie`] by UUID.
pub async fn update(
    uuid: &str,
    movie: MovieChangeset,
    db_pool: &crate::DbPool,
) -> Result<Movie, crate::Error> {
    movie.validate()?;

    let result = sqlx::query!(
        "UPDATE movies SET title = ?1, row_count = ?2, seats_per_row = ?3 WHERE uuid = ?4",
        movie.title,
        movie.row_count,
        movie.seats_per_row,
        uuid
    )
    .execute(db_pool)
    .await
    .map_err(crate::Error::DbError)?;

    if result.rows_affected() == 0 {
        return Err(crate::Error::NoRecordFound);
    }

    sqlx::query_as!(
        Movie,
        r#"SELECT id as "id!", uuid, title, row_count, seats_per_row FROM movies WHERE uuid = ?1"#,
        uuid
    )
    .fetch_one(db_pool)
    .await
    .map_err(crate::Error::DbError)
}
