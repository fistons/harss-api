pub mod channel {
    use serde::{Deserialize, Serialize};

    use crate::schema::channels;

    #[derive(Debug, Serialize, Deserialize, Clone, Insertable)]
    #[table_name = "channels"]
    pub struct NewChannel {
        pub name: String,
        pub url: String,
    }

    /// Source of articles, over da web
    #[derive(Debug, Serialize, Deserialize, Clone, Queryable)]
    pub struct Channel {
        pub id: i32,
        pub name: String,
        pub url: String,
    }

    pub mod db {
        use diesel::prelude::*;
        use diesel::SqliteConnection;

        use crate::schema::channels::dsl::*;

        use super::Channel;
        use super::NewChannel;

        pub fn insert(new_channel: NewChannel, db: &SqliteConnection) -> Result<(), diesel::result::Error> {
            diesel::insert_into(channels).values(&new_channel).execute(db)?;
            Ok(())
        }

        pub fn select_all(db: &SqliteConnection) -> Result<Vec<Channel>, diesel::result::Error> {
            let r = channels.load::<Channel>(db)?;
            Ok(r)
        }

        pub fn select_by_id(predicate: i32, db: &SqliteConnection) -> Result<Channel, diesel::result::Error> {
            Ok(channels.filter(id.eq(predicate))
                .first::<Channel>(db)?)
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
        pub channel_id: i32,
    }
    
    impl NewItem {
        /// Create an item to be inserted in the database, from a rss item.
        pub fn from_rss_item(item: rss::Item, channel_id: i32) -> NewItem {
            let title = item.title;
            let guid= item.guid.map(|x| x.value);
            let url = item.link;
            let content = item.description;
            let read = false;
            
            NewItem{title, guid, url, content, channel_id, read}
        }
    }

    pub mod db {
        use diesel::prelude::*;
        use diesel::SqliteConnection;

        use crate::schema::items::dsl::*;

        use super::Item;
        use super::NewItem;

        pub fn insert(new_item: NewItem, db: &SqliteConnection) -> Result<(), diesel::result::Error> {
            diesel::insert_into(items).values(&new_item).execute(db)?;
            Ok(())
        }
        
        pub fn get_items_of_channel(chan_id: i32,  db: &SqliteConnection) ->  Result<Vec<Item>, diesel::result::Error> {
            let res = items.filter(channel_id.eq(chan_id))
                .load::<Item>(db)?;
            
            Ok(res)
        }
    }
}

