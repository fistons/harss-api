use std::sync::Arc;

use diesel::pg::expression::dsl::any;
use diesel::prelude::*;

use crate::model::{Channel, NewChannel, ChannelUser};
use crate::schema::channel_users::dsl::*;
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

    pub fn create_or_link_channel(
        &self,
        new_channel: NewChannel,
        other_user_id: i32,
    ) -> Result<Channel, diesel::result::Error> {
        let connection = self.pool.get().unwrap();

        let chan = match channels
            .filter(url.eq(&new_channel.url))
            .first::<Channel>(&connection)
        {
            Ok(found_id) => found_id,
            Err(diesel::NotFound) => self.create_new_channel(&new_channel)?,
            x => return x,
        };
        
        let new_channel_user = ChannelUser{channel_id: chan.id, user_id: other_user_id};
        
        diesel::insert_into(channel_users)
            .values(new_channel_user)
            .on_conflict_do_nothing()
            .execute(&connection)?;

        Ok(chan)
    }

    fn create_new_channel(
        &self,
        new_channel: &NewChannel,
    ) -> Result<Channel, diesel::result::Error> {
        let connection = self.pool.get().unwrap();

        let generated_id: i32 = diesel::insert_into(channels)
            .values(new_channel)
            .returning(id)
            .get_result(&connection)?;

        channels
            .filter(id.eq(generated_id))
            .first::<Channel>(&connection)
    }

    pub fn select_all_by_user_id(&self, u_id: i32) -> Result<Vec<Channel>, diesel::result::Error> {
        let channel_ids = channel_users
            .filter(user_id.eq(u_id))
            .select(crate::schema::channel_users::columns::channel_id);

        channels
            .filter(id.eq(any(channel_ids)))
            .load::<Channel>(&self.pool.get().unwrap())
    }

    pub fn select_all(&self) -> Result<Vec<Channel>, diesel::result::Error> {
        channels.load::<Channel>(&self.pool.get().unwrap())
    }

    pub fn select_by_id(&self, other_channel_id: i32) -> Result<Channel, diesel::result::Error> {
        channels
            .filter(id.eq(other_channel_id))
            .first::<Channel>(&self.pool.get().unwrap())
    }

    pub fn select_by_id_and_user_id(
        &self,
        u_id: i32,
        chan_id: i32,
    ) -> Result<Channel, diesel::result::Error> {
        let channel_ids = channel_users
            .filter(user_id.eq(u_id))
            .select(crate::schema::channel_users::columns::channel_id);

        channels
            .filter(id.eq(any(channel_ids)).and(id.eq(chan_id)))
            .first::<Channel>(&self.pool.get().unwrap())
    }
}
