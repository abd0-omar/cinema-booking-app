use axum::{
    body::Body,
    http::{self, Method},
};
use cinema_booking_db::entities::movies::{
    create as create_movie, load as load_movie, load_all as load_movies, Movie, MovieChangeset,
};
use cinema_booking_macros::db_test;
use cinema_booking_web::test_helpers::{BodyExt, DbTestContext, RouterExt};
use googletest::prelude::*;
use hyper::StatusCode;
use serde_json::json;
use uuid::Uuid;

/// Non-admin test token (`FixedTokenVerifier` in test `AppState`).
const TEST_USER_AUTH: &str = "Bearer test-bearer-token";
/// Admin test token for movie POST/PUT/DELETE.
const TEST_ADMIN_AUTH: &str = "Bearer test-admin-bearer-token";

type MoviesList = Vec<Movie>;

fn sample_movie_cs() -> MovieChangeset {
    MovieChangeset {
        title: format!("Movie {}", Uuid::new_v4()),
        movie_time: "in 7 min".into(),
        row_count: 10,
        seats_per_row: 12,
    }
}

#[db_test]
async fn test_create_unauthorized(context: &DbTestContext) {
    let response = context
        .app
        .request("/movies")
        .method(Method::POST)
        .header(http::header::CONTENT_TYPE, "application/json")
        .send()
        .await;

    assert_that!(response.status(), eq(StatusCode::UNAUTHORIZED));
}

#[db_test]
async fn test_create_forbidden(context: &DbTestContext) {
    let payload = json!({
        "title": "No admin",
        "movie_time": "in 5 min",
        "rows": 1,
        "seats_per_rows": 1,
    });

    let response = context
        .app
        .request("/movies")
        .method(Method::POST)
        .body(Body::from(payload.to_string()))
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, TEST_USER_AUTH)
        .send()
        .await;

    assert_that!(response.status(), eq(StatusCode::FORBIDDEN));
}

#[db_test]
async fn test_create_invalid(context: &DbTestContext) {
    let payload = json!({
        "title": "",
        "movie_time": "in 5 min",
        "rows": 1,
        "seats_per_rows": 1,
    });

    let response = context
        .app
        .request("/movies")
        .method(Method::POST)
        .body(Body::from(payload.to_string()))
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, TEST_ADMIN_AUTH)
        .send()
        .await;

    assert_that!(response.status(), eq(StatusCode::UNPROCESSABLE_ENTITY));
}

#[db_test]
async fn test_create_success(context: &DbTestContext) {
    let cs = MovieChangeset {
        title: "Integration Movie".into(),
        movie_time: "in 5 min".into(),
        row_count: 10,
        seats_per_row: 12,
    };
    let payload = json!({
        "title": cs.title,
        "movie_time": cs.movie_time,
        "rows": cs.row_count,
        "seats_per_rows": cs.seats_per_row,
    });

    let response = context
        .app
        .request("/movies")
        .method(Method::POST)
        .body(Body::from(payload.to_string()))
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, TEST_ADMIN_AUTH)
        .send()
        .await;

    assert_that!(response.status(), eq(StatusCode::CREATED));

    let movies = load_movies(&context.db_pool).await.unwrap();
    assert_that!(movies, len(eq(1)));
    let row = movies.first().unwrap();
    assert_that!(row.title, eq(&cs.title));
    assert_that!(row.movie_time, eq(&cs.movie_time));
    assert_that!(row.row_count, eq(cs.row_count));
    assert_that!(row.seats_per_row, eq(cs.seats_per_row));
    assert_that!(row.slug, eq("integration-movie-1"));
}

#[db_test]
async fn test_read_all(context: &DbTestContext) {
    let cs = sample_movie_cs();
    create_movie(cs.clone(), &context.db_pool).await.unwrap();

    let response = context
        .app
        .request("/movies")
        .method(Method::GET)
        .send()
        .await;

    assert_that!(response.status(), eq(StatusCode::OK));

    let list: MoviesList = response.into_body().into_json::<MoviesList>().await;
    assert_that!(list, len(eq(1)));
    assert_that!(list.first().unwrap().title, eq(&cs.title));
    assert_that!(list.first().unwrap().movie_time, eq(&cs.movie_time));
}

#[db_test]
async fn test_read_one_nonexistent(context: &DbTestContext) {
    let response = context
        .app
        .request("/movies/definitely-missing-slug-999999")
        .method(Method::GET)
        .send()
        .await;

    assert_that!(response.status(), eq(StatusCode::NOT_FOUND));
}

#[db_test]
async fn test_read_one_success(context: &DbTestContext) {
    let cs = sample_movie_cs();
    let movie = create_movie(cs.clone(), &context.db_pool).await.unwrap();
    let slug = movie.slug.clone();

    let response = context
        .app
        .request(format!("/movies/{slug}").as_str())
        .method(Method::GET)
        .send()
        .await;

    assert_that!(response.status(), eq(StatusCode::OK));

    let got: Movie = response.into_body().into_json::<Movie>().await;
    assert_that!(got.slug, eq(&slug));
    assert_that!(got.title, eq(&cs.title));
    assert_that!(got.movie_time, eq(&cs.movie_time));
}

#[db_test]
async fn test_update_unauthorized(context: &DbTestContext) {
    let movie = create_movie(sample_movie_cs(), &context.db_pool)
        .await
        .unwrap();

    let response = context
        .app
        .request(format!("/movies/{}", movie.slug).as_str())
        .method(Method::PUT)
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            r#"{"title":"x","movie_time":"in 5 min","rows":1,"seats_per_rows":1}"#,
        ))
        .send()
        .await;

    assert_that!(response.status(), eq(StatusCode::UNAUTHORIZED));
}

#[db_test]
async fn test_update_forbidden(context: &DbTestContext) {
    let movie = create_movie(sample_movie_cs(), &context.db_pool)
        .await
        .unwrap();

    let response = context
        .app
        .request(format!("/movies/{}", movie.slug).as_str())
        .method(Method::PUT)
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            r#"{"title":"x","movie_time":"in 5 min","rows":1,"seats_per_rows":1}"#,
        ))
        .header(http::header::AUTHORIZATION, TEST_USER_AUTH)
        .send()
        .await;

    assert_that!(response.status(), eq(StatusCode::FORBIDDEN));
}

#[db_test]
async fn test_update_invalid(context: &DbTestContext) {
    let cs = sample_movie_cs();
    let movie = create_movie(cs.clone(), &context.db_pool).await.unwrap();

    let payload = json!({
        "title": "",
        "movie_time": "in 5 min",
        "rows": 1,
        "seats_per_rows": 1,
    });

    let response = context
        .app
        .request(format!("/movies/{}", movie.slug).as_str())
        .method(Method::PUT)
        .body(Body::from(payload.to_string()))
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, TEST_ADMIN_AUTH)
        .send()
        .await;

    assert_that!(response.status(), eq(StatusCode::UNPROCESSABLE_ENTITY));

    let after = load_movie(&movie.slug, &context.db_pool).await.unwrap();
    assert_that!(after.title, eq(&cs.title));
}

#[db_test]
async fn test_update_nonexistent(context: &DbTestContext) {
    let cs = MovieChangeset {
        title: "Nope".into(),
        movie_time: "in 5 min".into(),
        row_count: 1,
        seats_per_row: 1,
    };
    let payload = json!({
        "title": cs.title,
        "movie_time": cs.movie_time,
        "rows": cs.row_count,
        "seats_per_rows": cs.seats_per_row,
    });

    let response = context
        .app
        .request("/movies/definitely-missing-slug-999999")
        .method(Method::PUT)
        .body(Body::from(payload.to_string()))
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, TEST_ADMIN_AUTH)
        .send()
        .await;

    assert_that!(response.status(), eq(StatusCode::NOT_FOUND));
}

#[db_test]
async fn test_update_success(context: &DbTestContext) {
    let movie = create_movie(sample_movie_cs(), &context.db_pool)
        .await
        .unwrap();
    let next = MovieChangeset {
        title: "Updated title".into(),
        movie_time: "in 9 min".into(),
        row_count: 20,
        seats_per_row: 24,
    };
    let payload = json!({
        "title": next.title,
        "movie_time": next.movie_time,
        "rows": next.row_count,
        "seats_per_rows": next.seats_per_row,
    });

    let response = context
        .app
        .request(format!("/movies/{}", movie.slug).as_str())
        .method(Method::PUT)
        .body(Body::from(payload.to_string()))
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, TEST_ADMIN_AUTH)
        .send()
        .await;

    assert_that!(response.status(), eq(StatusCode::OK));

    let got: Movie = response.into_body().into_json::<Movie>().await;
    let expected_slug = format!("updated-title-{}", movie.id);
    assert_that!(&got.slug, eq(&expected_slug));
    assert_that!(got.title, eq(&next.title));
    assert_that!(got.movie_time, eq(&next.movie_time));
    assert_that!(got.row_count, eq(next.row_count));
    assert_that!(got.seats_per_row, eq(next.seats_per_row));
}

#[db_test]
async fn test_delete_unauthorized(context: &DbTestContext) {
    let movie = create_movie(sample_movie_cs(), &context.db_pool)
        .await
        .unwrap();

    let response = context
        .app
        .request(format!("/movies/{}", movie.slug).as_str())
        .method(Method::DELETE)
        .send()
        .await;

    assert_that!(response.status(), eq(StatusCode::UNAUTHORIZED));
}

#[db_test]
async fn test_delete_forbidden(context: &DbTestContext) {
    let movie = create_movie(sample_movie_cs(), &context.db_pool)
        .await
        .unwrap();

    let response = context
        .app
        .request(format!("/movies/{}", movie.slug).as_str())
        .method(Method::DELETE)
        .header(http::header::AUTHORIZATION, TEST_USER_AUTH)
        .send()
        .await;

    assert_that!(response.status(), eq(StatusCode::FORBIDDEN));
}

#[db_test]
async fn test_delete_success(context: &DbTestContext) {
    let movie = create_movie(sample_movie_cs(), &context.db_pool)
        .await
        .unwrap();
    let slug = movie.slug.clone();

    let response = context
        .app
        .request(format!("/movies/{slug}").as_str())
        .method(Method::DELETE)
        .header(http::header::AUTHORIZATION, TEST_ADMIN_AUTH)
        .send()
        .await;

    assert_that!(response.status(), eq(StatusCode::NO_CONTENT));

    assert!(matches!(
        load_movie(&slug, &context.db_pool).await,
        Err(cinema_booking_db::Error::NoRecordFound)
    ));
}

#[db_test]
async fn test_delete_nonexistent(context: &DbTestContext) {
    let response = context
        .app
        .request("/movies/definitely-missing-slug-999999")
        .method(Method::DELETE)
        .header(http::header::AUTHORIZATION, TEST_ADMIN_AUTH)
        .send()
        .await;

    assert_that!(response.status(), eq(StatusCode::NOT_FOUND));
}
