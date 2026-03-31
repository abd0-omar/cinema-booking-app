use cinema_booking_config::{load_config, Config, Environment};
use cinema_booking_macros::db_test;
use cinema_booking_web::test_helpers::DbTestContext;

#[db_test]
async fn test_db_test_context_supports_redis_namespace(context: &DbTestContext) {
    assert!(!context.redis_key_prefix.is_empty());
    assert!(context.redis_key_prefix.starts_with("db-test:"));

    let config: Config = load_config(&Environment::Test).expect("load test config");
    let Ok(client) = redis::Client::open(config.redis.url) else {
        eprintln!("redis unavailable: cannot build client");
        return;
    };
    let Ok(mut conn) = client.get_multiplexed_async_connection().await else {
        eprintln!("redis unavailable: cannot connect");
        return;
    };

    let key = format!("{}:probe", context.redis_key_prefix);
    redis::cmd("SET")
        .arg(&key)
        .arg("ok")
        .query_async::<String>(&mut conn)
        .await
        .expect("set probe key");

    let value: String = redis::cmd("GET")
        .arg(&key)
        .query_async(&mut conn)
        .await
        .expect("get probe key");
    assert_eq!(value, "ok");
}
