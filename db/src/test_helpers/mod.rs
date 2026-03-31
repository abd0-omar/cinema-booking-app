use crate::{connect_pool, DbPool};
use cinema_booking_config::DatabaseConfig;
use rand::distr::Alphanumeric;
use rand::{rng, Rng};
use std::path::Path;
use std::sync::Arc;

/// All test functionality related to the [`crate::entities::users::User`] entity
pub mod users;

/// Sets up a dedicated database to be used in a test case.
///
/// This sets up a dedicated SQLite database file for each test case. The database can be used in a test case to ensure the test case is isolated from other test cases. The function returns a connection pool connected to the created database.
/// This function is automatically called by the [`cinema-booking-macros::db_test`] macro. The return connection pool is passed to the test case via the [`cinema-booking-macros::DbTestContext`].
#[allow(unused)]
pub async fn setup_db(config: &DatabaseConfig) -> DbPool {
    let test_db_config = prepare_db(config).await;
    let pool = connect_pool(test_db_config)
        .await
        .expect("Could not connect to database!");

    // Run migrations (PRAGMA settings are applied via connect options)
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Could not run migrations!");

    pool
}

/// Drops a dedicated database for a test case.
///
/// This function is automatically called by the [`cinema-booking-macros::db_test`] macro. It ensures test-specific database are cleaned up after each test run so we don't end up with large numbers of unused databases.
pub async fn teardown_db(db_pool: DbPool) {
    let mut connect_options = db_pool.connect_options();
    let db_config = Arc::make_mut(&mut connect_options);

    // Get the database filename from the connection options
    let db_filename = db_config.get_filename();

    // Close the pool first
    db_pool.close().await;

    // Delete the database file and related WAL/SHM files if they exist
    if let Some(filename) = db_filename.to_str() {
        let path = Path::new(filename);
        if path.exists() {
            std::fs::remove_file(path).ok();
        }
        // Also remove WAL and SHM files
        let wal_path = path.with_extension("sqlite-wal");
        let shm_path = path.with_extension("sqlite-shm");
        if wal_path.exists() {
            std::fs::remove_file(&wal_path).ok();
        }
        if shm_path.exists() {
            std::fs::remove_file(&shm_path).ok();
        }
    }
}

async fn prepare_db(config: &DatabaseConfig) -> DatabaseConfig {
    // Generate a unique test database filename
    let test_db_suffix: String = rng()
        .sample_iter(&Alphanumeric)
        .take(30)
        .map(char::from)
        .collect();

    // Extract base path from the URL (e.g., "sqlite:./data/test.sqlite" -> "./data/test")
    let base_url = config.url.strip_prefix("sqlite:").unwrap_or(&config.url);

    // Remove query parameters if present
    let base_path = base_url.split('?').next().unwrap_or(base_url);

    // Remove extension and add unique suffix
    let path = Path::new(base_path);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("test");
    let parent = path.parent().unwrap_or(Path::new("."));

    // Ensure parent directory exists
    if !parent.exists() {
        std::fs::create_dir_all(parent).expect("Failed to create test database directory");
    }

    let test_db_path = parent.join(format!("{}_{}.sqlite", stem, test_db_suffix.to_lowercase()));
    let test_db_url = format!("sqlite:{}?mode=rwc", test_db_path.display());

    DatabaseConfig { url: test_db_url }
}
