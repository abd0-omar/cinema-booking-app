//! The cinema-booking-db crate contains all code related to database access: entities, migrations, functions for validating and reading and writing data.

use anyhow::{Context, Result};
use cinema_booking_config::DatabaseConfig;
use sqlx::sqlite::{
    SqliteAutoVacuum, SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous,
};
use sqlx::{Sqlite, Transaction};
use std::str::FromStr;
use std::time::Duration;
use thiserror::Error;

pub use sqlx::SqlitePool as DbPool;

/// Entity definitions and related functions
pub mod entities;

/// Creates connection options with optimal PRAGMA settings for performance.
///
/// These settings are based on best practices from SQLite optimization guides:
/// - WAL mode for concurrent reads/writes
/// - Synchronous NORMAL for balanced performance and safety
/// - Memory-mapped I/O for faster access
/// - Foreign key enforcement for data integrity
fn create_connect_options(url: &str) -> Result<SqliteConnectOptions, anyhow::Error> {
    let options = SqliteConnectOptions::from_str(url)
        .context("Failed to parse database URL")?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_secs(5))
        .foreign_keys(true)
        .auto_vacuum(SqliteAutoVacuum::Incremental)
        .page_size(8192)
        // These settings don't have dedicated methods, use pragma()
        .pragma("cache_size", "-20000") // 20MB cache
        .pragma("mmap_size", "2147483648") // 2GB memory-mapped I/O
        .pragma("temp_store", "MEMORY"); // Store temp tables in memory

    Ok(options)
}

/// Starts a new database transaction.
///
/// Example:
/// ```
/// let tx = transaction(&app_state.db_pool).await?;
/// tasks::create(task_data, &mut *tx)?;
/// users::create(user_data, &mut *tx)?;
///
/// match tx.commit().await {
///     Ok(_) => Ok((StatusCode::CREATED, Json(results))),
///     Err(e) => Err((internal_error(e), "".into())),
/// }
/// ```
///
/// Transactions are rolled back automatically when they are dropped without having been committed.
pub async fn transaction(db_pool: &DbPool) -> Result<Transaction<'static, Sqlite>, anyhow::Error> {
    let tx = db_pool
        .begin()
        .await
        .context("Failed to begin transaction")?;

    Ok(tx)
}

/// Errors that can occur as a result of a data layer operation.
#[derive(Error, Debug)]
pub enum Error {
    /// General database error, e.g. communicating with the database failed
    #[error("database query failed")]
    DbError(#[from] sqlx::Error),
    /// No record was found, e.g. when loading a record by ID. This variant is different from
    /// `Error::DbError(sqlx::Error::RowNotFound)` in that the latter indicates a bug, and
    /// `Error::NoRecordFound` does not. It merely originates from [sqlx::Executor::fetch_optional]
    /// returning `None`.
    #[error("no record found")]
    NoRecordFound,
    #[error("validation failed")]
    /// An invalid changeset was passed to a writing operation such as creating or updating a record.
    ValidationError(#[from] validator::ValidationErrors),
}

/// Creates a connection pool to the database specified in the passed [`cinema-booking-config::DatabaseConfig`]
///
/// The connection pool is configured with optimal SQLite PRAGMA settings for performance.
pub async fn connect_pool(config: DatabaseConfig) -> Result<DbPool, anyhow::Error> {
    let options = create_connect_options(&config.url)?;

    let pool = SqlitePoolOptions::new()
        .connect_with(options)
        .await
        .context("Failed to connect to database")?;

    Ok(pool)
}

/// Runs database migrations.
///
/// This embeds the migration files into the binary and runs them on startup.
pub async fn run_migrations(pool: &DbPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}

/// Functionality for working with data that is only relevant in tests but not as part of the normal application flow.
#[cfg(feature = "test-helpers")]
pub mod test_helpers;
