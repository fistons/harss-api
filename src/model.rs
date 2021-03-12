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
        pub title: String,
        pub url: String,
        pub content: String,
        pub channel_id: i32,
    }

    #[derive(Debug, Serialize, Deserialize, Clone, Queryable)]
    pub struct Item {
        pub id: i32,
        pub title: String,
        pub url: String,
        pub content: String,
        pub channel_id: i32,
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

    }
}

