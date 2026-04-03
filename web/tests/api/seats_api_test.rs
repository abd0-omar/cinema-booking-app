use axum::{
    body::Body,
    http::{self, Method},
};
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
async fn test_get_movie_seats_layout_and_states(context: &DbTestContext) {
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
            title: "Seats API film".into(),
            row_count: 2,
            seats_per_row: 3,
        },
        &context.db_pool,
    )
    .await
    .expect("seed movie")
    .slug;

    let uri = format!("/movies/{movie_slug}/seats");
    let response = context.app.request(&uri).method(Method::GET).send().await;
    assert_that!(response.status(), eq(StatusCode::OK));
    let body: serde_json::Value = response.into_body().into_json().await;
    let seats_arr = body["seats"].as_array().expect("seats");
    assert_that!(seats_arr, len(eq(6)));

    let hold_payload = json!({
        "movie_slug": movie_slug,
        "seat_uuid": "s1",
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

    // Hold uses `principal.sub` from the test JWT (`test-sub`), not the JSON body.
    let with_viewer = format!("/movies/{movie_slug}/seats?viewer=test-sub");
    let r2 = context
        .app
        .request(&with_viewer)
        .method(Method::GET)
        .send()
        .await;
    assert_that!(r2.status(), eq(StatusCode::OK));
    let j2: serde_json::Value = r2.into_body().into_json().await;
    let seats = j2["seats"].as_array().expect("seats");
    let s1 = seats.iter().find(|s| s["seat_uuid"] == "s1").expect("s1");
    assert_that!(s1["state"].as_str().expect("state"), eq("your_hold"));

    let r3 = context.app.request(&uri).method(Method::GET).send().await;
    assert_that!(r3.status(), eq(StatusCode::OK));
    let j3: serde_json::Value = r3.into_body().into_json().await;
    let seats3 = j3["seats"].as_array().expect("seats");
    let s1_other = seats3.iter().find(|s| s["seat_uuid"] == "s1").expect("s1");
    assert_that!(s1_other["state"].as_str().expect("state"), eq("other_hold"));

    let checkout = context
        .app
        .request("/bookings/checkout")
        .method(Method::POST)
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, TEST_AUTH)
        .body(Body::from(hold_payload.to_string()))
        .send()
        .await;
    assert_that!(checkout.status(), eq(StatusCode::CREATED));

    let r4 = context.app.request(&uri).method(Method::GET).send().await;
    assert_that!(r4.status(), eq(StatusCode::OK));
    let j4: serde_json::Value = r4.into_body().into_json().await;
    let seats4 = j4["seats"].as_array().expect("seats");
    let s1_booked = seats4.iter().find(|s| s["seat_uuid"] == "s1").expect("s1");
    assert_that!(s1_booked["state"].as_str().expect("state"), eq("confirmed"));
}
