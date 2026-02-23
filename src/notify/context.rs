use rppal::gpio::Level;
use std::time::Instant;

/// Context for sending notifications, containing the current GPIO level, timestamp, and whether it's a dry run.
pub struct Context {
    /// The current GPIO level (High or Low) that triggered the notification.
    pub level: Level,

    /// The current timestamp when the notification is being processed, used for timing logic in the notifiers.
    pub now: Instant,
}
