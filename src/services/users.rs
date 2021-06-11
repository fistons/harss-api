use std::sync::Arc;

use diesel::prelude::*;
use diesel::select;

use crate::model::user::{NewUser, User};
use crate::schema::users::dsl::*;
use crate::{last_insert_rowid, DbPool};

pub fn create_user(
    login: &str,
    pwd: &str,
    pool: &Arc<DbPool>,
) -> Result<User, diesel::result::Error> {
    let connection = pool.get().unwrap();

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

pub fn list_users(pool: &Arc<DbPool>) -> Result<Vec<User>, diesel::result::Error> {
    users.load::<User>(&pool.get().unwrap())
}

pub fn get_user(wanted_username: &str, pool: &Arc<DbPool>) -> Result<User, diesel::result::Error> {
    users
        .filter(username.eq(wanted_username))
        .first::<User>(&pool.get().unwrap())
}

fn encode_password(pwd: &str) -> String {
    let salt = std::env::var("PASSWORD_SALT").unwrap_or_else(|_| String::from("lepetitcerebos"));
    let config = argon2::Config::default();

    argon2::hash_encoded(pwd.as_bytes(), salt.as_bytes(), &config).unwrap()
}

pub fn match_password(user: &User, candidate: &str) -> bool {
    argon2::verify_encoded(&user.password, candidate.as_bytes()).unwrap()
}
