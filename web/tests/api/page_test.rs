use axum::body::Body;
use axum::http::{self, header, Method};
use cinema_booking_macros::db_test;
use cinema_booking_web::test_helpers::{BodyExt, DbTestContext, RouterExt};
use googletest::prelude::*;
use hyper::StatusCode;

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
