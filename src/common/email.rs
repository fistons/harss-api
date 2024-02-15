use anyhow::anyhow;
use handlebars::{DirectorySourceOptions, Handlebars};
use json_value_merge::Merge;
use once_cell::sync::Lazy;
use serde::Serialize;
use serde_json::Value;
use std::{env, error::Error, ops::Deref};
use tracing::{debug, error, instrument, warn};

static HANDLEBARS: Lazy<Handlebars> = Lazy::new(|| {
    let mut handlebars = Handlebars::new();
    let options = DirectorySourceOptions {
        tpl_extension: ".json".to_owned(),
        ..Default::default()
    };
    handlebars
        .register_templates_directory("templates/", options)
        .unwrap();
    handlebars
});

static EMAIL_PROPERTIES: Lazy<anyhow::Result<EmailApiProperties>> =
    Lazy::new(|| match EmailApiProperties::load() {
        Ok(props) => Ok(props),
        Err(err) => Err(anyhow!("Could not load Email properties: {:?}", err)),
    });

#[instrument(skip(email_content))]
pub async fn send_email<T>(template_name: &str, email_content: &T) -> anyhow::Result<()>
where
    T: Serialize,
{
    match EMAIL_PROPERTIES.deref() {
        Ok(email_properties) => {
            let mut data = serde_json::json!(email_properties);
            let content = serde_json::json!(email_content);

            data.merge(&content);

            let body: String = HANDLEBARS.render(template_name, &data)?;
            let client = reqwest::Client::new();
            let body: Value = serde_json::from_str(&body)?;
            let response = client
                .post(&email_properties.scw_email_endpoint)
                .header("X-Auth-Token", &email_properties.scw_api_key)
                .json(&body)
                .send()
                .await?;

            if !response.status().is_success() {
                error!("Transactional email API response {:?}", response.status());
                debug!("{:?}", response.text().await?)
            } else {
                debug!("Email sent");
            }
        }
        Err(e) => warn!("{e}"),
    }

    Ok(())
}

#[derive(Serialize)]
struct EmailApiProperties {
    sender_name: String,
    sender_email: String,
    project_id: String,
    scw_email_endpoint: String,
    scw_api_key: String,
    assets_path: String,
}

impl EmailApiProperties {
    pub fn load() -> Result<Self, Box<dyn Error>> {
        let sender_name = env::var("EMAIL_SENDER_NAME")?;
        let sender_email = env::var("EMAIL_SENDER")?;
        let project_id = env::var("SCW_PROJECT_ID")?;
        let scw_email_endpoint = env::var("SCW_EMAIL_ENDPOINT")?;
        let scw_api_key = env::var("SCW_API_KEY")?;
        let assets_path = env::var("ASSETS_PATH")?;

        Ok(EmailApiProperties {
            sender_name,
            sender_email,
            project_id,
            scw_email_endpoint,
            scw_api_key,
            assets_path,
        })
    }
}

#[cfg(test)]
mod tests {

    //TODO: Test this one day, by mocking the API
    // #[tokio::test]
    // async fn test_sendemail() {
    //     std::env::set_var("SCW_EMAIL_ENDPOINT", "http://127.0.0.1:9090");
    //     super::send_reset_password_email(&String::from("eric@pedr0.net"), "Eric", "lolno").await;
    // }
}
