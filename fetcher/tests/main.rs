use chrono::prelude::*;
use reqwest::Client;
use sea_orm::{entity::*, query::*};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use entity::items::Entity as Item;
use entity::users_items::Entity as UserItem;
use helpers::configure_database;

mod helpers;

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
    fetcher::Fetcher::new(Client::default(), db.clone())
        .fetch()
        .await
        .unwrap();

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
        Some(Utc.ymd(2022, 8, 3).and_hms(12, 0, 17).into()),
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
