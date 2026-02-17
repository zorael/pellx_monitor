use std::time::{Duration, Instant};

use crate::settings::Settings;

#[derive(Clone)]
pub struct NotificationState {
    pub previous: Option<Instant>,
    pub previous_failure: Option<Instant>,
    pub repeat_interval: Option<Duration>,
    pub retry_delay: Duration,
}

impl NotificationState {
    pub fn new(repeat_interval: Option<Duration>, retry_delay: Duration) -> Self {
        Self {
            previous: None,
            previous_failure: None,
            repeat_interval,
            retry_delay,
        }
    }

    pub fn reset(&mut self) {
        self.previous = None;
        self.previous_failure = None;
    }
}

/// Constructs a notification message body.
pub fn format_notification_message(template: &str, settings: &Settings, since: &Instant) -> String {
    template
        .replace(
            "{since}",
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

pub fn should_send(now: Instant, settings: &Settings, state: &NotificationState) -> bool {
    if let Some(then) = state.previous_failure
        && now.duration_since(then) < state.retry_delay
    {
        return false;
    }

    if let Some(then) = state.previous
        && let Some(repeat_interval) = state.repeat_interval
    {
        if now.duration_since(then) < repeat_interval {
            return false;
        }
    } else {
        return false;
    }

    if settings.debug {
        println!("...should send notification!");
    }

    true
}
