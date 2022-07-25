use std::collections::HashSet;
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
        match self.channel_service.select_all_enabled().await {
            Ok(channels) => self.update_channels(channels).await,
            Err(oops) => {
                tracing::error!("Couldn't get channels to refresh {:?}", oops);
            }
        }
    }

    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn refresh_channel_of_user(&self, user_id: i32) {
        match self
            .channel_service
            .select_all_enabled_by_user_id(user_id)
            .await
        {
            Ok(channels) => self.update_channels(channels).await,
            Err(oops) => {
                tracing::error!("Couldn't get channels to refresh {:?}", oops);
            }
        }
    }

    #[tracing::instrument(skip(self), level = "debug")]
    async fn update_channels(&self, channels: Vec<HttpChannel>) {
        let mut failed_channels: Vec<i32> = vec![];
        for channel in channels.iter() {
            if let Err(oops) = self.refresh_channel(channel).await {
                failed_channels.push(channel.id);
                tracing::error!("Couldn't refresh channel {}: {:?}", channel.id, oops);
            }
        }
        if let Err(x) = self.channel_service.fail_channels(failed_channels).await {
            tracing::error!("Error while updating failed channel count: {}", x);
        }
    }

    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn refresh_channel(&self, channel: &HttpChannel) -> Result<(), ApiError> {
        // Fetch the content of the channel
        let content = reqwest::get(&channel.url).await?.bytes().await?;
        let rss_channel = feed_rs::parser::parse(&content[..])?;

        // Get the ids of the already fetched items
        let items_set = self
            .item_service
            .get_all_items_of_channel(channel.id)
            .await?
            .into_iter()
            .filter_map(|x| x.guid.or(x.url))
            .collect::<HashSet<String>>();

        // Filters out the item not already fetched
        //TODO: check how the updated articles behave
        let new_items = rss_channel
            .entries
            .into_iter()
            .filter(|item| !items_set.contains(&item.id))
            .collect::<Vec<_>>();

        let mut channel_updated = false;
        for new_item in new_items {
            self.item_service.insert(new_item, channel.id).await?;
            channel_updated = true;
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
