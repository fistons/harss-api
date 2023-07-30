use deadpool_redis::{Config, Pool, Runtime};
use sqlx::postgres::PgPoolOptions;

use common::Pool as DbPool;

pub async fn init_postgres_connection() -> DbPool {
    let connection_spec =
        std::env::var("DATABASE_URL").expect("DATABASE_URL env variable should be set");

    PgPoolOptions::new()
        .max_connections(5)
        .connect(&connection_spec)
        .await
        .expect("Could not connect to postgres")
}

pub fn init_redis_connection() -> Pool {
    let url = std::env::var("REDIS_URL").unwrap_or_else(|_| String::from("redis://127.0.0.1"));
    let cfg = Config::from_url(url);
    cfg.create_pool(Some(Runtime::Tokio1))
        .expect("Could not connect to redis")
}
