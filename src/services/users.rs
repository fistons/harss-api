use std::sync::Arc;

use diesel::prelude::*;
use diesel::select;

use crate::model::user::{NewUser, User};
use crate::schema::users::dsl::*;
use crate::{last_insert_rowid, DbPool};

pub fn create_user(new_user: NewUser, pool: &Arc<DbPool>) -> Result<User, diesel::result::Error> {
    let connection = pool.get().unwrap();
    diesel::insert_into(users)
        .values(&new_user)
        .execute(&connection)?;

    let generated_id: i32 = select(last_insert_rowid).first(&connection).unwrap();

    users.filter(id.eq(generated_id)).first::<User>(&connection)
}

pub fn list_users(pool: &Arc<DbPool>) -> Result<Vec<User>, diesel::result::Error> {
    users.load::<User>(&pool.get().unwrap())
}
