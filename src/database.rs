use std::time::Duration;

use r2d2::Pool;
use redis::Client;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};

pub type RedisPool = Pool<Client>;

pub async fn init_postgres_connection() -> DatabaseConnection {
    let connection_spec =
        std::env::var("DATABASE_URL").expect("DATABASE_URL env variable should be set");

    let mut opt = ConnectOptions::new(connection_spec.to_owned());
    opt.min_connections(5)
        .max_connections(10)
        .connect_timeout(Duration::from_secs(8))
        .idle_timeout(Duration::from_secs(8))
        .max_lifetime(Duration::from_secs(8));

    Database::connect(opt)
        .await
        .expect("Could not connect to postgres")
}

pub fn init_redis_connection() -> RedisPool {
    let url = std::env::var("REDIS_URL").unwrap_or_else(|_| String::from("redis://127.0.0.1"));
    let client = Client::open(url).expect("Could not connect to redis");

    Pool::builder()
        .max_size(15)
        .build(client)
        .unwrap_or_else(|e| panic!("Error building redis pool: {}", e))
}
