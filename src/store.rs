pub struct RefreshTokenStore(pub std::sync::Mutex<redis::Connection>);

impl Default for RefreshTokenStore {
    fn default() -> Self {
        RefreshTokenStore::new(
            std::env::var("REDIS_URL").unwrap_or_else(|_| String::from("redis://127.0.0.1")),
        )
    }
}

impl RefreshTokenStore {
    fn new(url: String) -> Self {
        let connection = redis::Client::open(url).unwrap().get_connection().unwrap();

        Self(std::sync::Mutex::new(connection))
    }
}
