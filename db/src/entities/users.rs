use serde::Serialize;
use sqlx::Sqlite;

/// A user record.
#[derive(Serialize, Debug, Clone)]
pub struct User {
    /// The internal id of the record (aliases rowid for speed).
    pub id: i64,
    /// The external UUID for URLs/APIs.
    pub uuid: String,
    /// The user's name.
    pub name: String,
}

/// Loads a user based on the passed token.
///
/// If no user exists for the token, [`Option::None`] is returned, otherwise `Option::Some(User)` is returned.
pub async fn load_with_token(
    token: &str,
    executor: impl sqlx::Executor<'_, Database = Sqlite>,
) -> Result<Option<User>, anyhow::Error> {
    Ok(sqlx::query_as!(
        User,
        r#"SELECT id as "id!", uuid, name FROM users WHERE token = ?1"#,
        token
    )
    .fetch_optional(executor)
    .await?)
}

/// Loads a user based on the passed UUID.
///
/// If no user exists for the UUID, [`Option::None`] is returned, otherwise `Option::Some(User)` is returned.
pub async fn load(
    uuid: &str,
    executor: impl sqlx::Executor<'_, Database = Sqlite>,
) -> Result<Option<User>, anyhow::Error> {
    Ok(sqlx::query_as!(
        User,
        r#"SELECT id as "id!", uuid, name FROM users WHERE uuid = ?1"#,
        uuid
    )
    .fetch_optional(executor)
    .await?)
}
