use reqwest::blocking::Client;
use rppal::gpio::Level;
use std::sync::Arc;

pub const ERROR_EMOJI: &str = ":x:";
pub const SUCCESS_EMOJI: &str = ":white_check_mark:";

/// Defines the Slack backend for sending notifications to a Slack channel.
pub struct SlackBackend {
    /// HTTP client used to send requests to the Slack API.
    client: Arc<Client>,

    /// Slack webhook URL to which the notification will be sent.
    url: String,
}

impl SlackBackend {
    /// Creates a new instance of the SlackBackend with the provided HTTP client and Slack webhook URL.
    pub fn new(client: Arc<Client>, url: &str) -> Self {
        Self {
            client,
            url: url.to_owned(),
        }
    }
}

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
    fn send_message(&mut self, message: &str) -> Result<(), String> {
        let json: serde_json::Value = serde_json::from_str(message).expect("internal slack json");

        match self.client.post(&self.url).json(&json).send() {
            Ok(resp) if resp.status().is_success() => Ok(()),
            Ok(resp) => Err(format!("HTTP {}", resp.status())),
            Err(e) => Err(e.to_string()),
        }
    }
}
