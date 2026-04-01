use crate::{error::Error, state::SharedAppState};
use axum::{extract::Path, extract::State, http::StatusCode, Json};
use cinema_booking_db::entities::movies;
use tracing::info;

/// Creates a movie in the database.
#[axum::debug_handler]
pub async fn create(
    State(app_state): State<SharedAppState>,
    Json(movie): Json<movies::MovieChangeset>,
) -> Result<(StatusCode, Json<movies::Movie>), Error> {
    Ok(movies::create(movie, &app_state.db_pool)
        .await
        .map(|movie| (StatusCode::CREATED, Json(movie)))?)
}

/// Reads all movies.
#[axum::debug_handler]
pub async fn read_all(
    State(app_state): State<SharedAppState>,
) -> Result<Json<Vec<movies::Movie>>, Error> {
    let list = movies::load_all(&app_state.db_pool).await?;
    info!("responding with {} movies", list.len());
    Ok(Json(list))
}

/// Reads one movie by slug (`{slugified-title}-{id}`).
#[axum::debug_handler]
pub async fn read_one(
    State(app_state): State<SharedAppState>,
    Path(slug): Path<String>,
) -> Result<Json<movies::Movie>, Error> {
    let movie = movies::load(&slug, &app_state.db_pool).await?;
    Ok(Json(movie))
}

/// Updates a movie by slug.
#[axum::debug_handler]
pub async fn update(
    State(app_state): State<SharedAppState>,
    Path(slug): Path<String>,
    Json(movie): Json<movies::MovieChangeset>,
) -> Result<Json<movies::Movie>, Error> {
    let movie = movies::update(&slug, movie, &app_state.db_pool).await?;
    Ok(Json(movie))
}

/// Deletes a movie by slug.
#[axum::debug_handler]
pub async fn delete(
    State(app_state): State<SharedAppState>,
    Path(slug): Path<String>,
) -> Result<StatusCode, Error> {
    movies::delete(&slug, &app_state.db_pool).await?;
    Ok(StatusCode::NO_CONTENT)
}
