use log::debug;
use std::sync::Arc;

use crate::DbPool;

pub mod channel {
    use serde::{Deserialize, Serialize};

    use crate::schema::channels;

    #[derive(Debug, Serialize, Deserialize, Clone, Insertable)]
    #[table_name = "channels"]
    pub struct NewChannel {
        pub name: String,
        pub url: String,
        pub user_id: i32,
    }

    /// Source of articles, over da web
    #[derive(Debug, Serialize, Deserialize, Clone, Queryable)]
    pub struct Channel {
        pub id: i32,
        pub name: String,
        pub url: String,
        pub user_id: i32,
    }

    pub mod db {
        use diesel::prelude::*;

        use crate::schema::channels::dsl::*;

        use super::Channel;
        use super::NewChannel;
        use crate::DbPool;
        use std::sync::Arc;

        pub fn insert(
            new_channel: NewChannel,
            pool: Arc<DbPool>,
        ) -> Result<(), diesel::result::Error> {
            diesel::insert_into(channels)
                .values(&new_channel)
                .execute(&pool.get().unwrap())?;
            Ok(())
        }

        pub fn select_all(pool: &Arc<DbPool>) -> Result<Vec<Channel>, diesel::result::Error> {
            channels.load::<Channel>(&pool.get().unwrap())
        }

        pub fn select_by_id(
            predicate: i32,
            pool: &Arc<DbPool>,
        ) -> Result<Channel, diesel::result::Error> {
            channels
                .filter(id.eq(predicate))
                .first::<Channel>(&pool.get().unwrap())
        }
    }
}

pub mod items {
    use serde::{Deserialize, Serialize};

    use crate::schema::items;

    #[derive(Debug, Serialize, Deserialize, Clone, Insertable)]
    #[table_name = "items"]
    pub struct NewItem {
        pub guid: Option<String>,
        pub title: Option<String>,
        pub url: Option<String>,
        pub content: Option<String>,
        pub read: bool,
        pub channel_id: i32,
    }

    #[derive(Debug, Serialize, Deserialize, Clone, Queryable)]
    pub struct Item {
        pub id: i32,
        pub guid: Option<String>,
        pub title: Option<String>,
        pub url: Option<String>,
        pub content: Option<String>,
        pub read: bool,
        #[serde(skip_serializing)]
        pub channel_id: i32,
    }

    impl NewItem {
        /// Create an item to be inserted in the database, from a rss item.
        pub fn from_rss_item(item: rss::Item, channel_id: i32) -> NewItem {
            let title = item.title;
            let guid = item.guid.map(|x| x.value);
            let url = item.link;
            let content = item.description;
            let read = false;

            NewItem {
                title,
                guid,
                url,
                content,
                channel_id,
                read,
            }
        }
    }

    pub mod db {
        use std::sync::Arc;

        use diesel::prelude::*;

        use crate::schema::items::dsl::*;
        use crate::DbPool;

        use super::Item;
        use super::NewItem;

        pub fn insert(new_item: NewItem, pool: &Arc<DbPool>) -> Result<(), diesel::result::Error> {
            diesel::insert_into(items)
                .values(&new_item)
                .execute(&pool.get().unwrap())?;
            Ok(())
        }

        pub fn get_items_of_channel(
            chan_id: i32,
            pool: &Arc<DbPool>,
        ) -> Result<Vec<Item>, diesel::result::Error> {
            items
                .filter(channel_id.eq(chan_id))
                .load::<Item>(&pool.get().unwrap())
        }
    }
}

pub fn refresh(pool: &Arc<DbPool>) -> Result<(), diesel::result::Error> {
    let channels = channel::db::select_all(pool)?;

    for channel in channels.iter() {
        refresh_chan(pool, channel.id)?;
    }
    Ok(())
}

pub fn refresh_chan(pool: &Arc<DbPool>, channel_id: i32) -> Result<(), diesel::result::Error> {
    let channel = channel::db::select_by_id(channel_id, pool)?;
    debug!("Fetching {}", &channel.name);

    let content = reqwest::blocking::get(&channel.url)
        .unwrap()
        .bytes()
        .unwrap();
    let rss_channel = rss::Channel::read_from(&content[..]).unwrap();
    for item in rss_channel.items.into_iter() {
        let i = items::NewItem::from_rss_item(item, channel.id);
        items::db::insert(i, pool).unwrap();
    }

    Ok(())
}
