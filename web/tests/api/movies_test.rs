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

const TEST_AUTH: &str = "Bearer test-bearer-token";

type MoviesList = Vec<Movie>;

fn sample_movie_cs() -> MovieChangeset {
    MovieChangeset {
        title: format!("Movie {}", Uuid::new_v4()),
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
async fn test_create_invalid(context: &DbTestContext) {
    let payload = json!({
        "title": "",
        "rows": 1,
        "seats_per_rows": 1,
    });

    let response = context
        .app
        .request("/movies")
        .method(Method::POST)
        .body(Body::from(payload.to_string()))
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, TEST_AUTH)
        .send()
        .await;

    assert_that!(response.status(), eq(StatusCode::UNPROCESSABLE_ENTITY));
}

#[db_test]
async fn test_create_success(context: &DbTestContext) {
    let cs = MovieChangeset {
        title: "Integration Movie".into(),
        row_count: 10,
        seats_per_row: 12,
    };
    let payload = json!({
        "title": cs.title,
        "rows": cs.row_count,
        "seats_per_rows": cs.seats_per_row,
    });

    let response = context
        .app
        .request("/movies")
        .method(Method::POST)
        .body(Body::from(payload.to_string()))
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, TEST_AUTH)
        .send()
        .await;

    assert_that!(response.status(), eq(StatusCode::CREATED));

    let movies = load_movies(&context.db_pool).await.unwrap();
    assert_that!(movies, len(eq(1)));
    assert_that!(movies.first().unwrap().title, eq(&cs.title));
    assert_that!(movies.first().unwrap().row_count, eq(cs.row_count));
    assert_that!(movies.first().unwrap().seats_per_row, eq(cs.seats_per_row));
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
}

#[db_test]
async fn test_read_one_nonexistent(context: &DbTestContext) {
    let response = context
        .app
        .request(format!("/movies/{}", Uuid::new_v4()).as_str())
        .method(Method::GET)
        .send()
        .await;

    assert_that!(response.status(), eq(StatusCode::NOT_FOUND));
}

#[db_test]
async fn test_read_one_success(context: &DbTestContext) {
    let cs = sample_movie_cs();
    let movie = create_movie(cs.clone(), &context.db_pool).await.unwrap();
    let uuid = movie.uuid.clone();

    let response = context
        .app
        .request(format!("/movies/{uuid}").as_str())
        .method(Method::GET)
        .send()
        .await;

    assert_that!(response.status(), eq(StatusCode::OK));

    let got: Movie = response.into_body().into_json::<Movie>().await;
    assert_that!(got.uuid, eq(&uuid));
    assert_that!(got.title, eq(&cs.title));
}

#[db_test]
async fn test_update_unauthorized(context: &DbTestContext) {
    let movie = create_movie(sample_movie_cs(), &context.db_pool)
        .await
        .unwrap();

    let response = context
        .app
        .request(format!("/movies/{}", movie.uuid).as_str())
        .method(Method::PUT)
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"title":"x","rows":1,"seats_per_rows":1}"#))
        .send()
        .await;

    assert_that!(response.status(), eq(StatusCode::UNAUTHORIZED));
}

#[db_test]
async fn test_update_invalid(context: &DbTestContext) {
    let cs = sample_movie_cs();
    let movie = create_movie(cs.clone(), &context.db_pool).await.unwrap();

    let payload = json!({
        "title": "",
        "rows": 1,
        "seats_per_rows": 1,
    });

    let response = context
        .app
        .request(format!("/movies/{}", movie.uuid).as_str())
        .method(Method::PUT)
        .body(Body::from(payload.to_string()))
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, TEST_AUTH)
        .send()
        .await;

    assert_that!(response.status(), eq(StatusCode::UNPROCESSABLE_ENTITY));

    let after = load_movie(&movie.uuid, &context.db_pool).await.unwrap();
    assert_that!(after.title, eq(&cs.title));
}

#[db_test]
async fn test_update_nonexistent(context: &DbTestContext) {
    let cs = MovieChangeset {
        title: "Nope".into(),
        row_count: 1,
        seats_per_row: 1,
    };
    let payload = json!({
        "title": cs.title,
        "rows": cs.row_count,
        "seats_per_rows": cs.seats_per_row,
    });

    let response = context
        .app
        .request(format!("/movies/{}", Uuid::new_v4()).as_str())
        .method(Method::PUT)
        .body(Body::from(payload.to_string()))
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, TEST_AUTH)
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
        row_count: 20,
        seats_per_row: 24,
    };
    let payload = json!({
        "title": next.title,
        "rows": next.row_count,
        "seats_per_rows": next.seats_per_row,
    });

    let response = context
        .app
        .request(format!("/movies/{}", movie.uuid).as_str())
        .method(Method::PUT)
        .body(Body::from(payload.to_string()))
        .header(http::header::CONTENT_TYPE, "application/json")
        .header(http::header::AUTHORIZATION, TEST_AUTH)
        .send()
        .await;

    assert_that!(response.status(), eq(StatusCode::OK));

    let got: Movie = response.into_body().into_json::<Movie>().await;
    assert_that!(got.title, eq(&next.title));
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
        .request(format!("/movies/{}", movie.uuid).as_str())
        .method(Method::DELETE)
        .send()
        .await;

    assert_that!(response.status(), eq(StatusCode::UNAUTHORIZED));
}

#[db_test]
async fn test_delete_success(context: &DbTestContext) {
    let movie = create_movie(sample_movie_cs(), &context.db_pool)
        .await
        .unwrap();
    let uuid = movie.uuid.clone();

    let response = context
        .app
        .request(format!("/movies/{uuid}").as_str())
        .method(Method::DELETE)
        .header(http::header::AUTHORIZATION, TEST_AUTH)
        .send()
        .await;

    assert_that!(response.status(), eq(StatusCode::NO_CONTENT));

    assert!(matches!(
        load_movie(&uuid, &context.db_pool).await,
        Err(cinema_booking_db::Error::NoRecordFound)
    ));
}

#[db_test]
async fn test_delete_nonexistent(context: &DbTestContext) {
    let response = context
        .app
        .request(format!("/movies/{}", Uuid::new_v4()).as_str())
        .method(Method::DELETE)
        .header(http::header::AUTHORIZATION, TEST_AUTH)
        .send()
        .await;

    assert_that!(response.status(), eq(StatusCode::NOT_FOUND));
}
