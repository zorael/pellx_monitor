use std::time::{Duration, Instant};

use crate::settings::Settings;

/// Module for handling notifications, including Slack and Batsign.
#[derive(Clone)]
pub struct NotificationState {
    pub previous: Option<Instant>,
    pub previous_failure: Option<Instant>,
    pub repeat_interval: Option<Duration>,
    pub retry_delay: Duration,
}

impl NotificationState {
    /// Creates a new `NotificationState` with the specified repeat interval and retry delay.
    pub fn new(repeat_interval: Option<Duration>, retry_delay: Duration) -> Self {
        Self {
            previous: None,
            previous_failure: None,
            repeat_interval,
            retry_delay,
        }
    }

    /// Resets the notification state by clearing the previous success and failure timestamps.
    pub fn reset(&mut self) {
        self.previous = None;
        self.previous_failure = None;
    }

    /// Updates the notification state based on the provided HTTP status codes. If all statuses indicate success, the previous success timestamp is updated and any previous failure is cleared. If any status indicates a failure, the previous failure timestamp is updated.
    pub fn update_based_on_statuses(&mut self, now: Instant, statuses: &[reqwest::StatusCode]) {
        let no_errors = statuses.iter().filter(|s| !s.is_success()).count() == 0;

        if no_errors {
            self.previous = Some(now);
            self.previous_failure = None;
        } else {
            self.previous_failure = Some(now);
        }
    }
}

/// Constructs a notification message body.
pub fn format_notification_message(template: &str, settings: &Settings, since: &Instant) -> String {
    template
        .replace(
            "{elapsed}",
            &humantime::format_duration(since.elapsed()).to_string(),
        )
        .replace("{pin_number}", &settings.gpio.pin_number.to_string())
        .replace(
            "{poll_interval}",
            &humantime::format_duration(settings.gpio.poll_interval).to_string(),
        )
        .replace(
            "{hold}",
            &humantime::format_duration(settings.gpio.hold).to_string(),
        )
}

/// Determines whether a notification should be sent based on the current time, settings, and notification state.
pub fn should_send_notification(now: Instant, state: &NotificationState) -> bool {
    if let Some(then) = state.previous_failure
        && now.duration_since(then) < state.retry_delay
    {
        return false;
    }

    match (state.previous, state.repeat_interval) {
        (None, _) => true,
        (Some(_), None) => false,
        (Some(then), Some(repeat_interval)) => now.duration_since(then) >= repeat_interval,
    }
}
