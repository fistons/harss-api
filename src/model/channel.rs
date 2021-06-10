use serde::{Deserialize, Serialize};

use crate::schema::channels;

#[derive(Debug, Serialize, Deserialize, Clone, Insertable)]
#[table_name = "channels"]
pub struct NewChannel {
    pub name: String,
    pub url: String,
    pub user_id: i32,
}

impl NewChannel {
    pub fn set_user_id(&mut self, user_id: i32) {
        self.user_id = user_id;
    }
}

/// Source of articles, over da web
#[derive(Debug, Serialize, Deserialize, Clone, Queryable)]
pub struct Channel {
    pub id: i32,
    pub name: String,
    pub url: String,
    pub user_id: i32,
}
