use chrono::Utc;
use fake::Fake;
use sea_orm::sea_query::TableCreateStatement;
use sea_orm::{
    ActiveModelTrait, ConnectionTrait, Database, DatabaseConnection, DbBackend, NotSet, Schema,
    Set, TransactionTrait,
};
use wiremock::MockServer;

use entity::channel_users::Entity as ChannelUsers;
use entity::channels;
use entity::channels::Entity as Channels;
use entity::items;
use entity::items::Entity as Items;
use entity::users::Entity as Users;
use entity::users_items::Entity as UserItems;

pub async fn build_mock() -> MockServer {
    MockServer::start().await
}

pub async fn configure_database(host: String) -> DatabaseConnection {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    let schema = Schema::new(DbBackend::Sqlite);

    let tables_stmt: TableCreateStatement = schema.create_table_from_entity(Channels);
    let users_stmt: TableCreateStatement = schema.create_table_from_entity(Users);
    let items_stmt: TableCreateStatement = schema.create_table_from_entity(Items);
    let channel_users_stmt: TableCreateStatement = schema.create_table_from_entity(ChannelUsers);
    let user_items_stmt: TableCreateStatement = schema.create_table_from_entity(UserItems);

    let txn = db.begin().await.unwrap();
    txn.execute(db.get_database_backend().build(&tables_stmt))
        .await
        .unwrap();

    txn.execute(db.get_database_backend().build(&users_stmt))
        .await
        .unwrap();

    txn.execute(db.get_database_backend().build(&items_stmt))
        .await
        .unwrap();

    txn.execute(db.get_database_backend().build(&channel_users_stmt))
        .await
        .unwrap();

    txn.execute(db.get_database_backend().build(&user_items_stmt))
        .await
        .unwrap();

    for channel in channels_fixture(&host) {
        let channel = channel.insert(&txn).await.unwrap();
        dbg!(&channel);
        for item in items_fixtures(channel.id) {
            dbg!(&item);
            item.insert(&txn).await.unwrap();
        }
    }

    txn.commit().await.unwrap();

    db
}

fn channels_fixture(host: &str) -> Vec<channels::ActiveModel> {
    vec![channels::ActiveModel {
        id: Set(1),
        name: Set("Dummy One".to_owned()),
        url: Set(format!("{}/coucou", host)),
        last_update: NotSet,
        registration_timestamp: Set(Utc::now().into()),
        disabled: Set(false),
        failure_count: Set(0),
    }]
}

fn items_fixtures(chan_id: i32) -> Vec<items::ActiveModel> {
    vec![items::ActiveModel {
        id: Set(chan_id),
        guid: Set(optional_fake_sentence()),
        title: Set(optional_fake_sentence()),
        url: Set(optional_fake_url()),
        content: Set(optional_fake_sentence()),
        fetch_timestamp: Set(Utc::now().into()),
        publish_timestamp: Set(None),
        channel_id: Set(chan_id),
    }]
}

fn optional_fake_sentence() -> Option<String> {
    Some(fake::faker::lorem::en::Sentence(1..10).fake())
}

fn optional_fake_url() -> Option<String> {
    Some(fake::faker::internet::en::DomainSuffix().fake())
}
