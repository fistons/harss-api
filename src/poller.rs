use std::time::Duration;

use clokwerk::{AsyncScheduler, TimeUnits};
use sea_orm::DatabaseConnection;

use crate::services::channels::ChannelService;
use crate::services::items::ItemService;
use crate::services::GlobalService;

pub async fn start_poller(db: DatabaseConnection) {
    let item_service = ItemService::new(db.clone());
    let channel_service = ChannelService::new(db);
    let global_service = GlobalService::new(item_service, channel_service);

    let mut scheduler = AsyncScheduler::new();
    let polling_interval = std::env::var("POLLING_INTERVAL")
        .unwrap_or_else(|_| String::from("300"))
        .parse::<u32>()
        .unwrap()
        .seconds();
    tracing::info!("Poll every {:?}", polling_interval);

    scheduler
        .every(polling_interval)
        .run(move || refresh(global_service.clone()));
    actix_rt::spawn(async move {
        loop {
            scheduler.run_pending().await;
            actix_rt::time::sleep(Duration::from_millis(100)).await;
        }
    });
}

#[tracing::instrument(skip_all)]
async fn refresh(service: GlobalService) {
    service.refresh_all_channels().await;
}
