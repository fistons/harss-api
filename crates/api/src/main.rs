use api::databases::{init_postgres_connection, init_redis_connection};
use api::startup;
use std::env;
use std::net::TcpListener;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Init dotenv
    dotenvy::dotenv().ok();

    let subscriber = common::observability::get_subscriber("rss_aggregator", "info");
    common::observability::init_subscriber(subscriber);

    let postgres_connection = init_postgres_connection().await;
    let redis_pool = init_redis_connection();

    let listener = TcpListener::bind(
        env::var("RSS_AGGREGATOR_LISTEN_ON").unwrap_or_else(|_| String::from("0.0.0.0:8080")),
    )?;

    let _sentry_guard = common::observability::init_sentry();

    startup::startup(postgres_connection, redis_pool, listener).await
}
