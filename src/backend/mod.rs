pub mod batsign;
pub mod slack;

use reqwest::blocking::Client;
use rppal::gpio::Level;

/// Defines the `Backend` trait, which represents a notification backend that can build messages and send notifications based on the GPIO level and a message template.
pub trait Backend {
    /// Returns the name of the backend, which is used for logging and identification purposes.
    fn name(&self) -> &'static str;

    /// Builds the message to be sent via the backend based on the GPIO level and the provided template.
    fn build_message(&self, level: Level, template: &str) -> String;

    /// Sends a notification via the backend by making a request to the specified URL with the provided message, returning the HTTP status code or an error if the request fails.
    fn send_via_backend(
        &self,
        client: &Client,
        url: &str,
        message: String,
    ) -> Result<reqwest::StatusCode, reqwest::Error>;
}
