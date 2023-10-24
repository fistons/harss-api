use feed_rs::model::Feed;
use once_cell::sync::Lazy;
use scraper::Selector;

use crate::common::errors::RssParsingError;
use crate::common::errors::RssParsingError::NonOkStatus;
use crate::common::model::FoundRssChannel;

static ALTERNATE_LINK_HEADER: Lazy<Selector> = Lazy::new(|| {
    Selector::parse(r#"link[type="application/rss+xml"],link[type="application/atom+xml"]"#)
        .unwrap()
});

static CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .user_agent("rss-aggregator checker (+https://github.com/fistons/rss-aggregator)")
        .build()
        .expect("Could not build client")
});

#[tracing::instrument]
async fn download_url(url: &str) -> anyhow::Result<String> {
    let response = reqwest::get(url).await?;
    if !response.status().is_success() {
        return Err(anyhow::Error::msg(format!(
            "Couldn't fetch {}: HTTP Status {}",
            url,
            response.status().as_u16()
        )));
    }

    let url_content = response.bytes().await?;
    Ok(String::from_utf8_lossy(&url_content).to_string())
}

#[tracing::instrument(skip(content))]
pub fn look_for_rss(content: &str) -> Vec<FoundRssChannel> {
    let document = scraper::Html::parse_document(content);

    document
        .select(&ALTERNATE_LINK_HEADER)
        .filter_map(|element| {
            let url = element.value().attr("href")?;
            let title = element.value().attr("title")?;
            Some(FoundRssChannel::new(url, title))
        })
        .collect()
}

#[tracing::instrument]
pub async fn download_and_look_for_rss(url: &str) -> anyhow::Result<Vec<FoundRssChannel>> {
    let content = download_url(url).await?;
    Ok(look_for_rss(&content))
}

/// Check that the feed is correct
#[tracing::instrument]
pub async fn check_feed(url: &str) -> Result<Feed, RssParsingError> {
    let response = CLIENT.get(url).send().await?;
    if !response.status().is_success() {
        return Err(NonOkStatus(response.status().as_u16()));
    }
    let feed_content = response.bytes().await?;
    Ok(feed_rs::parser::parse(&feed_content[..])?)
}

#[cfg(test)]
mod tests {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    #[tokio::test]
    pub async fn test_find_some_rss_links() {
        let mock = MockServer::start().await;

        // Prepare the web server
        let html = r#"
                    <!DOCTYPE html>
                    <html>
                    <head>
                        <meta charset="utf-8">
                        <link rel="alternate" type="application/rss+xml" title="Pedr0.net" href="https://blog.pedr0.net/rss/" />
                        <title>Hello, world!</title>
                    </head>
                    <body>
                        <title>Hello, world!</title>
                        <h1 class="foo">Hello, <i>world!</i></h1>
                    </body>
                    </html>"#;
        let response = ResponseTemplate::new(200).set_body_string(html);
        Mock::given(method("GET"))
            .and(path("/coucou"))
            .respond_with(response)
            .expect(1)
            .mount(&mock)
            .await;

        let url = format!("{}/coucou", mock.uri());

        assert_eq!(
            download_and_look_for_rss(&url).await.unwrap(),
            vec![FoundRssChannel::new(
                "https://blog.pedr0.net/rss/",
                "Pedr0.net"
            )]
        );
    }

    #[tokio::test]
    pub async fn test_find_some_atom_links() {
        let mock = MockServer::start().await;

        // Prepare the web server
        let html = r#"
                    <!DOCTYPE html>
                    <html>
                    <head>
                        <meta charset="utf-8">
                        <link rel="alternate" type="application/atom+xml" title="Pedr0.net" href="https://blog.pedr0.net/rss/" />
                        <title>Hello, world!</title>
                    </head>
                    <body>
                        <h1 class="foo">Hello, <i>world!</i></h1>
                    </body>
                    </html>"#;
        let response = ResponseTemplate::new(200).set_body_string(html);
        Mock::given(method("GET"))
            .and(path("/coucou"))
            .respond_with(response)
            .expect(1)
            .mount(&mock)
            .await;

        let url = format!("{}/coucou", mock.uri());

        assert_eq!(
            download_and_look_for_rss(&url).await.unwrap(),
            vec![FoundRssChannel::new(
                "https://blog.pedr0.net/rss/",
                "Pedr0.net"
            )]
        );
    }

    #[tokio::test]
    pub async fn test_find_nothing() {
        let mock = MockServer::start().await;

        // Prepare the web server
        let html = r#"
                    <!DOCTYPE html>
                    <html>
                    <head>
                        <meta charset="utf-8">
                        <title>Hello, world!</title>
                    </head>
                    <body>
                        <h1 class="foo">Hello, <i>world!</i></h1>
                    </body>
                    </html>"#;
        let response = ResponseTemplate::new(200).set_body_string(html);
        Mock::given(method("GET"))
            .and(path("/coucou"))
            .respond_with(response)
            .expect(1)
            .mount(&mock)
            .await;

        let url = format!("{}/coucou", mock.uri());

        assert_eq!(download_and_look_for_rss(&url).await.unwrap(), vec![]);
    }

    #[tokio::test]
    pub async fn test_404() {
        let mock = MockServer::start().await;

        let response = ResponseTemplate::new(404);
        Mock::given(method("GET"))
            .and(path("/coucou"))
            .respond_with(response)
            .expect(1)
            .mount(&mock)
            .await;

        let url = format!("{}/coucou", mock.uri());

        assert!(matches!(download_and_look_for_rss(&url).await, Err(_)));
    }

    #[tokio::test]
    async fn test_check_feed_is_ok() {
        let mock = MockServer::start().await;

        let valid_response = r#"
        <?xml version="1.0" encoding="UTF-8" ?>
        <rss version="2.0">
        <channel>
          <title>W3Schools Home Page</title>
          <link>https://www.w3schools.com</link>
          <description>Free web building tutorials</description>
          <item>
            <title>RSS Tutorial</title>
            <link>https://www.w3schools.com/xml/xml_rss.asp</link>
            <description>New RSS tutorial on W3Schools</description>
          </item>
        </channel>
        "#;

        let response = ResponseTemplate::new(200).set_body_raw(valid_response, "application/xml");

        Mock::given(method("GET"))
            .respond_with(response)
            .expect(1)
            .mount(&mock)
            .await;

        assert!(check_feed(&mock.uri()).await.is_ok());
    }

    #[tokio::test]
    async fn test_check_feed_non_200() {
        let mock = MockServer::start().await;

        let response = ResponseTemplate::new(404);

        Mock::given(method("GET"))
            .respond_with(response)
            .expect(1)
            .mount(&mock)
            .await;

        assert!(matches!(
            check_feed(&mock.uri()).await,
            Err(RssParsingError::NonOkStatus { .. })
        ));
    }

    #[tokio::test]
    async fn test_check_feed_invalid_rss() {
        let mock = MockServer::start().await;

        let response = ResponseTemplate::new(200).set_body_raw("rss lol", "application/xml");
        Mock::given(method("GET"))
            .respond_with(response)
            .expect(1)
            .mount(&mock)
            .await;

        assert!(matches!(
            check_feed(&mock.uri()).await,
            Err(RssParsingError::ParseFeedError { .. })
        ));
    }
}
