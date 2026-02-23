use std::sync::Arc;
use std::time::{Duration, Instant};

use reqwest::blocking::Client;
use rppal::gpio::Level;

use crate::backend;

/// Defines the `Notifier` trait.
pub trait Notifier {
    /// Returns the name of the notifier, which is typically the name of the backend it uses (e.g., "slack" or "batsign").
    fn name(&self) -> &'static str;

    /// Sends a notification based on the current GPIO level and the configured backend, while managing timing for repeats and retries.
    fn send_notification(&mut self, ctx: &Context) -> NotificationResult;
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
    Success(reqwest::StatusCode),

    /// Indicates that the notification failed to send, containing the HTTP status code returned by the backend.
    Failure(reqwest::StatusCode),

    /// Indicates that there was an error while trying to send the notification, containing the error returned by the reqwest client.
    Error(reqwest::Error),
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

/// Implements a one-level notifier that can be used for either the alarm or restored state, tracking its own timing for when to send notifications and when to retry after failures.
impl LevelNotifier {
    /// Creates a new `LevelNotifier` with the specified parameters.
    fn new(
        level: Level,
        message_template: &str,
        repeat_interval: Option<Duration>,
        retry_interval: Duration,
    ) -> Self {
        Self {
            level,
            message_template: message_template.to_string(),
            last_sent: None,
            last_failed: None,
            repeat_interval,
            retry_interval,
        }
    }

    /// Determines whether a notification should be sent at the current time, based on the last sent and failed timestamps, as well as the configured repeat and retry intervals.
    fn should_send_now(&self, now: Instant) -> bool {
        if let Some(last_failed) = self.last_failed
            && now.duration_since(last_failed) < self.retry_interval
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
pub struct TwoLevelNotifier<B: backend::Backend> {
    /// The backend used to send notifications (e.g., Slack or Batsign).
    backend: B,

    /// The URL to send notifications to, which is specific to the backend being used.
    url: String,

    /// The HTTP client used to send requests to the backend, shared across both levels of notifications.
    client: Arc<Client>,

    /// The `LevelNotifier` responsible for managing notifications when the GPIO level is High (alarm state).
    alarm: LevelNotifier,

    /// The `LevelNotifier` responsible for managing notifications when the GPIO level is Low (restored state).
    restored: LevelNotifier,
}

/// Implements the `Notifier` trait for `TwoLevelNotifier`, allowing it to send notifications based on the current GPIO level and the configured backend, while managing timing for repeats and retries.
impl<B: backend::Backend> TwoLevelNotifier<B> {
    /// Creates a new `TwoLevelNotifier`.
    pub fn new(
        backend: B,
        url: &str,
        client: Arc<Client>,
        repeat_interval: Option<Duration>,
        retry_interval: Duration,
        alarm_template: &str,
        restored_template: &str,
    ) -> Self {
        Self {
            backend,
            url: url.to_owned(),
            client,
            alarm: LevelNotifier::new(Level::High, alarm_template, repeat_interval, retry_interval),
            restored: LevelNotifier::new(Level::Low, restored_template, None, retry_interval),
        }
    }

    /// Internal helper function to send a notification for a specific level, handling the logic for whether to send based on timing and whether it's a dry run, and recording the result of the attempt.
    fn send_one(
        backend: &B,
        client: Arc<Client>,
        url: &str,
        ctx: &Context,
        ln: &mut LevelNotifier,
    ) -> NotificationResult {
        if !ln.should_send_now(ctx.now) {
            return NotificationResult::NotYetTime;
        }

        let message = backend.build_message(ln.level, &ln.message_template);

        if ctx.dry_run {
            println!("[{}] DRY RUN to {}:\n{}\n", backend.name(), url, message);
            return NotificationResult::DryRun;
        }

        match backend.send_via_backend(&client, url, message) {
            Ok(status) if status.is_success() => {
                ln.record_success(ctx.now);
                NotificationResult::Success(status)
            }
            Ok(status) => {
                eprintln!("[!] {} returned HTTP {}", backend.name(), status);
                ln.record_failure(ctx.now);
                NotificationResult::Failure(status)
            }
            Err(e) => {
                eprintln!("[!] Could not reach {}: {e}", backend.name());
                ln.record_failure(ctx.now);
                NotificationResult::Error(e)
            }
        }
    }
}

/// Implements the `Notifier` trait for `TwoLevelNotifier`, allowing it to send notifications based on the current GPIO level and the configured backend, while managing timing for repeats and retries.
impl<B: backend::Backend> Notifier for TwoLevelNotifier<B> {
    /// Returns the name of the backend used by this notifier.
    fn name(&self) -> &'static str {
        self.backend.name()
    }

    /// Sends a notification based on the current GPIO level and the configured backend, while managing timing for repeats and retries. It determines which level notifier to use (alarm or restored) based on the GPIO level in the context, and then calls the internal `send_one` function to handle the sending logic.
    fn send_notification(&mut self, ctx: &Context) -> NotificationResult {
        let ln = match ctx.level {
            Level::Low => &mut self.restored,
            Level::High => &mut self.alarm,
        };

        TwoLevelNotifier::<B>::send_one(&self.backend, Arc::clone(&self.client), &self.url, ctx, ln)
    }
}
