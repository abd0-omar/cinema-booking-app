use anyhow::Context;
use cinema_booking_auth::AccessTokenVerifier;
use cinema_booking_auth_adapter_trailbase::TrailbaseJwtVerifier;
use cinema_booking_config::{Config, Environment};
use cinema_booking_db::{connect_pool, DbPool};
use cinema_booking_store_adapter_redis::{RedisSeatHoldStore, RedisSeatHoldStoreConfig};
use cinema_booking_store_port::SeatHoldStore;
use std::sync::Arc;

/// The application's state that is available in [`crate::controllers`] and [`crate::middlewares`].
pub struct AppState {
    /// The database pool that's used to get a connection to the application's database (see [`cinema_booking_db::DbPool`]).
    pub db_pool: DbPool,
    /// Validates `Authorization: Bearer` JWTs (Trailbase sidecar in production; see config).
    pub access_token_verifier: Arc<dyn AccessTokenVerifier + Send + Sync>,
    /// Trailbase HTTP API base URL (e.g. `http://127.0.0.1:4000`) for server-side login; [`None`] disables `POST /login`.
    pub trailbase_base_url: Option<String>,
    /// Redis-backed seat hold / confirm (`hold` before durable checkout).
    pub seat_hold_store: Arc<dyn SeatHoldStore>,
}

/// The application's state as it is shared across the application, e.g. in controllers and middlewares.
///
/// This is the [`AppState`] struct wrappend in an [`std::sync::Arc`].
pub type SharedAppState = Arc<AppState>;

/// Initializes the application state (connects using `config.database`).
///
/// For tests that use a per-case database URL, use [`app_state_from_pool`] instead.
pub async fn init_app_state(config: Config, environment: &Environment) -> AppState {
    let db_pool = connect_pool(config.database.clone())
        .await
        .expect("Could not connect to database!");
    app_state_from_pool(db_pool, &config, environment, None).await
}

/// Builds [`AppState`] with an existing pool (e.g. integration tests via [`setup_db`]).
///
/// In [`Environment::Test`] with the `test-helpers` feature, a fixed bearer token is accepted;
/// otherwise configure a Trailbase JWT public key.
///
/// `redis_key_prefix_override`: when set (e.g. per-test namespace from [`crate::test_helpers::DbTestContext`]),
/// replaces `config.redis.key_prefix` so Redis keys match teardown cleanup.
pub async fn app_state_from_pool(
    db_pool: DbPool,
    config: &Config,
    environment: &Environment,
    redis_key_prefix_override: Option<&str>,
) -> AppState {
    let access_token_verifier = build_access_token_verifier(config, environment)
        .await
        .expect("Could not initialize access token verifier");

    let seat_hold_store = build_seat_hold_store(&config.redis, redis_key_prefix_override);

    AppState {
        db_pool,
        access_token_verifier,
        trailbase_base_url: config.trailbase.base_url.clone(),
        seat_hold_store,
    }
}

fn build_seat_hold_store(
    redis: &cinema_booking_config::RedisConfig,
    key_prefix_override: Option<&str>,
) -> Arc<dyn SeatHoldStore> {
    let key_prefix = key_prefix_override
        .map(str::to_string)
        .unwrap_or_else(|| redis.key_prefix.clone());
    let store_config = RedisSeatHoldStoreConfig {
        url: redis.url.clone(),
        key_prefix,
        hold_ttl_seconds: redis.hold_ttl_seconds,
    };
    let store = RedisSeatHoldStore::new(store_config).expect(
        "Could not initialize Redis seat hold store (check redis.url and redis.hold_ttl_seconds)",
    );
    Arc::new(store)
}

async fn build_access_token_verifier(
    config: &Config,
    environment: &Environment,
) -> anyhow::Result<Arc<dyn AccessTokenVerifier + Send + Sync>> {
    #[cfg(feature = "test-helpers")]
    if *environment == Environment::Test {
        use cinema_booking_auth::FixedTokenVerifier;

        let user = cinema_booking_auth::Principal {
            sub: "test-sub".to_string(),
            email: "test@example.com".to_string(),
            is_admin: false,
            mfa: false,
            csrf_token: String::new(),
        };
        let admin = cinema_booking_auth::Principal {
            is_admin: true,
            ..user.clone()
        };
        return Ok(Arc::new(FixedTokenVerifier::from_pairs([
            ("test-bearer-token".to_string(), user),
            ("test-admin-bearer-token".to_string(), admin),
        ])));
    }

    #[cfg(not(feature = "test-helpers"))]
    if *environment == Environment::Test {
        anyhow::bail!(
            "Test environment requires the `test-helpers` feature on cinema-booking-web \
             (e.g. integration tests) to install FixedTokenVerifier"
        );
    }

    let pem = if let Some(path) = &config.trailbase.jwt_public_key_path {
        tokio::fs::read_to_string(path)
            .await
            .with_context(|| format!("read trailbase JWT public key PEM from {}", path.display()))?
    } else if let Some(pem) = &config.trailbase.jwt_public_key_pem {
        pem.clone()
    } else {
        anyhow::bail!(
            "trailbase.jwt_public_key_path or trailbase.jwt_public_key_pem is required \
             (e.g. APP_TRAILBASE__JWT_PUBLIC_KEY_PATH or APP_TRAILBASE__JWT_PUBLIC_KEY_PEM)"
        );
    };

    let verifier = TrailbaseJwtVerifier::from_public_key_pem(pem.as_bytes())
        .map_err(|e| anyhow::anyhow!("invalid Trailbase JWT public key PEM: {e}"))?;

    Ok(Arc::new(verifier))
}
