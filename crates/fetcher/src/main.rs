use std::time::Duration;

use dotenvy::dotenv;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};

use fetcher::process;
use rss_common::observability;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let subscriber = observability::get_subscriber("rss_aggregator-fetcher", "info");
    observability::init_subscriber(subscriber);

    let _sentry_guard = observability::init_sentry();

    let db = build_pool().await;

    if let Err(err) = process(&db).await {
        tracing::error!("Ho noes! {err}");
    }
}

async fn build_pool() -> DatabaseConnection {
    let mut opt = ConnectOptions::new(std::env::var("DATABASE_URL").expect("DATABASE_URL needed"));
    opt.min_connections(5)
        .max_connections(10)
        .connect_timeout(Duration::from_secs(8))
        .idle_timeout(Duration::from_secs(8))
        .max_lifetime(Duration::from_secs(8));

    Database::connect(opt)
        .await
        .expect("Could not setup database")
}
