#[cfg(feature = "test-helpers")]
use fake::{faker::lorem::en::*, Dummy};
use serde::Deserialize;
use serde::Serialize;
use sqlx::Sqlite;
use validator::Validate;

/// Lowercase slug base from a title: non-alphanumeric runs become single hyphens; empty → `"movie"`.
pub(crate) fn slugify_title(title: &str) -> String {
    let mut out = String::new();
    let mut prev_hyphen = false;
    for ch in title.to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            prev_hyphen = false;
        } else if !prev_hyphen && !out.is_empty() {
            out.push('-');
            prev_hyphen = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "movie".to_string()
    } else {
        out
    }
}

/// Public movie identifier: `{slugify(title)}-{id}`.
pub(crate) fn movie_slug(title: &str, id: i64) -> String {
    format!("{}-{}", slugify_title(title), id)
}

/// A movie with auditorium dimensions (rows × seats per row).
#[derive(Serialize, Debug, Deserialize, Clone)]
pub struct Movie {
    /// The internal id of the record (aliases rowid for speed).
    pub id: i64,
    /// URL-safe slug: slugified title plus numeric id (unique).
    pub slug: String,
    pub title: String,
    /// Short display time label shown in the program list (for example `"in 5 min"`).
    pub movie_time: String,
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
    #[cfg_attr(feature = "test-helpers", dummy(faker = "Word()"))]
    #[validate(length(min = 1, max = 32))]
    pub movie_time: String,
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
        r#"SELECT id, slug, title, movie_time, row_count, seats_per_row FROM movies"#
    )
    .fetch_all(executor)
    .await?;
    Ok(rows)
}

/// Load one [`Movie`] by slug.
pub async fn load(
    slug: &str,
    executor: impl sqlx::Executor<'_, Database = Sqlite>,
) -> Result<Movie, crate::Error> {
    sqlx::query_as!(
        Movie,
        r#"SELECT id as "id!", slug, title, movie_time, row_count, seats_per_row FROM movies WHERE slug = ?1"#,
        slug
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
        r#"SELECT id, slug, title, movie_time, row_count, seats_per_row FROM movies WHERE id = ?1"#,
        id
    )
    .fetch_optional(executor)
    .await
    .map_err(crate::Error::DbError)?
    .ok_or(crate::Error::NoRecordFound)
}

/// Delete a [`Movie`] by slug.
pub async fn delete(
    slug: &str,
    executor: impl sqlx::Executor<'_, Database = Sqlite>,
) -> Result<(), crate::Error> {
    let result = sqlx::query!("DELETE FROM movies WHERE slug = ?1", slug)
        .execute(executor)
        .await
        .map_err(crate::Error::DbError)?;

    if result.rows_affected() == 0 {
        return Err(crate::Error::NoRecordFound);
    }

    Ok(())
}

/// Create a [`Movie`] from a changeset (slug assigned after insert: `{slugify(title)}-{id}`).
pub async fn create(movie: MovieChangeset, db_pool: &crate::DbPool) -> Result<Movie, crate::Error> {
    movie.validate()?;
    let MovieChangeset {
        title,
        movie_time,
        row_count,
        seats_per_row,
    } = movie;

    let mut tx = db_pool.begin().await.map_err(crate::Error::DbError)?;
    let temp_slug = format!("__tmp_{}", uuid::Uuid::new_v4());
    let insert_title = title.clone();
    let insert_movie_time = movie_time.clone();

    let result = sqlx::query!(
        "INSERT INTO movies (slug, title, movie_time, row_count, seats_per_row) VALUES (?1, ?2, ?3, ?4, ?5)",
        temp_slug,
        insert_title,
        insert_movie_time,
        row_count,
        seats_per_row,
    )
    .execute(&mut *tx)
    .await
    .map_err(crate::Error::DbError)?;

    let id = result.last_insert_rowid();
    let slug = movie_slug(&title, id);
    sqlx::query!("UPDATE movies SET slug = ?1 WHERE id = ?2", slug, id)
        .execute(&mut *tx)
        .await
        .map_err(crate::Error::DbError)?;

    tx.commit().await.map_err(crate::Error::DbError)?;

    Ok(Movie {
        id,
        slug,
        title,
        movie_time,
        row_count,
        seats_per_row,
    })
}

/// Update a [`Movie`] by slug (slug recomputed if title changes).
pub async fn update(
    slug: &str,
    movie: MovieChangeset,
    db_pool: &crate::DbPool,
) -> Result<Movie, crate::Error> {
    movie.validate()?;
    let MovieChangeset {
        title,
        movie_time,
        row_count,
        seats_per_row,
    } = movie;

    let current = load(slug, db_pool).await?;
    let new_slug = movie_slug(&title, current.id);
    let update_slug = new_slug.clone();

    let result = sqlx::query!(
        "UPDATE movies SET slug = ?1, title = ?2, movie_time = ?3, row_count = ?4, seats_per_row = ?5 WHERE slug = ?6",
        update_slug,
        title,
        movie_time,
        row_count,
        seats_per_row,
        slug
    )
    .execute(db_pool)
    .await
    .map_err(crate::Error::DbError)?;

    if result.rows_affected() == 0 {
        return Err(crate::Error::NoRecordFound);
    }

    sqlx::query_as!(
        Movie,
        r#"SELECT id as "id!", slug, title, movie_time, row_count, seats_per_row FROM movies WHERE slug = ?1"#,
        new_slug
    )
    .fetch_one(db_pool)
    .await
    .map_err(crate::Error::DbError)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_title_basic() {
        assert_eq!(
            slugify_title("Demo: Midnight Express"),
            "demo-midnight-express"
        );
        assert_eq!(
            slugify_title("Hurl integration movie"),
            "hurl-integration-movie"
        );
    }

    #[test]
    fn movie_slug_format() {
        assert_eq!(
            movie_slug("Demo: Midnight Express", 1),
            "demo-midnight-express-1"
        );
    }
}
