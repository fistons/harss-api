use std::sync::Arc;

use diesel::prelude::*;
use diesel::select;

use crate::model::channel::{Channel, NewChannel};
use crate::schema::channels::dsl::*;
use crate::{last_insert_rowid, DbPool};

pub fn insert(
    new_channel: NewChannel,
    pool: Arc<DbPool>,
) -> Result<Channel, diesel::result::Error> {
    let connection = pool.get().unwrap();

    diesel::insert_into(channels)
        .values(&new_channel)
        .execute(&connection)?;

    let generated_id: i32 = select(last_insert_rowid).first(&connection).unwrap();

    channels
        .filter(id.eq(generated_id))
        .first::<Channel>(&connection)
}

pub fn select_all(pool: &Arc<DbPool>) -> Result<Vec<Channel>, diesel::result::Error> {
    channels.load::<Channel>(&pool.get().unwrap())
}

pub fn select_by_id(predicate: i32, pool: &Arc<DbPool>) -> Result<Channel, diesel::result::Error> {
    channels
        .filter(id.eq(predicate))
        .first::<Channel>(&pool.get().unwrap())
}
