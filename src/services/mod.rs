use std::sync::Arc;

use chrono::Utc;

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

    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn refresh_all_channels(&self) {
        match self.channel_service.select_all().await {
            Ok(channels) => {
                for channel in channels.iter() {
                    if let Err(oops) = self.refresh_channel(channel).await {
                        tracing::error!("Couldn't refresh channel {}: {:?}", channel.id, oops);
                    }
                }
            }
            Err(oops) => {
                tracing::error!("Couldn't get channels to refresh {:?}", oops);
            }
        }
    }

    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn refresh_channel_of_user(&self, user_id: i32) {
        match self.channel_service.select_all_by_user_id(user_id).await {
            Ok(channels) => {
                for channel in channels.iter() {
                    if let Err(oops) = self.refresh_channel(channel).await {
                        tracing::error!("Couldn't refresh channel {}: {:?}", channel.id, oops);
                    }
                }
            }
            Err(oops) => {
                tracing::error!("Couldn't get channels to refresh {:?}", oops);
            }
        }
    }

    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn refresh_channel(&self, channel: &HttpChannel) -> Result<(), ApiError> {
        // Get the ids of the already fetched items
        let items = self
            .item_service
            .get_all_items_of_channel(channel.id)
            .await?;
        let items: Vec<&String> = items
            .iter()
            .filter_map(|x| x.guid.as_ref().or(x.url.as_ref()))
            .collect();

        let content = reqwest::get(&channel.url).await?.bytes().await?;

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
