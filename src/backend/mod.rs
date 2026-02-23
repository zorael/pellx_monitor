pub mod batsign;
pub mod slack;

use reqwest::blocking::Client;
use rppal::gpio::Level;

pub trait Backend {
    fn name(&self) -> &'static str;

    fn build_message(&self, level: Level, template: &str) -> String;

    fn send_via_backend(
        &self,
        client: &Client,
        url: &str,
        message: String,
    ) -> Result<reqwest::StatusCode, reqwest::Error>;
}
