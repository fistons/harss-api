use serde_json::{json, Value};
use std::env;

/// Send a reset password email.
pub async fn send_reset_password_email(
    dest_email: &String,
    dest_name: &str,
    token: &str,
) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let response = client
        .post(env::var("SCW_EMAIL_ENDPOINT").unwrap())
        .header("X-Auth-Token", env::var("SCW_API_KEY").unwrap())
        .json(&build_request_body(dest_name, dest_email, token))
        .send()
        .await?;

    tracing::debug!("Reset password email response {:?}", response);

    Ok(())
}

fn build_request_body(dest_name: &str, dest_email: &String, token: &str) -> Value {
    let email_sender_name = env::var("EMAIL_SENDER_NAME").unwrap();
    let email_sender_email = env::var("EMAIL_SENDER").unwrap();
    let project_id = env::var("SCW_PROJECT_ID").unwrap();

    json!(
        {
            "from": {
                "name": email_sender_name,
                "email": email_sender_email,
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
            "project_id": project_id
        }
    )
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
