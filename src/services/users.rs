use std::sync::Arc;

use diesel::prelude::*;
use diesel::select;

use crate::model::user::{NewUser, User};
use crate::schema::users::dsl::*;
use crate::{last_insert_rowid, DbPool};

#[derive(Clone)]
pub struct UserService {
    pub(crate) pool: Arc<DbPool>,
}

impl UserService {
    pub fn create_user(&self, login: &str, pwd: &str) -> Result<User, diesel::result::Error> {
        let connection = self.pool.get().unwrap();

        let new_user = NewUser {
            username: String::from(login),
            password: encode_password(pwd),
        };

        diesel::insert_into(users)
            .values(&new_user)
            .execute(&connection)?;

        let generated_id: i32 = select(last_insert_rowid).first(&connection).unwrap();

        users.filter(id.eq(generated_id)).first::<User>(&connection)
    }

    pub fn list_users(&self) -> Result<Vec<User>, diesel::result::Error> {
        users.load::<User>(&self.pool.get().unwrap())
    }

    pub fn get_user(&self, wanted_username: &str) -> Result<User, diesel::result::Error> {
        users
            .filter(username.eq(wanted_username))
            .first::<User>(&self.pool.get().unwrap())
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
