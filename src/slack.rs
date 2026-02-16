use reqwest::blocking::Client;

pub const SLACK_ERROR_EMOJI: &str = ":x:";
pub const SLACK_SUCCESS_EMOJI: &str = ":white_tick:";

pub fn send_slack_notification(
    client: &Client,
    slack_webhook_url: &str,
    message: &str,
    emoji: &str,
) -> Result<(), reqwest::Error> {
    let payload = serde_json::json!({
        "text": format!("{} {}", emoji, message)
    });

    client
        .post(slack_webhook_url)
        .body(payload.to_string())
        .send()?;
    Ok(())
}
