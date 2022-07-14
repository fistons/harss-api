use std::env;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::net::TcpListener;

use rss_aggregator::database::init_database;
use rss_aggregator::model::configuration::ApplicationConfiguration;
use rss_aggregator::observability::{get_subscriber, init_subscriber};
use rss_aggregator::poller::start_poller;
use rss_aggregator::startup;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Init dotenv
    dotenv::dotenv().ok();

    let subscriber = get_subscriber("rss_aggregator".into(), "info".into());
    init_subscriber(subscriber);

    let db = init_database().await;

    let configuration = load_configuration().unwrap();
    let listener = TcpListener::bind(
        env::var("RSS_AGGREGATOR_LISTEN_ON").unwrap_or_else(|_| String::from("0.0.0.0:8080")),
    )?;

    start_poller(db.clone()).await;

    startup::startup(db, configuration, listener).await
}

fn load_configuration() -> Result<ApplicationConfiguration, Box<dyn Error>> {
    let file = File::open(
        std::env::var("CONFIG_PATH").unwrap_or_else(|_| String::from("configuration.yaml")),
    )?;
    let reader = BufReader::new(file);
    let configuration = serde_yaml::from_reader(reader)?;

    Ok(configuration)
}
