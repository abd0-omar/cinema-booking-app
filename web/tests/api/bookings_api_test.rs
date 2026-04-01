use axum::{
    body::Body,
    http::{self, Method},
};
use cinema_booking_db::entities::bookings::Booking;
use cinema_booking_db::test_helpers::users::{create as create_user, UserChangeset};
use cinema_booking_macros::db_test;
use cinema_booking_web::test_helpers::{BodyExt, DbTestContext, RouterExt};
use fake::{Fake, Faker};
use googletest::prelude::*;
use hyper::StatusCode;
use serde_json::json;

const TEST_AUTH: &str = "Bearer test-bearer-token";

async fn seed_user_uuid(pool: &cinema_booking_db::DbPool) -> String {
    let user_changeset: UserChangeset = Faker.fake();
    create_user(user_changeset, pool)
        .await
        .expect("seed user")
        .uuid
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

    let user_uuid = seed_user_uuid(&context.db_pool).await;
    let movie_uuid = "movie-api-1";
    let seat_uuid = "seat-api-1";

    let hold_payload = json!({
        "movie_uuid": movie_uuid,
        "seat_uuid": seat_uuid,
        "user_uuid": user_uuid,
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
    assert_that!(booking.movie_uuid, eq(movie_uuid));
    assert_that!(booking.seat_uuid, eq(seat_uuid));
    assert_that!(booking.user_uuid, eq(&user_uuid));

    let list_uri = format!("/bookings/movies/{movie_uuid}");
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
