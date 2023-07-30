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
