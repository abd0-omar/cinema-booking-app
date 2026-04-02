use axum::body::Body;
use axum::http::{self, header, Method};
use cinema_booking_db::entities::movies::{create as create_movie, MovieChangeset};
use cinema_booking_macros::db_test;
use cinema_booking_web::test_helpers::{BodyExt, DbTestContext, RouterExt};
use googletest::prelude::*;
use hyper::StatusCode;
use uuid::Uuid;

#[db_test]
async fn test_get_index_html(context: &DbTestContext) {
    let response = context.app.request("/").send().await;

    assert_that!(response.status(), eq(StatusCode::OK));
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .expect("Content-Type")
        .to_str()
        .unwrap();
    assert_that!(content_type, contains_substring("text/html"));

    let body = response.into_body().into_bytes().await;
    let html = String::from_utf8(body.to_vec()).expect("utf8");
    assert_that!(html, contains_substring("Cinema Booking"));
    assert_that!(html, contains_substring("datastar.js"));
    assert_that!(html, contains_substring("user:"));
    assert_that!(html, contains_substring("Log in"));
    assert_that!(html, contains_substring("Sign up"));
}

#[db_test]
async fn test_get_index_lists_seeded_movie(context: &DbTestContext) {
    let title = format!("Index marquee film {}", Uuid::new_v4());
    let movie = create_movie(
        MovieChangeset {
            title: title.clone(),
            row_count: 5,
            seats_per_row: 8,
        },
        &context.db_pool,
    )
    .await
    .expect("seed movie");

    let response = context.app.request("/").send().await;
    assert_that!(response.status(), eq(StatusCode::OK));
    let body = response.into_body().into_bytes().await;
    let html = String::from_utf8(body.to_vec()).expect("utf8");
    assert_that!(html, contains_substring(title.as_str()));
    assert_that!(
        html,
        contains_substring(format!(r#"data-movie-slug="{}""#, movie.slug).as_str())
    );
}

#[db_test]
async fn test_get_login_html(context: &DbTestContext) {
    let response = context.app.request("/login").send().await;

    assert_that!(response.status(), eq(StatusCode::OK));
    let body = response.into_body().into_bytes().await;
    let html = String::from_utf8(body.to_vec()).expect("utf8");
    assert_that!(html, contains_substring("Sign in"));
    assert_that!(html, contains_substring(r#"action="/login""#));
    assert_that!(html, contains_substring("name=\"email\""));
    assert_that!(html, contains_substring("name=\"password\""));
    assert_that!(html, contains_substring(r#"href="/signup""#));
}

#[db_test]
async fn test_get_login_after_signup_shows_success_banner(context: &DbTestContext) {
    let response = context.app.request("/login?r=1").send().await;

    assert_that!(response.status(), eq(StatusCode::OK));
    let body = response.into_body().into_bytes().await;
    let html = String::from_utf8(body.to_vec()).expect("utf8");
    assert_that!(html, contains_substring("Registration accepted"));
}

#[db_test]
async fn test_get_signup_html(context: &DbTestContext) {
    let response = context.app.request("/signup").send().await;

    assert_that!(response.status(), eq(StatusCode::OK));
    let body = response.into_body().into_bytes().await;
    let html = String::from_utf8(body.to_vec()).expect("utf8");
    assert_that!(html, contains_substring("Create account"));
    assert_that!(html, contains_substring(r#"action="/signup""#));
    assert_that!(html, contains_substring("name=\"password_repeat\""));
    assert_that!(html, contains_substring(r#"href="/login""#));
}

#[db_test]
async fn test_post_signup_without_trailbase_base_url_returns_503(context: &DbTestContext) {
    let body = "email=u%40x.test&password=secret&password_repeat=secret";
    let response = context
        .app
        .request("/signup")
        .method(Method::POST)
        .header(
            http::header::CONTENT_TYPE,
            "application/x-www-form-urlencoded",
        )
        .body(Body::from(body))
        .send()
        .await;

    assert_that!(response.status(), eq(StatusCode::SERVICE_UNAVAILABLE));
    let bytes = response.into_body().into_bytes().await;
    let text = String::from_utf8_lossy(&bytes);
    assert_that!(text, contains_substring("trailbase.base_url"));
}

#[db_test]
async fn test_post_signup_password_mismatch_redirects(context: &DbTestContext) {
    let body = "email=u%40x.test&password=one&password_repeat=two";
    let response = context
        .app
        .request("/signup")
        .method(Method::POST)
        .header(
            http::header::CONTENT_TYPE,
            "application/x-www-form-urlencoded",
        )
        .body(Body::from(body))
        .send()
        .await;

    assert_that!(response.status(), eq(StatusCode::SEE_OTHER));
    let loc = response
        .headers()
        .get(http::header::LOCATION)
        .expect("Location")
        .to_str()
        .unwrap();
    assert_that!(loc, contains_substring("mismatch"));
}

#[db_test]
async fn test_post_login_without_trailbase_base_url_returns_503(context: &DbTestContext) {
    let body = "email=u%40x.test&password=secret";
    let response = context
        .app
        .request("/login")
        .method(Method::POST)
        .header(
            http::header::CONTENT_TYPE,
            "application/x-www-form-urlencoded",
        )
        .body(Body::from(body))
        .send()
        .await;

    assert_that!(response.status(), eq(StatusCode::SERVICE_UNAVAILABLE));
    let bytes = response.into_body().into_bytes().await;
    let text = String::from_utf8_lossy(&bytes);
    assert_that!(text, contains_substring("trailbase.base_url"));
}

#[db_test]
async fn test_ds_seat_map_sse_post(context: &DbTestContext) {
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

    let movie = create_movie(
        MovieChangeset {
            title: "SSE seat map film".into(),
            row_count: 2,
            seats_per_row: 2,
        },
        &context.db_pool,
    )
    .await
    .expect("seed movie");

    let body = format!(
        r#"{{"movie_slug":"{}","viewer":"","interval_ms":1,"max_ticks":1}}"#,
        movie.slug
    );
    let response = context
        .app
        .request("/ds/seat-map")
        .method(Method::POST)
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .send()
        .await;

    assert_that!(response.status(), eq(StatusCode::OK));
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .expect("Content-Type")
        .to_str()
        .unwrap();
    assert_that!(content_type, starts_with("text/event-stream"));

    let bytes = response.into_body().into_bytes().await;
    let text = String::from_utf8_lossy(&bytes);
    assert_that!(text, contains_substring("datastar-patch-elements"));
    assert_that!(text, contains_substring("seatGrid"));
    assert_that!(text, contains_substring("s1"));
}

#[db_test]
async fn test_ds_hello_world_sse_post(context: &DbTestContext) {
    let response = context
        .app
        .request("/ds/hello-world")
        .method(Method::POST)
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"delay":1}"#))
        .send()
        .await;

    assert_that!(response.status(), eq(StatusCode::OK));
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .expect("Content-Type")
        .to_str()
        .unwrap();
    assert_that!(content_type, starts_with("text/event-stream"));

    let body = response.into_body().into_bytes().await;
    let text = String::from_utf8_lossy(&body);
    assert_that!(text, contains_substring("event"));
    assert_that!(text, contains_substring("datastar-patch-elements"));
}
