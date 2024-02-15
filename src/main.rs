use std::env;
use std::net::TcpListener;

use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{error, info};

use harss_api::common::{init_postgres_connection, init_redis_connection};
use harss_api::services;
use harss_api::startup;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Init dotenv
    dotenvy::dotenv().ok();

    let subscriber = harss_api::common::observability::get_subscriber("harss-api", "info");
    harss_api::common::observability::init_subscriber(subscriber);

    let postgres_connection = init_postgres_connection().await;
    let redis_pool = init_redis_connection();

    let listener = TcpListener::bind(
        env::var("RSS_AGGREGATOR_LISTEN_ON").unwrap_or_else(|_| String::from("0.0.0.0:8080")),
    )?;

    if !check_configuration() {
        panic!("You need to check your configuration")
    }

    let postgres_connection_clone = postgres_connection.clone();
    let redis_pool_clone = redis_pool.clone();

    // Init scheduler
    let sched = JobScheduler::new().await.unwrap();
    let schedule = env::var("FETCH_CRON").unwrap_or("0 0 * * * *".to_owned());
    sched
        .add(
            Job::new_async(&schedule[..], move |_, _| {
                let postgres_connection = postgres_connection_clone.clone();
                let redis_pool = redis_pool_clone.clone();
                Box::pin(async move {
                    info!("Scheduled fetching in progress");
                    if let Err(e) =
                        services::fetching::process(&postgres_connection, &redis_pool).await
                    {
                        error!("Error during the scheduled fetching: {:?}", e);
                    } else {
                        info!("Scheduled fetching done");
                    }
                })
            })
            .expect("Could not add create fetching task"),
        )
        .await
        .expect("Could not schedule fetching task");
    sched.start().await.expect("Could not start scheduler");

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
