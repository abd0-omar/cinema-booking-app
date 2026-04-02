use async_trait::async_trait;
use chrono::{Duration, Utc};
use cinema_booking_store_port::{
    BookingStoreError, SeatHoldChangeset, SeatHoldStore, SeatReservationSession,
    SeatReservationStatus,
};
use redis::{aio::MultiplexedConnection, AsyncCommands, Script};
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
            hold_ttl_seconds: 120,
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

    fn seat_key(&self, movie_slug: &str, seat_uuid: &str) -> String {
        if self.key_prefix.is_empty() {
            format!("seat:{movie_slug}:{seat_uuid}")
        } else {
            format!("{}:seat:{movie_slug}:{seat_uuid}", self.key_prefix)
        }
    }

    /// Glob pattern for [`SCAN`](https://redis.io/commands/scan/) over all seat keys for one movie.
    fn seat_keys_pattern(&self, movie_slug: &str) -> String {
        if self.key_prefix.is_empty() {
            format!("seat:{movie_slug}:*")
        } else {
            format!("{}:seat:{movie_slug}:*", self.key_prefix)
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
        let key = self.seat_key(&changeset.movie_slug, &changeset.seat_uuid);
        let ttl = i64::try_from(self.hold_ttl_seconds).map_err(|_| {
            BookingStoreError::Validation("hold_ttl_seconds is too large".to_string())
        })?;
        let expires_at = Utc::now()
            + Duration::try_seconds(ttl).ok_or_else(|| {
                BookingStoreError::Validation("hold_ttl_seconds is out of range".to_string())
            })?;
        let session = SeatReservationSession {
            session_uuid: Uuid::new_v4().to_string(),
            movie_slug: changeset.movie_slug,
            seat_uuid: changeset.seat_uuid,
            user_uuid: changeset.user_uuid,
            status: SeatReservationStatus::Held,
            expires_at,
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
        movie_slug: &str,
        seat_uuid: &str,
        user_uuid: &str,
    ) -> Result<SeatReservationSession, BookingStoreError> {
        if movie_slug.trim().is_empty() {
            return Err(BookingStoreError::Validation(
                "movie_slug must not be empty".to_string(),
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

        let key = self.seat_key(movie_slug, seat_uuid);
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

    async fn list_held_sessions_for_movie(
        &self,
        movie_slug: &str,
    ) -> Result<Vec<SeatReservationSession>, BookingStoreError> {
        if movie_slug.trim().is_empty() {
            return Err(BookingStoreError::Validation(
                "movie_slug must not be empty".to_string(),
            ));
        }

        let pattern = self.seat_keys_pattern(movie_slug);
        let mut connection = self.connection().await?;

        let keys: Vec<String> = {
            let mut iter: redis::AsyncIter<String> = connection
                .scan_match(&pattern)
                .await
                .map_err(|e| BookingStoreError::Internal(format!("redis SCAN failed: {e}")))?;
            let mut out = Vec::new();
            while let Some(key) = iter.next_item().await {
                out.push(key);
            }
            out
        };

        let mut sessions = Vec::new();
        for key in keys {
            let payload: Option<String> = redis::cmd("GET")
                .arg(&key)
                .query_async(&mut connection)
                .await
                .map_err(|e| BookingStoreError::Internal(format!("redis GET failed: {e}")))?;
            let Some(payload) = payload else {
                continue;
            };
            let Ok(session) = serde_json::from_str::<SeatReservationSession>(&payload) else {
                continue;
            };
            if session.status == SeatReservationStatus::Held && session.movie_slug == movie_slug {
                sessions.push(session);
            }
        }

        Ok(sessions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
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
            movie_slug: "m1".to_string(),
            seat_uuid: "s1".to_string(),
            user_uuid: "u1".to_string(),
        };
        let first = store.hold(hold.clone()).await.unwrap();
        assert_eq!(first.status, SeatReservationStatus::Held);
        let now = Utc::now();
        assert!(first.expires_at > now);
        assert!(first.expires_at < now + Duration::seconds(125));

        let second = store.hold(hold.clone()).await.unwrap_err();
        assert!(matches!(second, BookingStoreError::SeatUnavailable));

        let confirmed = store
            .confirm(&hold.movie_slug, &hold.seat_uuid, &hold.user_uuid)
            .await
            .unwrap();
        assert_eq!(confirmed.status, SeatReservationStatus::Confirmed);
        assert_eq!(confirmed.expires_at, first.expires_at);

        let key = store.seat_key(&hold.movie_slug, &hold.seat_uuid);
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
            movie_slug: "m2".to_string(),
            seat_uuid: "s2".to_string(),
            user_uuid: "owner".to_string(),
        };
        store.hold(hold.clone()).await.unwrap();
        let err = store
            .confirm(&hold.movie_slug, &hold.seat_uuid, "intruder")
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
            movie_slug: "m3".to_string(),
            seat_uuid: "s3".to_string(),
            user_uuid: "u3".to_string(),
        };
        quick_store.hold(hold.clone()).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let err = quick_store
            .confirm(&hold.movie_slug, &hold.seat_uuid, &hold.user_uuid)
            .await
            .unwrap_err();
        assert!(matches!(err, BookingStoreError::SessionNotFound));

        let _ = store; // keep connection helper used in this module across tests
    }

    #[tokio::test]
    async fn list_held_sessions_for_movie_lists_held_and_drops_after_confirm() {
        let Some(store) = redis_store().await else {
            return;
        };
        let movie = "list-held-movie";
        let hold1 = SeatHoldChangeset {
            movie_slug: movie.into(),
            seat_uuid: "s1".into(),
            user_uuid: "u1".into(),
        };
        let hold2 = SeatHoldChangeset {
            movie_slug: movie.into(),
            seat_uuid: "s2".into(),
            user_uuid: "u2".into(),
        };
        store.hold(hold1.clone()).await.unwrap();
        store.hold(hold2.clone()).await.unwrap();

        let mut list = store.list_held_sessions_for_movie(movie).await.unwrap();
        assert_eq!(list.len(), 2);
        list.sort_by(|a, b| a.seat_uuid.cmp(&b.seat_uuid));
        assert_eq!(list[0].seat_uuid, "s1");
        assert_eq!(list[1].seat_uuid, "s2");

        store
            .confirm(movie, &hold1.seat_uuid, &hold1.user_uuid)
            .await
            .unwrap();
        let after = store.list_held_sessions_for_movie(movie).await.unwrap();
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].seat_uuid, "s2");
    }
}
