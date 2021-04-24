use serde::{Deserialize, Serialize};

use crate::schema::users;

#[derive(Debug, Serialize, Deserialize, Clone, Insertable)]
#[table_name = "users"]
pub struct NewUser {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Queryable)]
pub struct User {
    pub id: i32,
    pub username: String,
    #[serde(skip_serializing)]
    pub password: String,
}
