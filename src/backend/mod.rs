pub mod batsign;
pub mod slack;

use rppal::gpio::Level;

/// Backend owns everything it needs (urls, client, command, etc.).
pub trait Backend {
    /// Returns the name of the backend, which is used for logging and identification purposes.
    fn name(&self) -> String;

    /// Build the backend-specific payload/body from a plain template/body.
    fn build_message(&self, level: Level, template: &str) -> String;

    /// Deliver the already-built message using backend-owned configuration.
    fn send_message(&mut self, message: &str) -> Result<(), String>;
}
