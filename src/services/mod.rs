use chrono::Utc;
use std::sync::Arc;

use crate::errors::ApiError;
use crate::model::HttpChannel;
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

    pub async fn refresh_channel_of_user(&self, user_id: i32) {
        log::info!("Refresh channels of user {user_id}");
        match self.channel_service.select_all_by_user_id(user_id).await {
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
        log::info!("Refresh channels of user {user_id} done");
    }

    pub async fn refresh_channel(&self, channel: &HttpChannel) -> Result<(), ApiError> {
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
        let mut channel_updated = false;
        for item in rss_channel.entries.into_iter() {
            if !items.contains(&&item.id) {
                self.item_service.insert(item, channel.id).await.unwrap();
                channel_updated = true;
            }
        }

        if channel_updated {
            let last_update = rss_channel.updated.unwrap_or_else(Utc::now);
            self.channel_service
                .update_last_fetched(channel.id, last_update)
                .await?;
        }

        Ok(())
    }
}
