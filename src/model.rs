use diesel::SqliteConnection;
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
    use crate::schema::channels::dsl::*;
    use crate::schema::channels;
    use diesel::prelude::*;

    use diesel::SqliteConnection;
    
    use super::Channel as Channel;
    use super::NewChannel as NewChannel;
    

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