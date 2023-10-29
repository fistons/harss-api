use serde_json::{json, Value};
use std::env;

pub type Email = String;

/// Send a reset password email.
pub async fn send_reset_password_email(dest_email: &Email, dest_name: &str, token: &str) {
    let client = reqwest::Client::new();
    client
        .post(env::var("SCW_EMAIL_ENDPOINT").unwrap())
        .header("X-Auth-Token", env::var("SCW_API_KEY").unwrap())
        .json(&build_request_body(dest_name, dest_email, token))
        .send()
        .await
        .unwrap();
}

fn build_request_body(dest_name: &str, dest_email: &Email, token: &str) -> Value {
    let email_sender_name = env::var("EMAIL_SERMDER_NAME").unwrap();
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
            "subject": "Hello from rust",
            "text": format!("Hello {dest_name},\n\nHere is your token: {token}.\n\nBye."),
            "html": "Hello from <b>rust</b>, my friend",
            "project_id": project_id
        }
    )
}

#[cfg(test)]
mod tests {

    // #[tokio::test]
    // async fn test_sendemail() {
    //     std::env::set_var("SCW_EMAIL_ENDPOINT", "http://127.0.0.1:9090");
    //     super::send_reset_password_email(&String::from("eric@pedr0.net"), "Eric", "lolno").await;
    // }
}
