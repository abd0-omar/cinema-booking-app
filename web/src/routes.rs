use crate::controllers::{bookings, movies, tasks};
use crate::middlewares::auth::auth;
use crate::state::AppState;
use crate::views;
use axum::{
    middleware,
    routing::{delete, get, post, put},
    Router,
};
use std::sync::Arc;

/// Initializes the application's routes.
///
/// This function maps paths (e.g. "/greet") and HTTP methods (e.g. "GET") to functions in [`crate::controllers`] as well as includes middlewares defined in [`crate::middlewares`] into the routing layer (see [`axum::Router`]).
pub fn init_routes(app_state: AppState) -> Router {
    let shared_app_state = Arc::new(app_state);
    let public = Router::new()
        .route("/", get(views::cinema_index))
        .route("/login", get(views::login_get).post(views::login_post))
        .route("/logout", post(views::logout_post))
        .route("/signup", get(views::signup_get).post(views::signup_post))
        .route(
            "/ds/hello-world",
            get(views::ds_hello_world).post(views::ds_hello_world),
        );
    let api = Router::new()
        .route("/tasks", post(tasks::create))
        .route("/tasks", put(tasks::create_batch))
        .route("/tasks/{id}", delete(tasks::delete))
        .route("/tasks/{id}", put(tasks::update))
        .route("/movies", post(movies::create))
        .route("/movies/{slug}", delete(movies::delete))
        .route("/movies/{slug}", put(movies::update))
        .route("/bookings/hold", post(bookings::hold))
        .route("/bookings/checkout", post(bookings::checkout))
        .route(
            "/bookings/movies/{movie_slug}",
            get(bookings::list_by_movie),
        )
        .route_layer(middleware::from_fn_with_state(
            shared_app_state.clone(),
            auth,
        ))
        .route("/tasks", get(tasks::read_all))
        .route("/tasks/{id}", get(tasks::read_one))
        .route("/movies", get(movies::read_all))
        .route("/movies/{slug}", get(movies::read_one));
    public.merge(api).with_state(shared_app_state)
}
