use reqwest::blocking::Client;
use rppal::gpio::Level;

pub const ERROR_EMOJI: &str = ":x:";
pub const SUCCESS_EMOJI: &str = ":white_check_mark:";

/// Defines the Slack backend for sending notifications to a Slack channel.
pub struct SlackBackend;

impl super::Backend for SlackBackend {
    /// Returns the name of the backend, which is "slack" in this case.
    fn name(&self) -> &'static str {
        "slack"
    }

    /// Builds the message to be sent via Slack by formatting the provided template with an appropriate emoji based on the GPIO level (error emoji for high level and success emoji for low level).
    fn build_message(&self, level: Level, template: &str) -> String {
        let emoji = match level {
            Level::High => ERROR_EMOJI,
            Level::Low => SUCCESS_EMOJI,
        };

        serde_json::json!({ "text": format!("{emoji} {template}") }).to_string()
    }

    /// Sends a notification via the Slack backend by making a POST request to the specified URL with the message as a JSON payload.
    fn send_via_backend(
        &self,
        client: &Client,
        url: &str,
        message: String,
    ) -> Result<reqwest::StatusCode, reqwest::Error> {
        let json: serde_json::Value = serde_json::from_str(&message).expect("internal slack json");
        let res = client.post(url).json(&json).send()?;
        Ok(res.status())
    }
}
