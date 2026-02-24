use rppal::gpio::Level;
use std::time::{Duration, Instant};

/// Internal struct to track the state of notifications for a specific GPIO level,
/// including timing for repeats and retries.
pub struct LevelNotifier {
    /// The GPIO level that this notifier is responsible for
    /// (e.g., High for alarm, Low for restored).
    pub level: Level,

    /// The message template to use when building the notification message for this level.
    pub message_template: String,

    /// The timestamp of the last successful notification sent for this level,
    /// used for determining when to send the next notification based on the repeat interval.
    last_sent: Option<Instant>,

    /// The timestamp of the last failed notification attempt for this level,
    /// used for determining when to retry sending based on the retry interval.
    last_failed: Option<Instant>,

    /// The interval to wait before sending another notification for this level
    /// after a successful send, or `None` if it should only be sent once.
    repeat_interval: Option<Duration>,

    /// The interval to wait before retrying to send a notification for this level after a failure.
    retry_interval: Duration,
}

impl LevelNotifier {
    /// Creates a new `LevelNotifier`.
    pub fn new(
        level: Level,
        message_template: &str,
        repeat: Option<Duration>,
        retry: Duration,
    ) -> Self {
        Self {
            level,
            message_template: message_template.to_string(),
            last_sent: None,
            last_failed: None,
            repeat_interval: repeat,
            retry_interval: retry,
        }
    }

    /// Determines whether a notification should be sent at the current time,
    /// based on the last sent and failed timestamps, as well as the configured
    /// repeat and retry intervals.
    pub fn should_send_now(&self, now: Instant) -> bool {
        if let Some(t) = self.last_failed
            && now.duration_since(t) < self.retry_interval
        {
            return false;
        }

        match (self.last_sent, self.repeat_interval) {
            (None, _) => true,
            (Some(_), None) => false,
            (Some(t), Some(iv)) => now.duration_since(t) >= iv,
        }
    }

    /// Records a successful notification attempt, updating the last
    /// sent timestamp and clearing any failure state.
    pub fn record_success(&mut self, now: Instant) {
        self.last_sent = Some(now);
        self.last_failed = None;
    }

    /// Records a failed notification attempt, updating the last failed timestamp.
    pub fn record_failure(&mut self, now: Instant) {
        self.last_failed = Some(now);
    }
}
