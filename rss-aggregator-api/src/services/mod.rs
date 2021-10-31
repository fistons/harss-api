use std::sync::Arc;

use log::debug;

use crate::services::channels::ChannelService;
use crate::services::items::ItemService;
use crate::model::Channel;
use crate::errors::ApiError;

pub mod auth;
pub mod channels;
pub mod items;
pub mod users;

#[derive(Clone)]
pub struct GlobalService {
    item_service: Arc<ItemService>,
    channel_service: Arc<ChannelService>,
}

impl GlobalService {
    pub fn new(item_service: ItemService, channel_service: ChannelService) -> Self {
        Self {
            item_service: Arc::new(item_service),
            channel_service: Arc::new(channel_service),
        }
    }
    
    pub fn refresh_all_channels(&self) -> Result<(), ApiError> {
        log::info!("Refreshing all channels");
        let channels = self.channel_service.select_all()?;

        for channel in channels.iter() {
            self.refresh_channel(channel)?;
            log::debug!("done");
        }
        Ok(())
    }

    pub fn refresh_channel(&self, channel: &Channel) -> Result<(), ApiError> {
        debug!("Fetching {}", &channel.name);
        // Get the ids of the already fetched items
        let items = self.item_service.get_items_of_channel(channel.id)?;
        let items: Vec<&String> = items.iter().map(|x| x.guid.as_ref().or_else(|| x.url.as_ref())).flatten().collect();

        let content = reqwest::blocking::get(&channel.url)
            .unwrap()
            .bytes()
            .unwrap();
        let rss_channel = rss::Channel::read_from(&content[..]).unwrap();
        for item in rss_channel.items.into_iter() {
            let i = crate::model::NewItem::from_rss_item(item, channel.id);
            log::debug!("{:?}", i);
            match i.guid.as_ref().or_else(|| i.url.as_ref()) {
                Some(ref x) if !items.contains(x) => {
                    self.item_service.insert(i).unwrap();
                }
                _ => ()
            };
        }

        Ok(())
    }
}
