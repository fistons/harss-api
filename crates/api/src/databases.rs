use std::time::Duration;

use deadpool_redis::{Config, Pool, Runtime};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use tracing::log::LevelFilter::Trace;

pub async fn init_postgres_connection() -> DatabaseConnection {
    let connection_spec =
        std::env::var("DATABASE_URL").expect("DATABASE_URL env variable should be set");

    let mut opt = ConnectOptions::new(connection_spec.to_owned());
    opt.min_connections(5)
        .max_connections(10)
        .connect_timeout(Duration::from_secs(8))
        .idle_timeout(Duration::from_secs(8))
        .max_lifetime(Duration::from_secs(8))
        .sqlx_logging_level(Trace);

    Database::connect(opt)
        .await
        .expect("Could not connect to postgres")
}

pub fn init_redis_connection() -> Pool {
    let url = std::env::var("REDIS_URL").unwrap_or_else(|_| String::from("redis://127.0.0.1"));
    let cfg = Config::from_url(url);
    cfg.create_pool(Some(Runtime::Tokio1))
        .expect("Could not connect to redis")
}
