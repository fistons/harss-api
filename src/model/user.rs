use diesel_derive_enum::DbEnum;
use serde::{Deserialize, Serialize};

use crate::schema::users;

#[derive(Debug, Serialize, Deserialize, Clone, Insertable)]
#[table_name = "users"]
pub struct NewUser {
    pub username: String,
    pub password: String,
    pub role: UserRole,
}

#[derive(Debug, Serialize, Deserialize, Clone, Queryable)]
pub struct User {
    pub id: i32,
    pub username: String,
    #[serde(skip_serializing)]
    pub password: String,
    pub role: UserRole,
}

#[derive(DbEnum, Debug, Serialize, Deserialize, Clone)]
#[PgType = "user_role"]
#[DieselType = "User_role"]
pub enum UserRole {
    Basic,
    Admin,
}
