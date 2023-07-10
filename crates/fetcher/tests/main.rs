use chrono::prelude::*;
use sea_orm::{entity::*, query::*};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use entity::channels::Entity as Channel;
use entity::channels_errors::Entity as ChannelsError;
use entity::items::Entity as Item;
use entity::users_items::Entity as UserItem;
use fetcher::fetch;
use helpers::configure_database;

mod helpers;

#[tokio::test]
async fn test_error_is_filled() {
    let mock = MockServer::start().await;
    let db = configure_database(mock.uri()).await;

    Mock::given(method("GET"))
        .and(path("/coucou"))
        .respond_with(ResponseTemplate::new(500))
        .expect(1)
        .mount(&mock)
        .await;

    fetch(&db).await.unwrap();

    let errors = ChannelsError::find().all(&db).await.unwrap();

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

    let channel = Channel::find()
        .filter(entity::channels::Column::Id.eq(1))
        .one(&db)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(1, channel.failure_count, "Failure count has increased");
}

#[tokio::test]
async fn happy_path() {
    // Create DB and webserver
    let mock = MockServer::start().await;
    let db = configure_database(mock.uri()).await;

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
    fetch(&db).await.unwrap();

    // Check that stuff have been inserted
    let inserted_items = Item::find()
        .order_by_desc(entity::items::Column::FetchTimestamp)
        .all(&db)
        .await
        .unwrap();

    assert_eq!(
        inserted_items.len(),
        61,
        "1 pre-existing + 60 newly inserted items should be found"
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
        Some(Utc.with_ymd_and_hms(2022, 8, 3, 12, 0, 17).unwrap().into()),
        "Publish timestamp should match"
    );

    // Check that the link with the user is done
    let inserted_user_items = UserItem::find().all(&db).await.unwrap();
    assert_eq!(
        inserted_user_items.len(),
        61,
        "1 pre-existing + 60 newly inserted items should be found"
    );
}
