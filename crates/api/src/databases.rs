use deadpool_redis::{Config, Pool, Runtime};

pub fn init_redis_connection() -> Pool {
    let url = std::env::var("REDIS_URL").unwrap_or_else(|_| String::from("redis://127.0.0.1"));
    let cfg = Config::from_url(url);
    cfg.create_pool(Some(Runtime::Tokio1))
        .expect("Could not connect to redis")
}
