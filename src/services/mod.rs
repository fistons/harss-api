use core::time::Duration;
use std::collections::HashSet;
use std::sync::Arc;

use chrono::Utc;
use tracing::Instrument;

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
    client: reqwest::Client,
}

impl GlobalService {
    pub fn new(item_service: ItemService, channel_service: ChannelService) -> Self {
        Self {
            item_service: Arc::new(item_service),
            channel_service: Arc::new(channel_service),
            client: reqwest::ClientBuilder::default()
                .timeout(Duration::from_secs(
                    std::env::var("FETCH_TIMEOUT")
                        .unwrap_or_else(|_| String::from("3"))
                        .parse()
                        .expect("FETCH_TIMEOUT must be an integer"),
                ))
                .user_agent("rss-aggregator")
                .build()
                .unwrap(),
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

    #[tracing::instrument(skip(self, channels), level = "debug")]
    async fn update_channels(&self, channels: Vec<HttpChannel>) {
        let mut tasks = vec![];
        for channel in channels.into_iter() {
            let service = self.clone();
            let future = async move {
                if let Err(oops) = service.refresh_channel(&channel).await {
                    tracing::error!("Couldn't refresh channel {}: {:?}", channel.id, oops);

                    if let Err(oops) = service.channel_service.fail_channels(channel.id).await {
                        tracing::error!("Couldn't fail channel {}: {:?}", channel.id, oops);
                    }
                }
            };
            tasks.push(tokio::spawn(future.in_current_span()));
        }

        for task in tasks {
            task.await.unwrap();
        }

        if let Err(x) = self.channel_service.disable_channels().await {
            tracing::error!("Error while disabling failed channel count: {}", x);
        }
    }

    #[tracing::instrument(skip(self), level = "debug")]
    pub async fn refresh_channel(&self, channel: &HttpChannel) -> Result<(), ApiError> {
        // Fetch the content of the channel

        let content = self.client.get(&channel.url).send().await?.bytes().await?;
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
