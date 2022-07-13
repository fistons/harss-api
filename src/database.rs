use std::time::Duration;

use sea_orm::{ConnectOptions, Database, DatabaseConnection};

pub async fn init_database() -> DatabaseConnection {
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
