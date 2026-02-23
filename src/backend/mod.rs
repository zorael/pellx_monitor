pub mod batsign;
pub mod slack;

use rppal::gpio::Level;

use crate::notifications::Context;

/// Backend owns everything it needs (urls, client, command, etc.).
pub trait Backend {
    /// Returns the name of the backend, which is used for logging and identification purposes.
    fn name(&self) -> &'static str;

    /// Build the backend-specific payload/body from a plain template/body.
    fn build_message(&self, level: Level, template: &str, ctx: &Context) -> String;

    /// Deliver the already-built message using backend-owned configuration.
    fn send_message(&mut self, message: &str, ctx: &Context) -> Result<(), String>;
}
