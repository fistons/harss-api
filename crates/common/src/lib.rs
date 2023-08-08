use deadpool_redis::{Config, Pool as Redis, Runtime};
use sqlx::postgres::PgPoolOptions;
pub use sqlx::Error as DbError;
pub use sqlx::PgPool as Pool;

pub mod channels;
pub mod errors;
pub mod items;
pub mod model;
pub mod observability;
pub mod password;
pub mod rss;
pub mod users;

/// Build the Postgres connection
pub async fn init_postgres_connection() -> Pool {
    let connection_spec =
        std::env::var("DATABASE_URL").expect("DATABASE_URL env variable should be set");

    PgPoolOptions::new()
        .max_connections(5)
        .connect(&connection_spec)
        .await
        .expect("Could not connect to postgres")
}

pub fn init_redis_connection() -> Redis {
    let url = std::env::var("REDIS_URL").unwrap_or_else(|_| String::from("redis://127.0.0.1"));
    let cfg = Config::from_url(url);
    cfg.create_pool(Some(Runtime::Tokio1))
        .expect("Could not connect to redis")
}
