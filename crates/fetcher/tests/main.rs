use chrono::{TimeZone, Utc};
use serial_test::serial;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use common::items::{get_items_of_user, insert_items, insert_items_delta_for_all_registered_users};
use common::model::NewItem;
use common::{init_redis_connection, Pool};
use fetcher::process;

#[sqlx::test(migrations = "../../migrations")]
#[serial]
async fn test_delta_is_loaded(pool: Pool) {
    let mock = MockServer::start().await;
    build_and_link_channel(&mock.uri(), &pool).await;
    let redis_pool = init_redis_connection();

    // Prepare the web server
    let bytes = include_bytes!("feed.xml").to_vec();
    let response = ResponseTemplate::new(200).set_body_raw(bytes, "application/xml");
    Mock::given(method("GET"))
        .and(path("/coucou"))
        .respond_with(response)
        .expect(1)
        .mount(&mock)
        .await;

    // First, let's load some bogus articles for the chan
    let items: Vec<NewItem> = (1..=5)
        .map(|id| NewItem {
            guid: Some(id.to_string()),
            title: Some(format!("Article {id}")),
            url: Some("url".to_owned()),
            content: Some("content".to_owned()),
            fetch_timestamp: Utc.with_ymd_and_hms(2000, 1, 1, 0, 0, 0).unwrap(),
            publish_timestamp: Some(Utc.with_ymd_and_hms(2000, 1, 1, 0, 0, 0).unwrap()),
            channel_id: 1,
        })
        .collect();

    let ids = insert_items(&pool, &items).await.unwrap();
    assert_eq!(vec![1, 2, 3, 4, 5], ids); // Let's hope my ids are predictable here. They should.

    // Link them to the user
    insert_items_delta_for_all_registered_users(&pool, 1, &Utc::now())
        .await
        .unwrap();

    // User 1 should now have 5 articles
    let items = get_items_of_user(&pool, Some(1), None, None, 1, 1, 200)
        .await
        .unwrap();
    assert_eq!(5, *items.total_items(), "5 items should be inserted");

    // Ok now let's fetch some new items
    process(&pool, &redis_pool).await.unwrap();

    // User 1 should now have 65 articles
    let items = get_items_of_user(&pool, Some(1), None, None, 1, 1, 200)
        .await
        .unwrap();
    assert_eq!(
        65,
        *items.total_items(),
        "60 items should be inserted  + 5 previous"
    );
}

#[sqlx::test(migrations = "../../migrations")]
#[serial]
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

    let items_count = sqlx::query_scalar!(
        r#"
        SELECT count(*) FROM items WHERE channel_id=1;
        "#
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .unwrap();

    assert_eq!(0, items_count, "items count is still 0");
}

#[sqlx::test(migrations = "../../migrations")]
#[serial]
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
