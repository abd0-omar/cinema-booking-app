use async_trait::async_trait;
use cinema_booking_store_port::{
    BookingStoreError, SeatHoldChangeset, SeatHoldStore, SeatReservationSession,
    SeatReservationStatus,
};
use redis::{aio::MultiplexedConnection, Script};
use uuid::Uuid;

const CONFIRM_SCRIPT: &str = r#"
local key = KEYS[1]
local user_uuid = ARGV[1]
local payload = redis.call('GET', key)
if not payload then
  return {0, ''}
end
local session = cjson.decode(payload)
if session['user_uuid'] ~= user_uuid then
  return {1, ''}
end
session['status'] = 'confirmed'
local updated = cjson.encode(session)
redis.call('DEL', key)
return {2, updated}
"#;

/// redis seat hold flow
/// create session          ->        confirm/buy that seat
///   hold()                                confirm()
///   TTL reserve seat                     consume hold key
///
/// Runtime configuration for [`RedisSeatHoldStore`].
#[derive(Debug, Clone)]
pub struct RedisSeatHoldStoreConfig {
    pub url: String,
    pub key_prefix: String,
    pub hold_ttl_seconds: u64,
}

impl Default for RedisSeatHoldStoreConfig {
    fn default() -> Self {
        Self {
            url: "redis://127.0.0.1:6379/".to_string(),
            key_prefix: String::new(),
            hold_ttl_seconds: 420,
        }
    }
}

/// Redis-backed seat hold store for hold/confirm reservation flows.
pub struct RedisSeatHoldStore {
    client: redis::Client,
    key_prefix: String,
    hold_ttl_seconds: u64,
}

impl RedisSeatHoldStore {
    pub fn new(config: RedisSeatHoldStoreConfig) -> Result<Self, BookingStoreError> {
        if config.hold_ttl_seconds == 0 {
            return Err(BookingStoreError::Validation(
                "hold_ttl_seconds must be greater than 0".to_string(),
            ));
        }
        let client = redis::Client::open(config.url)
            .map_err(|e| BookingStoreError::Internal(format!("redis client init failed: {e}")))?;
        Ok(Self {
            client,
            key_prefix: config.key_prefix.trim().trim_matches(':').to_string(),
            hold_ttl_seconds: config.hold_ttl_seconds,
        })
    }

    async fn connection(&self) -> Result<MultiplexedConnection, BookingStoreError> {
        self.client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| BookingStoreError::Internal(format!("redis connect failed: {e}")))
    }

    fn seat_key(&self, movie_uuid: &str, seat_uuid: &str) -> String {
        if self.key_prefix.is_empty() {
            format!("seat:{movie_uuid}:{seat_uuid}")
        } else {
            format!("{}:seat:{movie_uuid}:{seat_uuid}", self.key_prefix)
        }
    }
}

#[async_trait]
impl SeatHoldStore for RedisSeatHoldStore {
    async fn hold(
        &self,
        changeset: SeatHoldChangeset,
    ) -> Result<SeatReservationSession, BookingStoreError> {
        changeset.validate()?;
        let key = self.seat_key(&changeset.movie_uuid, &changeset.seat_uuid);
        let session = SeatReservationSession {
            session_uuid: Uuid::new_v4().to_string(),
            movie_uuid: changeset.movie_uuid,
            seat_uuid: changeset.seat_uuid,
            user_uuid: changeset.user_uuid,
            status: SeatReservationStatus::Held,
        };
        let payload = serde_json::to_string(&session)
            .map_err(|e| BookingStoreError::Serialization(e.to_string()))?;

        let mut connection = self.connection().await?;
        let result: Option<String> = redis::cmd("SET")
            .arg(&key)
            .arg(payload)
            .arg("NX")
            .arg("EX")
            .arg(self.hold_ttl_seconds)
            .query_async(&mut connection)
            .await
            .map_err(|e| BookingStoreError::Internal(format!("redis SET failed: {e}")))?;

        if result.is_none() {
            return Err(BookingStoreError::SeatUnavailable);
        }

        Ok(session)
    }

    async fn confirm(
        &self,
        movie_uuid: &str,
        seat_uuid: &str,
        user_uuid: &str,
    ) -> Result<SeatReservationSession, BookingStoreError> {
        if movie_uuid.trim().is_empty() {
            return Err(BookingStoreError::Validation(
                "movie_uuid must not be empty".to_string(),
            ));
        }
        if seat_uuid.trim().is_empty() {
            return Err(BookingStoreError::Validation(
                "seat_uuid must not be empty".to_string(),
            ));
        }
        if user_uuid.trim().is_empty() {
            return Err(BookingStoreError::Validation(
                "user_uuid must not be empty".to_string(),
            ));
        }

        let key = self.seat_key(movie_uuid, seat_uuid);
        let mut connection = self.connection().await?;
        let script = Script::new(CONFIRM_SCRIPT);
        let (status, payload): (i64, String) = script
            .key(&key)
            .arg(user_uuid)
            .invoke_async(&mut connection)
            .await
            .map_err(|e| BookingStoreError::Internal(format!("redis confirm failed: {e}")))?;

        match status {
            0 => Err(BookingStoreError::SessionNotFound),
            1 => Err(BookingStoreError::SessionOwnershipMismatch),
            2 => serde_json::from_str::<SeatReservationSession>(&payload)
                .map_err(|e| BookingStoreError::Serialization(e.to_string())),
            _ => Err(BookingStoreError::Internal(
                "redis confirm returned unknown status".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use redis::AsyncCommands;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_prefix() -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        format!("redis-adapter-test-{nanos}")
    }

    async fn redis_store() -> Option<RedisSeatHoldStore> {
        let config = RedisSeatHoldStoreConfig {
            key_prefix: unique_prefix(),
            ..RedisSeatHoldStoreConfig::default()
        };
        let store = RedisSeatHoldStore::new(config).ok()?;
        if store.connection().await.is_err() {
            return None;
        }
        Some(store)
    }

    #[tokio::test]
    async fn hold_conflict_confirm_and_consume() {
        let Some(store) = redis_store().await else {
            return;
        };
        let hold = SeatHoldChangeset {
            movie_uuid: "m1".to_string(),
            seat_uuid: "s1".to_string(),
            user_uuid: "u1".to_string(),
        };
        let first = store.hold(hold.clone()).await.unwrap();
        assert_eq!(first.status, SeatReservationStatus::Held);

        let second = store.hold(hold.clone()).await.unwrap_err();
        assert!(matches!(second, BookingStoreError::SeatUnavailable));

        let confirmed = store
            .confirm(&hold.movie_uuid, &hold.seat_uuid, &hold.user_uuid)
            .await
            .unwrap();
        assert_eq!(confirmed.status, SeatReservationStatus::Confirmed);

        let key = store.seat_key(&hold.movie_uuid, &hold.seat_uuid);
        let mut conn = store.connection().await.unwrap();
        let ttl: i64 = conn.ttl(key).await.unwrap();
        assert_eq!(ttl, -2);
    }

    #[tokio::test]
    async fn confirm_requires_ownership() {
        let Some(store) = redis_store().await else {
            return;
        };
        let hold = SeatHoldChangeset {
            movie_uuid: "m2".to_string(),
            seat_uuid: "s2".to_string(),
            user_uuid: "owner".to_string(),
        };
        store.hold(hold.clone()).await.unwrap();
        let err = store
            .confirm(&hold.movie_uuid, &hold.seat_uuid, "intruder")
            .await
            .unwrap_err();
        assert!(matches!(err, BookingStoreError::SessionOwnershipMismatch));
    }

    #[tokio::test]
    async fn confirm_after_expiry_not_found() {
        let Some(store) = redis_store().await else {
            return;
        };
        let quick_store = RedisSeatHoldStore::new(RedisSeatHoldStoreConfig {
            hold_ttl_seconds: 1,
            key_prefix: unique_prefix(),
            ..RedisSeatHoldStoreConfig::default()
        })
        .unwrap();
        let hold = SeatHoldChangeset {
            movie_uuid: "m3".to_string(),
            seat_uuid: "s3".to_string(),
            user_uuid: "u3".to_string(),
        };
        quick_store.hold(hold.clone()).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let err = quick_store
            .confirm(&hold.movie_uuid, &hold.seat_uuid, &hold.user_uuid)
            .await
            .unwrap_err();
        assert!(matches!(err, BookingStoreError::SessionNotFound));

        let _ = store; // keep connection helper used in this module across tests
    }
}
