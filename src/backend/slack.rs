use reqwest::blocking::Client;
use rppal::gpio::Level;

pub const ERROR_EMOJI: &str = ":x:";
pub const SUCCESS_EMOJI: &str = ":white_check_mark:";

pub struct SlackBackend;

impl super::Backend for SlackBackend {
    fn name(&self) -> &'static str {
        "slack"
    }

    fn build_message(&self, level: Level, template: &str) -> String {
        let emoji = match level {
            Level::High => ERROR_EMOJI,
            Level::Low => SUCCESS_EMOJI,
        };

        serde_json::json!({ "text": format!("{emoji} {template}") }).to_string()
    }

    fn send_via_backend(
        &self,
        client: &Client,
        url: &str,
        message: String,
    ) -> Result<reqwest::StatusCode, reqwest::Error> {
        let v: serde_json::Value = serde_json::from_str(&message).expect("internal slack json");
        let res = client.post(url).json(&v).send()?;
        Ok(res.status())
    }
}
