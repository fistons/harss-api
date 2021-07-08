use std::sync::Arc;

use diesel::prelude::*;

use crate::model::channel::{Channel, NewChannel};
use crate::schema::channels::dsl::*;
use crate::DbPool;

#[derive(Clone)]
pub struct ChannelService {
    pool: Arc<DbPool>,
}

impl ChannelService {
    pub fn new(pool: DbPool) -> Self {
        Self {
            pool: Arc::new(pool),
        }
    }

    pub fn insert(&self, new_channel: NewChannel) -> Result<Channel, diesel::result::Error> {
        let connection = self.pool.get().unwrap();

        let generated_id: i32 = diesel::insert_into(channels)
            .values(&new_channel)
            .returning(id)
            .get_result(&connection)?;

        channels
            .filter(id.eq(generated_id))
            .first::<Channel>(&connection)
    }

    pub fn select_all_by_user_id(&self, u_id: i32) -> Result<Vec<Channel>, diesel::result::Error> {
        channels
            .filter(user_id.eq(u_id))
            .load::<Channel>(&self.pool.get().unwrap())
    }

    pub fn select_by_id_and_user_id(
        &self,
        u_id: i32,
        chan_id: i32,
    ) -> Result<Channel, diesel::result::Error> {
        channels
            .filter(id.eq(chan_id).and(user_id.eq(u_id)))
            .first::<Channel>(&self.pool.get().unwrap())
    }
}
