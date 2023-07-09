use chrono::Utc;
use fake::Fake;
use sea_orm::sea_query::TableCreateStatement;
use sea_orm::{
    ActiveModelTrait, ConnectionTrait, Database, DatabaseConnection, DbBackend, NotSet, Schema,
    Set, TransactionTrait,
};

use entity::channel_users::Entity as ChannelUsers;
use entity::channels;
use entity::channels::Entity as Channels;
use entity::items;
use entity::items::Entity as Items;
use entity::sea_orm_active_enums::UserRole;
use entity::users;
use entity::users::Entity as Users;
use entity::users_items::Entity as UserItems;
use entity::channels_errors::Entity as ChannelsError;

pub async fn configure_database(host: String) -> DatabaseConnection {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    let schema = Schema::new(DbBackend::Sqlite);

    let tables_stmt: TableCreateStatement = schema.create_table_from_entity(Channels);
    let users_stmt: TableCreateStatement = schema.create_table_from_entity(Users);
    let items_stmt: TableCreateStatement = schema.create_table_from_entity(Items);
    let channel_users_stmt: TableCreateStatement = schema.create_table_from_entity(ChannelUsers);
    let user_items_stmt: TableCreateStatement = schema.create_table_from_entity(UserItems);
    let channel_errors_stmt: TableCreateStatement = schema.create_table_from_entity(ChannelsError);

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

    txn.execute(db.get_database_backend().build(&channel_errors_stmt))
        .await
        .unwrap();

    for user in user_fixture() {
        user.insert(&txn).await.unwrap();
    }

    for channel in channels_fixture(&host) {
        let channel = channel.insert(&txn).await.unwrap();

        entity::channel_users::ActiveModel {
            channel_id: Set(channel.id),
            user_id: Set(1),
            registration_timestamp: Set(Utc::now().into()),
        }
        .insert(&txn)
        .await
        .unwrap();

        for item in items_fixtures(channel.id) {
            item.insert(&txn).await.unwrap();
        }
        build_user_channels(1, 1, 1).insert(&txn).await.unwrap();
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
        id: Set(1),
        guid: Set(Some("https://canard.com/i-dont-exist".into())),
        title: Set(optional_fake_sentence()),
        url: Set(optional_fake_url()),
        content: Set(optional_fake_sentence()),
        fetch_timestamp: Set(Utc::now().into()),
        publish_timestamp: Set(None),
        channel_id: Set(chan_id),
    }]
}

fn user_fixture() -> Vec<users::ActiveModel> {
    vec![users::ActiveModel {
        id: Set(1),
        username: Set(fake::faker::lorem::en::Word().fake()),
        password: Set(fake::faker::lorem::en::Word().fake()),
        role: Set(UserRole::Basic),
    }]
}

fn build_user_channels(
    user_id: i32,
    chan_id: i32,
    item_id: i32,
) -> entity::users_items::ActiveModel {
    entity::users_items::ActiveModel {
        user_id: Set(user_id),
        channel_id: Set(chan_id),
        item_id: Set(item_id),
        read: Set(false),
        starred: Set(false),
    }
}

fn optional_fake_sentence() -> Option<String> {
    Some(fake::faker::lorem::en::Sentence(1..10).fake())
}

fn optional_fake_url() -> Option<String> {
    Some(fake::faker::internet::en::DomainSuffix().fake())
}
