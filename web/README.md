# cinema-booking-web

This crate implements the application's web interface. It contains controllers and middleware and is responsible for booting up the application which includes setting up tracing.

## Application state

The code for defining the application state and creating a fresh state when the application boots is in _[src/state.rs](./src/state.rs)_. The state holds a database pool and an [`AccessTokenVerifier`](../../crates/auth/src/lib.rs) (Trailbase JWT in production; fixed token in `test-helpers` + test env).

## Routing

Routes are defined in _[src/routes.rs](./src/routes.rs)_, e.g.:

```rust
pub fn init_routes(app_state: AppState) -> Router {
    Router::new()
        .route("/tasks", post(create_task))
        .route("/tasks", get(get_tasks))
        .route("/tasks/{id}", get(get_task))
}
```

## Controllers and Middlewares

Controllers and middlewares are kept in the respectively named directories. Controllers export axum request handlers. Middlewares are standard Tower middlewares.

## Tests

Gerust follows a full stack testing approach. The application's endpoint including database access are tested via tests in the `web` crate. Using Gerust's test macros, tests receive a fully configured and booted up instance of the application that requests can be made against.In order to allow requests to access the database without the risk of different tests interfering with each other, each test uses its own dedicated database. A pool of connections is passed to the test via the test context and the application instance is preconfigured to use the same database:


```rust
#[db_test]
async fn test_read_all(context: &DbTestContext) {
    let task_changeset: TaskChangeset = Faker.fake();
    create_task(task_changeset.clone(), &context.db_pool)
        .await
        .unwrap();

    let response = context
        .app
        .request("/tasks")
        .method(Method::GET)
        .send()
        .await;

    assert_that!(response.status(), eq(StatusCode::OK));

    let tasks: TasksList = response.into_body().into_json::<TasksList>().await;
    assert_that!(tasks, len(eq(1)));
    assert_that!(
        tasks.first().unwrap().description,
        eq(task_changeset.description)
    );
}
```
### Test helpers

The cinema-booking-web crate includes test helpers in _[src/test_helpers](./src/test_helpers/)_ that add a number of convience functions for easier issuing of requests and parsing of responses. Those helpers depend on the `test-helpers` feature flag which is automatically enabled when running tests but not for production builds. 

> [!INFO]
>
> You should not need to make any changes to these helpers.
