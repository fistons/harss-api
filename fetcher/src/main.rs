use dotenvy::dotenv;
use reqwest::Client;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::time::Duration;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let subscriber =
        rss_common::observability::get_subscriber("rss_aggregator-fetcher".into(), "info".into());
    rss_common::observability::init_subscriber(subscriber);

    let _sentry_guard = rss_common::observability::init_sentry();

    let client = build_client().expect("Could not build client");
    let db = build_pool().await;

    fetcher::fetch(client, db).await;
}

fn build_client() -> reqwest::Result<Client> {
    reqwest::ClientBuilder::new()
        .timeout(Duration::from_secs(10))
        .user_agent("rss-aggregator fetcher (+https://github.com/fistons/rss-aggregator)")
        .build()
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
