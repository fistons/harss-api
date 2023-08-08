use chrono::{TimeZone, Utc};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use common::{init_redis_connection, Pool};
use fetcher::process;

#[sqlx::test(migrations = "../../migrations")]
async fn test_errors_are_filled(pool: Pool) {
    let mock = MockServer::start().await;
    build_and_link_channel(&mock.uri(), &pool).await;
    let redis_pool = init_redis_connection();

    Mock::given(method("GET"))
        .and(path("/coucou"))
        .respond_with(ResponseTemplate::new(500))
        .expect(1)
        .mount(&mock)
        .await;

    process(&pool, &redis_pool).await.unwrap();

    let errors = sqlx::query!(
        r#"
        SELECT * FROM channels_errors;
        "#
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(1, errors.len(), "An error should have been inserted");
    assert_eq!(
        1, errors[0].channel_id,
        "The error should concerne channel_id 1"
    );
    assert_eq!(1, errors[0].id, "ID of the error should be 1");
    assert_eq!(
        Some("HTTP status code error: Upstream feed returned HTTP status code 500".to_owned()),
        errors[0].error_reason,
        "Error reason should match"
    );

    let channel = sqlx::query!(
        r#"
        SELECT failure_count FROM channels WHERE id=1;
        "#
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(1, channel.failure_count, "Failure count has increased");
}

#[sqlx::test(migrations = "../../migrations")]
async fn happy_path(pool: Pool) {
    // Create DB and webserver
    let mock = MockServer::start().await;
    let redis_pool = init_redis_connection();

    build_and_link_channel(&mock.uri(), &pool).await;

    // Prepare the web server
    let bytes = include_bytes!("feed.xml").to_vec();
    let response = ResponseTemplate::new(200).set_body_raw(bytes, "application/xml");
    Mock::given(method("GET"))
        .and(path("/coucou"))
        .respond_with(response)
        .expect(1)
        .mount(&mock)
        .await;

    // "Fetch" stuff
    process(&pool, &redis_pool).await.unwrap();

    // Check that stuff have been inserted
    let inserted_items = sqlx::query!(
        r#"
        SELECT * from items ORDER BY fetch_timestamp DESC, id desc
        "#
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(
        inserted_items.len(),
        60,
        "60 newly inserted items should be found"
    );

    assert_eq!(
        inserted_items[0].guid,
        Some("https://www.canardpc.com/?post_type=news&p=43057".into()),
        "Item GUID should match"
    );

    assert_eq!(
        inserted_items[0].url,
        Some("https://www.canardpc.com/news/mine-dor/".into()),
        "URL should match"
    );

    assert_eq!(
        inserted_items[0].guid,
        Some("https://www.canardpc.com/?post_type=news&p=43057".into()),
        "Item GUID should match"
    );

    assert_eq!(
        inserted_items[0].title,
        Some("Mine d’or".into()),
        "Title should match"
    );

    assert_eq!(
        inserted_items[0].publish_timestamp,
        Some(Utc.with_ymd_and_hms(2022, 8, 3, 12, 0, 17).unwrap()),
        "Publish timestamp should match"
    );

    // Check that the link with the user is done
    let inserted_user_items = sqlx::query!(
        r#"
        SELECT * from users_items
    "#
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        inserted_user_items.len(),
        60,
        "60 newly inserted items should be found"
    );
}

async fn build_and_link_channel(host: &str, pool: &Pool) {
    sqlx::query!(
        r#"
        INSERT INTO channels
        (id, name, url, registration_timestamp, last_update, disabled, failure_count)
        VALUES (1, 'Dummy', $1, now(), now(), false, 0)
    "#,
        format!("{}/coucou", host)
    )
    .execute(pool)
    .await
    .unwrap();

    sqlx::query!(
        r#"
        INSERT INTO channel_users
        (channel_id, user_id)
        VALUES (1, 1)
    "#
    )
    .execute(pool)
    .await
    .unwrap();
}
