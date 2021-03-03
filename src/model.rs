/// # A bunch of stuff
use serde::{Deserialize, Serialize};
use crate::schema::channels;

#[table_name="channels"]
#[derive(Debug, Serialize, Deserialize, Clone, Insertable)]
pub struct NewChannel {
    pub name: String,
    pub url: String
}

/// Source of articles, over da web
#[table_name="channels"]
#[derive(Debug, Serialize, Deserialize, Clone, Queryable)]
pub struct Channel {
    pub id: i32,
    pub name: String,
    pub url: String
}

