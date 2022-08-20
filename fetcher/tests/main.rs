use reqwest::Client;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use helpers::configure_database;

mod helpers;

#[tokio::test]
async fn happy_path() {
    // Create DB and webserver
    let mock = MockServer::start().await;
    let db = configure_database(mock.uri()).await;

    // Prepare the web server
    let bytes = include_bytes!("feed.xml").to_vec();
    let response = ResponseTemplate::new(200).set_body_raw(bytes, "application/xml");
    Mock::given(method("GET"))
        .and(path("/coucou"))
        .respond_with(response)
        .expect(1)
        .mount(&mock)
        .await;

    fetcher::Fetcher::new(Client::default(), db).fetch().await;
}
