use std::sync::Arc;

use entity::channels as channel;

use crate::errors::ApiError;
use crate::model::HttpNewItem;
use crate::services::channels::ChannelService;
use crate::services::items::ItemService;

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

    pub async fn refresh_all_channels(&self) {
        log::info!("Refreshing all channels");
        match self.channel_service.select_all().await {
            Ok(channels) => {
                for channel in channels.iter() {
                    if let Err(oops) = self.refresh_channel(channel).await {
                        log::error!("Couldn't refresh channel {}: {:?}", channel.id, oops);
                    }
                }
            }
            Err(oops) => {
                log::error!("Couldn't get channels to refresh {:?}", oops);
            }
        }
        log::info!("Refreshing all channels done");
    }

    pub async fn refresh_channel(&self, channel: &channel::Model) -> Result<(), ApiError> {
        log::debug!("Fetching {}", channel.name);
        // Get the ids of the already fetched items
        let items = self
            .item_service
            .get_all_items_of_channel(channel.id)
            .await?;
        let items: Vec<&String> = items
            .iter()
            .filter_map(|x| x.guid.as_ref().or(x.url.as_ref()))
            .collect();

        let content = reqwest::get(&channel.url)
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();

        let rss_channel = feed_rs::parser::parse(&content[..])?;
        for item in rss_channel.entries.into_iter() {
            let i = HttpNewItem::from_rss_item(item, channel.id);
            match i.guid.as_ref().or(i.url.as_ref()) {
                Some(ref x) if !items.contains(x) => {
                    self.item_service.insert(i, channel.id).await.unwrap();
                }
                _ => (),
            };
        }

        Ok(())
    }
}
