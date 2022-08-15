use std::env;
use std::net::TcpListener;

use rss_aggregator::databases::{init_postgres_connection, init_redis_connection};
use rss_aggregator::observability;
use rss_aggregator::observability::{get_subscriber, init_subscriber};
use rss_aggregator::startup;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Init dotenv
    dotenv::dotenv().ok();

    let subscriber = get_subscriber("rss_aggregator".into(), "info".into());
    init_subscriber(subscriber);

    let postgres_connection = init_postgres_connection().await;
    let redis_pool = init_redis_connection();

    let listener = TcpListener::bind(
        env::var("RSS_AGGREGATOR_LISTEN_ON").unwrap_or_else(|_| String::from("0.0.0.0:8080")),
    )?;

    let _sentry_guard = observability::init_sentry();

    startup::startup(postgres_connection, redis_pool, listener).await
}
