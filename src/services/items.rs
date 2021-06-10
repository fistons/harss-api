use std::sync::Arc;

use diesel::prelude::*;
use diesel::select;

use crate::model::item::{Item, NewItem};
use crate::schema::items::dsl::*;
use crate::{last_insert_rowid, DbPool};


pub fn insert(new_item: NewItem, pool: &Arc<DbPool>) -> Result<Item, diesel::result::Error> {
    let connection = pool.get().unwrap();

    diesel::insert_into(items)
        .values(&new_item)
        .execute(&connection)?;

    let generated_id: i32 = select(last_insert_rowid).first(&connection).unwrap();

    items.filter(id.eq(generated_id)).first::<Item>(&connection)
}

pub fn get_items_of_channel(
    chan_id: i32,
    pool: &Arc<DbPool>,
) -> Result<Vec<Item>, diesel::result::Error> {
    items.filter(channel_id.eq(chan_id)).load::<Item>(&pool.get().unwrap())
}
