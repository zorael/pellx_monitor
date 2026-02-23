use reqwest::blocking::Client;
use rppal::gpio::Level;
use std::sync::Arc;

/// Defines the Slack backend for sending notifications to a Slack channel.
pub struct SlackBackend {
    /// Unique identifier for the Slack backend instance, used for logging and identification purposes.
    id: usize,

    /// HTTP client used to send requests to the Slack API.
    client: Arc<Client>,

    /// Slack webhook URL to which the notification will be sent.
    url: String,
}

impl SlackBackend {
    /// Creates a new instance of the SlackBackend with the provided HTTP client and Slack webhook URL.
    pub fn new(id: usize, client: Arc<Client>, url: &str) -> Self {
        Self {
            id,
            client,
            url: url.to_owned(),
        }
    }
}

impl super::Backend for SlackBackend {
    /// Returns the name of the backend, which is "slack" in this case.
    fn name(&self) -> String {
        // This can be cached if it turns out to be a hotspot.
        format!("slack#{}", self.id)
    }

    /// Builds the message to be sent via Slack by formatting the provided template with an appropriate emoji based on the GPIO level (error emoji for high level and success emoji for low level).
    fn build_message(&self, _level: Level, template: &str) -> String {
        serde_json::json!({ "text": format!("{template}") }).to_string()
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
