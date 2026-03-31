#[cfg(feature = "test-helpers")]
use fake::{faker::lorem::en::*, Dummy};
use serde::Deserialize;
use serde::Serialize;
use sqlx::Sqlite;
use validator::Validate;

/// A task, i.e. TODO item.
#[derive(Serialize, Debug, Deserialize)]
pub struct Task {
    /// The internal id of the record (aliases rowid for speed).
    pub id: i64,
    /// The external UUID for URLs/APIs.
    pub uuid: String,
    /// The description, i.e. what to do.
    pub description: String,
}

/// A changeset representing the data that is intended to be used to either create a new task or update an existing task.
///
/// Changesets are validatated in the [`create`] and [`update`] functions which return an [Result::Err] if validation fails.
///
/// Changesets can also be used to generate fake data for tests when the `test-helpers` feature is enabled:
///
/// ```
/// let task_changeset: TaskChangeset = Faker.fake();
/// ```
#[derive(Deserialize, Validate, Clone)]
#[cfg_attr(feature = "test-helpers", derive(Serialize, Dummy))]
pub struct TaskChangeset {
    /// The description must be at least 1 character long.
    #[cfg_attr(feature = "test-helpers", dummy(faker = "Sentence(3..8)"))]
    #[validate(length(min = 1))]
    pub description: String,
}

/// Load all [`Task`]s from the database.
pub async fn load_all(
    executor: impl sqlx::Executor<'_, Database = Sqlite>,
) -> Result<Vec<Task>, crate::Error> {
    let tasks = sqlx::query_as!(Task, r#"SELECT id, uuid, description FROM tasks"#)
        .fetch_all(executor)
        .await?;
    Ok(tasks)
}

/// Load one [`Task`] from the database identified by its UUID.
///
/// If no record can be found for the UUID, a [`crate::Error::NoRecordFound`] will be returned.
pub async fn load(
    uuid: &str,
    executor: impl sqlx::Executor<'_, Database = Sqlite>,
) -> Result<Task, crate::Error> {
    sqlx::query_as!(
        Task,
        r#"SELECT id as "id!", uuid, description FROM tasks WHERE uuid = ?1"#,
        uuid
    )
    .fetch_optional(executor)
    .await
    .map_err(crate::Error::DbError)?
    .ok_or(crate::Error::NoRecordFound)
}

/// Load one [`Task`] from the database identified by its internal ID.
///
/// If no record can be found for the ID, a [`crate::Error::NoRecordFound`] will be returned.
pub async fn load_by_id(
    id: i64,
    executor: impl sqlx::Executor<'_, Database = Sqlite>,
) -> Result<Task, crate::Error> {
    sqlx::query_as!(
        Task,
        r#"SELECT id, uuid, description FROM tasks WHERE id = ?1"#,
        id
    )
    .fetch_optional(executor)
    .await
    .map_err(crate::Error::DbError)?
    .ok_or(crate::Error::NoRecordFound)
}

/// Delete a [`Task`] from the database identified by its UUID.
///
/// If no record can be found for the UUID, a [`crate::Error::NoRecordFound`] will be returned.
pub async fn delete(
    uuid: &str,
    executor: impl sqlx::Executor<'_, Database = Sqlite>,
) -> Result<(), crate::Error> {
    let result = sqlx::query!("DELETE FROM tasks WHERE uuid = ?1", uuid)
        .execute(executor)
        .await
        .map_err(crate::Error::DbError)?;

    if result.rows_affected() == 0 {
        return Err(crate::Error::NoRecordFound);
    }

    Ok(())
}

/// Create a task in the database with the data in the passed [`TaskChangeset`].
///
/// If the data in the changeset isn't valid, a [`crate::Error::ValidationError`] will be returned, otherwise the created task is returned.
pub async fn create(
    task: TaskChangeset,
    executor: impl sqlx::Executor<'_, Database = Sqlite>,
) -> Result<Task, crate::Error> {
    task.validate()?;

    // Generate UUID in application code
    let uuid = uuid::Uuid::new_v4().to_string();

    let result = sqlx::query!(
        "INSERT INTO tasks (uuid, description) VALUES (?1, ?2)",
        uuid,
        task.description
    )
    .execute(executor)
    .await
    .map_err(crate::Error::DbError)?;

    Ok(Task {
        id: result.last_insert_rowid(),
        uuid,
        description: task.description,
    })
}

/// Updates a task in the database with the data in the passed [`TaskChangeset`].
///
/// If the data in the changeset isn't valid, a [`crate::Error::ValidationError`] will be returned, otherwise the updated [`Task`] is returned. If no record can be found for the UUID, a [`crate::Error::NoRecordFound`] will be returned.
pub async fn update(
    uuid: &str,
    task: TaskChangeset,
    db_pool: &crate::DbPool,
) -> Result<Task, crate::Error> {
    task.validate()?;

    let result = sqlx::query!(
        "UPDATE tasks SET description = ?1 WHERE uuid = ?2",
        task.description,
        uuid
    )
    .execute(db_pool)
    .await
    .map_err(crate::Error::DbError)?;

    if result.rows_affected() == 0 {
        return Err(crate::Error::NoRecordFound);
    }

    // Fetch the updated record to get the id
    sqlx::query_as!(
        Task,
        r#"SELECT id as "id!", uuid, description FROM tasks WHERE uuid = ?1"#,
        uuid
    )
    .fetch_one(db_pool)
    .await
    .map_err(crate::Error::DbError)
}
