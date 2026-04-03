use axum::{
    body::Body,
    http::{self, Method},
};
use cinema_booking_db::entities::bookings::Booking;
use cinema_booking_db::entities::movies::{self, MovieChangeset};
use cinema_booking_db::entities::users;
use cinema_booking_macros::db_test;
use cinema_booking_web::test_helpers::{BodyExt, DbTestContext, RouterExt};
use googletest::prelude::*;
use hyper::StatusCode;
use serde_json::json;

const TEST_AUTH: &str = "Bearer test-bearer-token";

async fn seed_auth_user(pool: &cinema_booking_db::DbPool) {
    users::upsert_for_auth_subject("test-sub", "test user", "", pool)
        .await
        .expect("seed auth user");
}

#[db_test]
async fn test_bookings_hold_checkout_list_happy_path(context: &DbTestContext) {
    let config = cinema_booking_config::load_config::<cinema_booking_config::Config>(
        &cinema_booking_config::Environment::Test,
    )
    .expect("load config");
    let Ok(client) = redis::Client::open(config.redis.url.as_str()) else {
        eprintln!("redis unavailable: cannot build client");
        return;
    };
    if client.get_multiplexed_async_connection().await.is_err() {
        eprintln!("redis unavailable: cannot connect");
        return;
    }

    seed_auth_user(&context.db_pool).await;
    let movie_slug = movies::create(
        MovieChangeset {
            title: "API test movie".into(),
            row_count: 10,
            seats_per_row: 10,
        },
        &context.db_pool,
    )
    .await
    .expect("seed movie")
    .slug;
    let seat_uuid = "s1";

    let hold_payload = json!({
        "movie_slug": movie_slug,
        "seat_uuid": seat_uuid,
    });

    let hold_response = context
        .app
        .request("/bookings/hold")
        .method(Method::POST)
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, TEST_AUTH)
        .body(Body::from(hold_payload.to_string()))
        .send()
        .await;

    assert_that!(hold_response.status(), eq(StatusCode::CREATED));

    let checkout_response = context
        .app
        .request("/bookings/checkout")
        .method(Method::POST)
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, TEST_AUTH)
        .body(Body::from(hold_payload.to_string()))
        .send()
        .await;

    assert_that!(checkout_response.status(), eq(StatusCode::CREATED));
    let booking: Booking = checkout_response.into_body().into_json::<Booking>().await;
    assert_that!(booking.movie_slug, eq(&movie_slug));
    assert_that!(booking.seat_uuid, eq(seat_uuid));
    assert_that!(booking.user_uuid, eq("test-sub"));

    let list_uri = format!("/bookings/movies/{movie_slug}");
    let list_response = context
        .app
        .request(&list_uri)
        .method(Method::GET)
        .header(http::header::AUTHORIZATION, TEST_AUTH)
        .send()
        .await;

    assert_that!(list_response.status(), eq(StatusCode::OK));
    let list: Vec<Booking> = list_response.into_body().into_json::<Vec<Booking>>().await;
    assert_that!(list, len(eq(1)));
    assert_that!(list[0].uuid, eq(&booking.uuid));
}

#[db_test]
async fn test_bookings_hold_switches_to_latest_seat_for_same_user(context: &DbTestContext) {
    let config = cinema_booking_config::load_config::<cinema_booking_config::Config>(
        &cinema_booking_config::Environment::Test,
    )
    .expect("load config");
    let Ok(client) = redis::Client::open(config.redis.url.as_str()) else {
        eprintln!("redis unavailable: cannot build client");
        return;
    };
    if client.get_multiplexed_async_connection().await.is_err() {
        eprintln!("redis unavailable: cannot connect");
        return;
    }

    seed_auth_user(&context.db_pool).await;
    let movie_slug = movies::create(
        MovieChangeset {
            title: "API switch seat movie".into(),
            row_count: 10,
            seats_per_row: 10,
        },
        &context.db_pool,
    )
    .await
    .expect("seed movie")
    .slug;

    let first_hold_payload = json!({
        "movie_slug": movie_slug,
        "seat_uuid": "s1",
    });
    let second_hold_payload = json!({
        "movie_slug": movie_slug,
        "seat_uuid": "s2",
    });

    let first_hold_response = context
        .app
        .request("/bookings/hold")
        .method(Method::POST)
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, TEST_AUTH)
        .body(Body::from(first_hold_payload.to_string()))
        .send()
        .await;
    assert_that!(first_hold_response.status(), eq(StatusCode::CREATED));

    let second_hold_response = context
        .app
        .request("/bookings/hold")
        .method(Method::POST)
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, TEST_AUTH)
        .body(Body::from(second_hold_payload.to_string()))
        .send()
        .await;
    assert_that!(second_hold_response.status(), eq(StatusCode::CREATED));

    let old_checkout_response = context
        .app
        .request("/bookings/checkout")
        .method(Method::POST)
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, TEST_AUTH)
        .body(Body::from(first_hold_payload.to_string()))
        .send()
        .await;
    assert_that!(old_checkout_response.status(), eq(StatusCode::NOT_FOUND));

    let checkout_response = context
        .app
        .request("/bookings/checkout")
        .method(Method::POST)
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, TEST_AUTH)
        .body(Body::from(second_hold_payload.to_string()))
        .send()
        .await;
    assert_that!(checkout_response.status(), eq(StatusCode::CREATED));
    let booking: Booking = checkout_response.into_body().into_json::<Booking>().await;
    assert_that!(booking.seat_uuid, eq("s2"));

    let list_uri = format!("/bookings/movies/{movie_slug}");
    let list_response = context
        .app
        .request(&list_uri)
        .method(Method::GET)
        .header(http::header::AUTHORIZATION, TEST_AUTH)
        .send()
        .await;
    assert_that!(list_response.status(), eq(StatusCode::OK));
    let list: Vec<Booking> = list_response.into_body().into_json::<Vec<Booking>>().await;
    assert_that!(list, len(eq(1)));
    assert_that!(list[0].seat_uuid, eq("s2"));
}
