use dotenvy::dotenv;

use common::observability;
use fetcher::process;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let subscriber = observability::get_subscriber("rss_aggregator-fetcher", "info");
    observability::init_subscriber(subscriber);

    let _sentry_guard = observability::init_sentry();

    let db = common::init_postgres_connection().await;

    tracing::info!("Running RSS fetcher");
    if let Err(err) = process(&db).await {
        tracing::error!("Ho noes! {err}");
    }
    tracing::info!("RSS fetcher done!");
}
