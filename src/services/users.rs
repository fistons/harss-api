use std::sync::Arc;

use sea_orm::{entity::*, query::*};
use sea_orm::DatabaseConnection;

use entity::sea_orm_active_enums::UserRole;
use entity::users;
use entity::users::Entity as User;

use crate::errors::ApiError;

#[derive(Clone)]
pub struct UserService {
    db: Arc<DatabaseConnection>,
}

impl UserService {
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            db: Arc::new(db),
        }
    }

    pub async fn get_user(&self, wanted_username: &str) -> Result<Option<users::Model>, ApiError> {
        Ok(User::find()
            .filter(users::Column::Username.eq(wanted_username))
            .one(self.db.as_ref()).await?)
    }

    pub async fn list_users(&self) -> Result<Vec<users::Model>, ApiError> {
        Ok(User::find()
            .all(self.db.as_ref()).await?)
    }

    pub async fn create_user(
        &self,
        login: &str,
        pwd: &str,
        user_role: UserRole,
    ) -> Result<users::Model, ApiError> {
        let new_user = users::ActiveModel {
            id: NotSet,
            username: Set(String::from(login)),
            password: Set(encode_password(pwd)),
            role: Set(user_role),
        };

        Ok(new_user.insert(self.db.as_ref()).await?)
    }
}

fn encode_password(pwd: &str) -> String {
    let salt = std::env::var("PASSWORD_SALT").unwrap_or_else(|_| String::from("lepetitcerebos"));
    let config = argon2::Config::default();

    argon2::hash_encoded(pwd.as_bytes(), salt.as_bytes(), &config).unwrap()
}

pub fn match_password(user: &users::Model, candidate: &str) -> bool {
    argon2::verify_encoded(&user.password, candidate.as_bytes()).unwrap()
}
