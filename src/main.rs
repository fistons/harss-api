use std::env;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::net::TcpListener;
use std::time::Duration;

use clokwerk::{AsyncScheduler, TimeUnits};

use rss_aggregator::database::init_database;
use rss_aggregator::model::configuration::ApplicationConfiguration;
use rss_aggregator::observability::{get_subscriber, init_subscriber};
use rss_aggregator::startup;
use rss_aggregator::services::channels::ChannelService;
use rss_aggregator::services::items::ItemService;
use rss_aggregator::services::GlobalService;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Init dotenv
    dotenv::dotenv().ok();

    let subscriber = get_subscriber("rss_aggregator".into(), "info".into());
    init_subscriber(subscriber);

    let db = init_database().await;

    let item_service = ItemService::new(db.clone());
    let channel_service = ChannelService::new(db.clone());
    let global_service = GlobalService::new(item_service.clone(), channel_service.clone());

    let configuration = load_configuration().unwrap();

    let mut scheduler = AsyncScheduler::new();
    let global = global_service.clone();

    let polling = std::env::var("POLLING_INTERVAL")
        .unwrap_or_else(|_| String::from("300"))
        .parse::<u32>()
        .unwrap()
        .seconds();
    tracing::info!("Poll every {:?}", polling);

    let listener = TcpListener::bind(
        env::var("RSS_AGGREGATOR_LISTEN_ON").unwrap_or_else(|_| String::from("0.0.0.0:8080")),
    )?;

    scheduler
        .every(polling)
        .run(move || refresh(global.clone()));
    actix_rt::spawn(async move {
        loop {
            scheduler.run_pending().await;
            actix_rt::time::sleep(Duration::from_millis(100)).await;
        }
    });

    startup::startup(db, configuration, listener).await
}

async fn refresh(service: GlobalService) {
    service.refresh_all_channels().await;
}

fn load_configuration() -> Result<ApplicationConfiguration, Box<dyn Error>> {
    let file = File::open(
        std::env::var("CONFIG_PATH").unwrap_or_else(|_| String::from("configuration.yaml")),
    )?;
    let reader = BufReader::new(file);
    let configuration = serde_yaml::from_reader(reader)?;

    Ok(configuration)
}
