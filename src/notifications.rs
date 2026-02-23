use rppal::gpio::Level;
use std::time::{Duration, Instant};

use crate::backend::Backend;

/// Defines the `Notifier` trait.
pub trait Notifier {
    /// Returns the name of the notifier, which is typically the name of the backend it uses (e.g., "slack" or "batsign").
    fn name(&self) -> &'static str;

    /// Sends a notification.
    fn send_notification(&mut self, ctx: &Context) -> NotificationResult;
}

impl<B: Backend> Notifier for TwoLevelNotifier<B> {
    /// Returns the name of the backend used by this notifier.
    fn name(&self) -> &'static str {
        self.name()
    }

    /// Sends a notification based on the current GPIO level and the configured backend, while managing timing for repeats and retries.
    fn send_notification(&mut self, ctx: &Context) -> NotificationResult {
        TwoLevelNotifier::send_notification(self, ctx)
    }
}

/// Context for sending notifications, containing the current GPIO level, timestamp, and whether it's a dry run.
pub struct Context {
    /// The current GPIO level (High or Low) that triggered the notification.
    pub level: Level,

    /// The current timestamp when the notification is being processed, used for timing logic in the notifiers.
    pub now: Instant,

    /// Indicates whether the notification should be sent as a dry run (i.e., printed to the console instead of actually sending it).
    pub dry_run: bool,
}

/// Result type for notification attempts, indicating whether it was sent, skipped, or if there was an error.
pub enum NotificationResult {
    /// Indicates that it's not yet time to send the notification based on the configured intervals.
    NotYetTime,

    /// Indicates that the notification was processed as a dry run, meaning it was printed to the console instead of being sent.
    DryRun,

    /// Indicates that the notification was successfully sent, containing the HTTP status code returned by the backend.
    Success, //(reqwest::StatusCode),

    /// Indicates that the notification failed to send, containing the HTTP status code returned by the backend.
    Failure(String), //(reqwest::StatusCode),
}

/// Internal struct to track the state of notifications for a specific GPIO level, including timing for repeats and retries.
struct LevelNotifier {
    /// The GPIO level that this notifier is responsible for (e.g., High for alarm, Low for restored).
    level: Level,

    /// The message template to use when building the notification message for this level.
    message_template: String,

    /// The timestamp of the last successful notification sent for this level, used for determining when to send the next notification based on the repeat interval.
    last_sent: Option<Instant>,

    /// The timestamp of the last failed notification attempt for this level, used for determining when to retry sending based on the retry interval.
    last_failed: Option<Instant>,

    /// The interval to wait before sending another notification for this level after a successful send, or `None` if it should only be sent once.
    repeat_interval: Option<Duration>,

    /// The interval to wait before retrying to send a notification for this level after a failure.
    retry_interval: Duration,
}

impl LevelNotifier {
    /// Creates a new `LevelNotifier` with the specified GPIO level, message template, repeat interval, and retry interval.
    fn new(
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

    /// Determines whether a notification should be sent at the current time, based on the last sent and failed timestamps, as well as the configured repeat and retry intervals.
    fn should_send_now(&self, now: Instant) -> bool {
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

    /// Records a successful notification attempt, updating the last sent timestamp and clearing any failure state.
    fn record_success(&mut self, now: Instant) {
        self.last_sent = Some(now);
        self.last_failed = None;
    }

    /// Records a failed notification attempt, updating the last failed timestamp.
    fn record_failure(&mut self, now: Instant) {
        self.last_failed = Some(now);
    }
}

/// A notifier that manages two levels of notifications (alarm and restored) using a specified backend, handling the logic for when to send notifications based on the GPIO level and timing.
pub struct TwoLevelNotifier<B: Backend> {
    /// The backend used to send notifications (e.g., Slack or Batsign).
    backend: B,

    /// The `LevelNotifier` responsible for managing notifications when the GPIO level is High (alarm state).
    alarm: LevelNotifier,

    /// The `LevelNotifier` responsible for managing notifications when the GPIO level is Low (restored state).
    restored: LevelNotifier,
}

impl<B: Backend> TwoLevelNotifier<B> {
    /// Creates a new `TwoLevelNotifier`.
    pub fn new(
        backend: B,
        repeat_interval: Option<Duration>,
        retry_interval: Duration,
        alarm_template: &str,
        restored_template: &str,
    ) -> Self {
        Self {
            backend,
            alarm: LevelNotifier::new(Level::High, alarm_template, repeat_interval, retry_interval),
            restored: LevelNotifier::new(Level::Low, restored_template, None, retry_interval),
        }
    }

    /// Returns the name of the backend used by this notifier.
    pub fn name(&self) -> &'static str {
        self.backend.name()
    }

    /// Sends a notification based on the current GPIO level and the configured backend, while managing timing for repeats and retries.
    pub fn send_notification(&mut self, ctx: &Context) -> NotificationResult {
        let ln = match ctx.level {
            Level::Low => &mut self.restored,
            Level::High => &mut self.alarm,
        };

        if !ln.should_send_now(ctx.now) {
            return NotificationResult::NotYetTime;
        }

        let msg = self
            .backend
            .build_message(ln.level, &ln.message_template, ctx);

        if ctx.dry_run {
            println!("[{}] DRY RUN:\n{}\n", self.backend.name(), msg);
            return NotificationResult::DryRun;
        }

        match self.backend.send_message(&msg, ctx) {
            Ok(()) => {
                ln.record_success(ctx.now);
                NotificationResult::Success
            }
            Err(e) => {
                eprintln!("[!] {} failed: {e}", self.backend.name());
                ln.record_failure(ctx.now);
                NotificationResult::Failure(e)
            }
        }
    }
}
