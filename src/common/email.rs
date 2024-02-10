use serde_json::{json, Value};
use std::{env, error::Error};
use tracing::{debug, error, warn};

pub async fn send_reset_password_email(
    dest_name: &str,
    dest_email: &str,
    token: &str,
) -> anyhow::Result<()> {
    match EmailApiProperties::load() {
        Ok(email_properties) => {
            let body = json!(
                {
                    "from": {
                        "name": &email_properties.email_sender_name,
                        "email": &email_properties.email_sender_email,
                    },
                    "to": [
                        {
                            "name": dest_name,
                            "email": dest_email,
                        }
                    ],
                    "subject": "Your reset password token",
                    "text": format!("Hello {dest_name},\n\nHere is your token: {token}.\nIt will be valid 15 minutes.\n\nBye."),
                    "html": format!("Hello {dest_name},<br/><br/>Here is your token: <b>{token}</b>.
                        <br/>It will be valid 15 minutes.<br/><br/>Bye."),
                    "project_id": &email_properties.project_id
                }
            );
            send_email(&body, &email_properties).await?;
        }

        Err(e) => warn!(?e, "Could not load email properties. No email will be sent"),
    }
    Ok(())
}

pub async fn send_confirm_email(
    dest_name: &str,
    dest_email: &String,
    token: &str,
) -> anyhow::Result<()> {
    match EmailApiProperties::load() {
        Ok(email_properties) => {
            let body = json!(
                {
                    "from": {
                        "name": &email_properties.email_sender_name,
                        "email": &email_properties.email_sender_email,
                    },
                    "to": [
                        {
                            "name": dest_name,
                            "email": dest_email,
                        }
                    ],
                    "subject": "Please confirm your email",
                    "text": format!("Hello {dest_name},\n\nHere is your token: {token}.\nIt will be valid 15 minutes.\n\nBye."),
                    "html": format!("Hello {dest_name},<br/><br/>Here is your token: <b>{token}</b>.
                        <br/>It will be valid 15 minutes.<br/><br/>Bye."),
                    "project_id": &email_properties.project_id
                }
            );
            send_email(&body, &email_properties).await?;
        }
        Err(e) => warn!(?e, "Could not load email properties. No email will be sent"),
    }

    Ok(())
}

/// Send a reset password email.
async fn send_email(
    email_content: &Value,
    email_properties: &EmailApiProperties,
) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    debug!("Email body {email_content}");
    let response = client
        .post(&email_properties.scw_email_endpoint)
        .header("X-Auth-Token", &email_properties.scw_api_key)
        .json(email_content)
        .send()
        .await?;

    if !response.status().is_success() {
        error!("Email response {:?}", response.status());
        debug!("{:?}", response.text().await?)
    } else {
        debug!("Email sent");
    }

    Ok(())
}

struct EmailApiProperties {
    email_sender_name: String,
    email_sender_email: String,
    project_id: String,
    scw_email_endpoint: String,
    scw_api_key: String,
}

impl EmailApiProperties {
    pub fn load() -> Result<Self, Box<dyn Error>> {
        let email_sender_name = env::var("EMAIL_SENDER_NAME")?;
        let email_sender_email = env::var("EMAIL_SENDER")?;
        let project_id = env::var("SCW_PROJECT_ID")?;
        let scw_email_endpoint = env::var("SCW_EMAIL_ENDPOINT")?;
        let scw_api_key = env::var("SCW_API_KEY")?;

        Ok(EmailApiProperties {
            email_sender_name,
            email_sender_email,
            project_id,
            scw_email_endpoint,
            scw_api_key,
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
