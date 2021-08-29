use serde::{Deserialize, Serialize};
use diesel_derive_enum::DbEnum;

use crate::schema::channels;
use crate::schema::items;
use crate::schema::users;
use crate::schema::channel_users;

pub mod configuration;

#[derive(Debug, Serialize, Deserialize, Clone, Insertable)]
#[table_name = "channels"]
pub struct NewChannel {
    pub name: String,
    pub url: String
}

/// Source of articles, over da web
#[derive(Debug, Serialize, Deserialize, Clone, Queryable, Associations, Identifiable)]
pub struct Channel {
    pub id: i32,
    pub name: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Insertable)]
#[table_name = "users"]
pub struct NewUser {
    pub username: String,
    pub password: String,
    pub role: UserRole,
}

#[derive(Debug, Serialize, Deserialize, Clone, Queryable, Associations, Identifiable)]
pub struct User {
    pub id: i32,
    pub username: String,
    #[serde(skip_serializing)]
    pub password: String,
    pub role: UserRole,
}

#[derive(DbEnum, Debug, Serialize, Deserialize, Clone, PartialOrd, PartialEq)]
#[PgType = "user_role"]
#[DieselType = "User_role"]
pub enum UserRole {
    Basic,
    Admin,
}

#[derive(Queryable, Associations, Identifiable, Insertable)]
#[belongs_to(User)]
#[belongs_to(Channel)]
#[primary_key(channel_id,user_id)]
pub struct ChannelUser {
    pub channel_id: i32,
    pub user_id: i32,
}

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
            guid,
            title,
            url,
            content,
            read,
            channel_id,
        }
    }
}
