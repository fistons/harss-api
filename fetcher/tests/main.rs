use reqwest::Client;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

use helpers::configure_database;

use crate::helpers::build_mock;

mod helpers;

#[tokio::test]
async fn test_1() {
    let mock = build_mock().await;
    let db = configure_database(mock.uri()).await;

    Mock::given(method("GET"))
        .and(path("/coucou"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&mock)
        .await;

    fetcher::fetch(Client::default(), db).await;
}
