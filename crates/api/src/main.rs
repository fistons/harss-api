use api::databases::init_redis_connection;
use api::startup;
use common::init_postgres_connection;
use std::env;
use std::net::TcpListener;
use tracing::error;

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

    if !check_configuration() {
        panic!()
    }

    startup::startup(postgres_connection, redis_pool, listener).await
}

/// Check that the configuration is OK
fn check_configuration() -> bool {
    if env::var("JWT_SECRET").is_err() {
        error!("JWT_SECRET environment variable is mandatory");

        return false;
    }

    true
}
