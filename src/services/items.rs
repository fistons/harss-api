use std::sync::Arc;

use diesel::prelude::*;
use diesel::select;

use crate::model::item::{Item, NewItem};
use crate::schema::items::dsl::*;
use crate::{last_insert_rowid, DbPool};

#[derive(Clone)]
pub struct ItemService {
    pub(crate) pool: Arc<DbPool>,
}

impl ItemService {
    pub fn insert(&self, new_item: NewItem) -> Result<Item, diesel::result::Error> {
        let connection = self.pool.get().unwrap();

        diesel::insert_into(items)
            .values(&new_item)
            .execute(&connection)?;

        let generated_id: i32 = select(last_insert_rowid).first(&connection).unwrap();

        items.filter(id.eq(generated_id)).first::<Item>(&connection)
    }

    pub fn get_items_of_channel(&self, chan_id: i32) -> Result<Vec<Item>, diesel::result::Error> {
        log::debug!("Refreshing items of channel {}", chan_id);
        items
            .filter(channel_id.eq(chan_id))
            .load::<Item>(&self.pool.get().unwrap())
    }
}
