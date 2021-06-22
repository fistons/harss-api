use std::sync::Arc;

use log::debug;

use crate::services::channels::ChannelService;
use crate::services::items::ItemService;

pub mod auth;
pub mod channels;
pub mod items;
pub mod users;

#[derive(Clone)]
pub struct GlobalService {
    pub(crate) item_service: Arc<ItemService>,
    pub(crate) channel_service: Arc<ChannelService>,
}

impl GlobalService {
    pub fn refresh(
        &self,
        // item_service: &ItemService,
        // channel_service: &ChannelService,
        user_id: i32,
    ) -> Result<(), diesel::result::Error> {
        let channels = self.channel_service.select_all_by_user_id(user_id)?;

        for channel in channels.iter() {
            self.refresh_chan(channel.id, user_id)?;
        }
        Ok(())
    }

    pub fn refresh_chan(
        &self,
        // item_service: &ItemService,
        // channel_service: &ChannelService,
        channel_id: i32,
        user_id: i32,
    ) -> Result<(), diesel::result::Error> {
        let channel = self
            .channel_service
            .select_by_id_and_user_id(channel_id, user_id)?;
        debug!("Fetching {}", &channel.name);

        let content = reqwest::blocking::get(&channel.url)
            .unwrap()
            .bytes()
            .unwrap();
        let rss_channel = rss::Channel::read_from(&content[..]).unwrap();
        for item in rss_channel.items.into_iter() {
            let i = crate::model::item::NewItem::from_rss_item(item, channel.id);
            self.item_service.insert(i).unwrap();
        }

        Ok(())
    }
}
