use cinema_booking_db::entities::bookings::{self, BookingChangeset};
use cinema_booking_db::entities::movies::{self, MovieChangeset};
use cinema_booking_db::test_helpers::users::{create as create_user, UserChangeset};
use cinema_booking_db::Error;
use cinema_booking_macros::db_test;
use fake::{Fake, Faker};
use googletest::prelude::*;
use uuid::Uuid;

async fn seed_user_uuid(pool: &cinema_booking_db::DbPool) -> String {
    let user_changeset: UserChangeset = Faker.fake();
    create_user(user_changeset, pool)
        .await
        .expect("seed user")
        .uuid
}

async fn seed_movie_slug(pool: &cinema_booking_db::DbPool) -> String {
    let cs = MovieChangeset {
        title: format!("movie-{}", Uuid::new_v4()),
        row_count: 10,
        seats_per_row: 12,
    };
    movies::create(cs, pool).await.expect("seed movie").slug
}

#[db_test]
async fn test_create_rejects_invalid_changeset(
    context: &cinema_booking_web::test_helpers::DbTestContext,
) {
    let user_uuid = seed_user_uuid(&context.db_pool).await;
    let changeset = BookingChangeset {
        movie_slug: String::new(),
        seat_uuid: "seat-1".into(),
        user_uuid,
    };

    let result = bookings::create(changeset, &context.db_pool).await;
    assert_that!(result, err(anything()));
    assert!(matches!(result.unwrap_err(), Error::ValidationError(_)));
}

#[db_test]
async fn test_create_fails_when_user_uuid_not_in_users(
    context: &cinema_booking_web::test_helpers::DbTestContext,
) {
    let movie_slug = seed_movie_slug(&context.db_pool).await;
    let mut changeset: BookingChangeset = Faker.fake();
    changeset.movie_slug = movie_slug;
    changeset.user_uuid = Uuid::new_v4().to_string();
    let result = bookings::create(changeset, &context.db_pool).await;
    assert_that!(result, err(anything()));
    assert!(matches!(result.unwrap_err(), Error::DbError(_)));
}

#[db_test]
async fn test_load_returns_not_found(context: &cinema_booking_web::test_helpers::DbTestContext) {
    let result = bookings::load(&Uuid::new_v4().to_string(), &context.db_pool).await;
    assert!(matches!(result, Err(Error::NoRecordFound)));
}

#[db_test]
async fn test_load_by_id_returns_not_found(
    context: &cinema_booking_web::test_helpers::DbTestContext,
) {
    let result = bookings::load_by_id(9_999_999, &context.db_pool).await;
    assert!(matches!(result, Err(Error::NoRecordFound)));
}

#[db_test]
async fn test_load_all_empty(context: &cinema_booking_web::test_helpers::DbTestContext) {
    let rows = bookings::load_all(&context.db_pool).await.unwrap();
    assert_that!(rows, is_empty());
}

#[db_test]
async fn test_list_by_movie_slug_filters_rows(
    context: &cinema_booking_web::test_helpers::DbTestContext,
) {
    let user_one_uuid = seed_user_uuid(&context.db_pool).await;
    let user_two_uuid = seed_user_uuid(&context.db_pool).await;
    let movie_a = seed_movie_slug(&context.db_pool).await;
    let movie_b = seed_movie_slug(&context.db_pool).await;

    bookings::create(
        BookingChangeset {
            movie_slug: movie_a.clone(),
            seat_uuid: "seat-1".into(),
            user_uuid: user_one_uuid.clone(),
        },
        &context.db_pool,
    )
    .await
    .unwrap();

    bookings::create(
        BookingChangeset {
            movie_slug: movie_a.clone(),
            seat_uuid: "seat-2".into(),
            user_uuid: user_two_uuid.clone(),
        },
        &context.db_pool,
    )
    .await
    .unwrap();

    bookings::create(
        BookingChangeset {
            movie_slug: movie_b.clone(),
            seat_uuid: "seat-9".into(),
            user_uuid: user_one_uuid,
        },
        &context.db_pool,
    )
    .await
    .unwrap();

    let rows = bookings::list_by_movie_slug(&movie_a, &context.db_pool)
        .await
        .unwrap();
    assert_that!(rows, len(eq(2)));
    assert!(rows.iter().all(|row| row.movie_slug == movie_a));
    assert_that!(rows[0].seat_uuid, eq("seat-1"));
    assert_that!(rows[1].seat_uuid, eq("seat-2"));
}

#[db_test]
async fn test_create_load_update_delete_round_trip(
    context: &cinema_booking_web::test_helpers::DbTestContext,
) {
    let user_uuid = seed_user_uuid(&context.db_pool).await;
    let movie_first = seed_movie_slug(&context.db_pool).await;
    let movie_second = seed_movie_slug(&context.db_pool).await;
    let mut changeset: BookingChangeset = Faker.fake();
    changeset.user_uuid = user_uuid.clone();
    changeset.movie_slug = movie_first.clone();
    changeset.seat_uuid = "seat-roundtrip-1".into();

    let created = bookings::create(changeset.clone(), &context.db_pool)
        .await
        .unwrap();
    assert_that!(created.movie_slug, eq(&changeset.movie_slug));
    assert_that!(created.seat_uuid, eq(&changeset.seat_uuid));
    assert_that!(created.user_uuid, eq(&user_uuid));

    let by_uuid = bookings::load(&created.uuid, &context.db_pool)
        .await
        .unwrap();
    assert_that!(by_uuid.id, eq(created.id));
    assert_that!(by_uuid.uuid, eq(&created.uuid));

    let by_id = bookings::load_by_id(created.id, &context.db_pool)
        .await
        .unwrap();
    assert_that!(by_id.uuid, eq(&created.uuid));

    let all = bookings::load_all(&context.db_pool).await.unwrap();
    assert_that!(all, len(eq(1)));

    let update_cs = BookingChangeset {
        movie_slug: movie_second.clone(),
        seat_uuid: "new-seat".into(),
        user_uuid: user_uuid.clone(),
    };
    let updated = bookings::update(&created.uuid, update_cs.clone(), &context.db_pool)
        .await
        .unwrap();
    assert_that!(updated.movie_slug, eq(&update_cs.movie_slug));
    assert_that!(updated.seat_uuid, eq(&update_cs.seat_uuid));
    assert_that!(updated.user_uuid, eq(&user_uuid));

    bookings::delete(&created.uuid, &context.db_pool)
        .await
        .unwrap();

    assert!(matches!(
        bookings::load(&created.uuid, &context.db_pool).await,
        Err(Error::NoRecordFound)
    ));
}

#[db_test]
async fn test_update_not_found(context: &cinema_booking_web::test_helpers::DbTestContext) {
    let user_uuid = seed_user_uuid(&context.db_pool).await;
    let mut changeset: BookingChangeset = Faker.fake();
    changeset.user_uuid = user_uuid;

    let result = bookings::update(&Uuid::new_v4().to_string(), changeset, &context.db_pool).await;
    assert!(matches!(result, Err(Error::NoRecordFound)));
}

#[db_test]
async fn test_delete_not_found(context: &cinema_booking_web::test_helpers::DbTestContext) {
    let result = bookings::delete(&Uuid::new_v4().to_string(), &context.db_pool).await;
    assert!(matches!(result, Err(Error::NoRecordFound)));
}
