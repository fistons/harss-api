use std::sync::Mutex;

pub mod database;
pub mod errors;
pub mod model;
pub mod observability;
pub mod routes;
pub mod services;
pub mod startup;

pub struct RefreshTokenStore(pub Mutex<redis::Connection>);

impl Default for RefreshTokenStore {
    fn default() -> Self {
        let connection = redis::Client::open(
            std::env::var("REDIS_URL").unwrap_or_else(|_| String::from("redis://127.0.0.1")),
        )
        .unwrap()
        .get_connection()
        .unwrap();

        Self(Mutex::new(connection))
    }
}
