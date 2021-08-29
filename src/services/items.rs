use std::sync::Arc;

use crate::model::{Item, NewItem};
use crate::schema::items::dsl::*;
use crate::DbPool;
use diesel::prelude::*;

#[derive(Clone)]
pub struct ItemService {
    pool: Arc<DbPool>,
}

impl ItemService {
    pub fn new(pool: DbPool) -> Self {
        Self {
            pool: Arc::new(pool),
        }
    }

    pub fn insert(&self, new_item: NewItem) -> Result<Item, diesel::result::Error> {
        let connection = self.pool.get().unwrap();

        let generated_id: i32 = diesel::insert_into(items)
            .values(&new_item)
            .returning(id)
            .get_result(&connection)?;

        items.filter(id.eq(generated_id)).first::<Item>(&connection)
    }

    pub fn get_items_of_channel(&self, chan_id: i32) -> Result<Vec<Item>, diesel::result::Error> {
        log::debug!("Getting items of channel {}", chan_id);
        items
            .filter(channel_id.eq(chan_id))
            .load::<Item>(&self.pool.get().unwrap())
    }
}
