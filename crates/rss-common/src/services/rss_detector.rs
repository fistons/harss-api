use once_cell::sync::Lazy;
use scraper::Selector;

use crate::model::FoundRssChannel;

static ALTERNATE_LINK_HEADER: Lazy<Selector> =
    Lazy::new(|| Selector::parse(r#"link[type="application/rss+xml"]"#).unwrap());

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
                    <meta charset="utf-8">
                    <link rel="alternate" type="application/rss+xml" title="Pedr0.net" href="https://blog.pedr0.net/rss/" />
                    <title>Hello, world!</title>
                    <h1 class="foo">Hello, <i>world!</i></h1>"#;
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
                    <meta charset="utf-8">
                    <title>Hello, world!</title>
                    <h1 class="foo">Hello, <i>world!</i></h1>"#;
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
}
