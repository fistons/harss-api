use std::sync::Arc;

use diesel::prelude::*;
use diesel::select;

use crate::{DbPool, last_insert_rowid};
use crate::model::channel::{Channel, NewChannel};
use crate::schema::channels::dsl::*;

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

pub fn select_all_by_user_id(pool: &Arc<DbPool>, u_id: i32) -> Result<Vec<Channel>, diesel::result::Error> {
    channels.filter(user_id.eq(u_id)).load::<Channel>(&pool.get().unwrap())
    // channels.load::<Channel>(&pool.get().unwrap())
}

pub fn select_by_id_and_user_id(u_id: i32, chan_id: i32, pool: &Arc<DbPool>) -> Result<Channel, diesel::result::Error> {
    channels
        .filter(id.eq(chan_id).and(user_id.eq(u_id)))
        .first::<Channel>(&pool.get().unwrap())
}
