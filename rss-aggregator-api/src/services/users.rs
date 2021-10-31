use std::sync::Arc;

use diesel::prelude::*;

use crate::model::{NewUser, User, UserRole};
use crate::schema::users::dsl::*;
use crate::DbPool;
use crate::errors::ApiError;

#[derive(Clone)]
pub struct UserService {
    pool: Arc<DbPool>,
}

impl UserService {
    pub fn new(pool: DbPool) -> Self {
        Self {
            pool: Arc::new(pool),
        }
    }

    pub fn create_user(
        &self,
        login: &str,
        pwd: &str,
        user_role: &UserRole,
    ) -> Result<User, ApiError> {
        let connection = self.pool.get().unwrap();

        let new_user = NewUser {
            username: String::from(login),
            password: encode_password(pwd),
            role: user_role.clone(),
        };

        let generated_id: i32 = diesel::insert_into(users)
            .values(&new_user)
            .returning(id)
            .get_result(&connection)?;

        Ok(users.filter(id.eq(generated_id)).first::<User>(&connection)?)
    }

    pub fn list_users(&self) -> Result<Vec<User>, ApiError> {
        Ok(users.load::<User>(&self.pool.get().unwrap())?)
    }

    pub fn get_user(&self, wanted_username: &str) -> Result<User, ApiError> {
        Ok(users
            .filter(username.eq(wanted_username))
            .first::<User>(&self.pool.get().unwrap())?)
    }
}

fn encode_password(pwd: &str) -> String {
    let salt = std::env::var("PASSWORD_SALT").unwrap_or_else(|_| String::from("lepetitcerebos"));
    let config = argon2::Config::default();

    argon2::hash_encoded(pwd.as_bytes(), salt.as_bytes(), &config).unwrap()
}

pub fn match_password(user: &User, candidate: &str) -> bool {
    argon2::verify_encoded(&user.password, candidate.as_bytes()).unwrap()
}
