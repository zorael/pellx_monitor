use crate::notify::Context;
use crate::notify::NotificationResult;

/// Defines the `Notifier` trait.
pub trait Notifier {
    /// Returns the name of the notifier, which is typically the name of the backend it uses (e.g., "slack" or "batsign").
    fn name(&self) -> String;

    /// Sends a notification.
    fn send_notification(&mut self, ctx: &Context) -> NotificationResult;
}
