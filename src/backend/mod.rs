pub mod batsign;
pub mod slack;

//use reqwest::blocking::Client;
use rppal::gpio::Level;

use crate::notifications::Context;

/// Backend owns everything it needs (urls, client, command, etc.).
pub trait Backend {
    fn name(&self) -> &'static str;

    /// Build the backend-specific payload/body from a plain template/body.
    fn build_message(&self, level: Level, template: &str, ctx: &Context) -> String;

    /// Deliver the already-built message using backend-owned configuration.
    fn send_message(&mut self, message: &str, ctx: &Context) -> Result<(), String>;
}

/*pub trait Backend {
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
}*/
