use std::time::{Duration, Instant};

use reqwest::StatusCode;
use reqwest::blocking::Client;
use rppal::gpio::Level;

use crate::settings::Settings;

#[derive(Debug)]
pub struct Context {
    pub level: Level,
    pub now: Instant,
    pub dry_run: bool,
}

pub trait Notifier {
    fn name(&self) -> &'static str;
    fn send_notification(&mut self, ctx: &Context) -> (Vec<StatusCode>, Vec<StatusCode>);
}

pub struct LevelNotifier {
    level: Level,
    message_template: String,
    last_sent: Option<Instant>,
    last_failed: Option<Instant>,
    repeat_interval: Option<Duration>,
    retry_interval: Duration,
}

impl LevelNotifier {
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

    fn should_send_now(&self, now: Instant) -> bool {
        // Check retry logic
        if let Some(last_failed) = self.last_failed
            && now.duration_since(last_failed) < self.retry_interval
        {
            return false;
        }

        if let Some(last_sent) = self.last_sent {
            if let Some(repeat_interval) = self.repeat_interval {
                if now.duration_since(last_sent) < repeat_interval {
                    return false;
                }
            } else {
                // Not supposed to repeat this type of notification
                return false;
            }
        }

        true
    }
}

pub struct SlackNotifier {
    level_notifiers: Vec<LevelNotifier>,
    client: Client,
    url: String,
}

impl SlackNotifier {
    pub fn new(
        url: &str,
        repeat_interval: Option<Duration>,
        retry_interval: Duration,
        alarm_template: &str,
        restored_template: &str,
    ) -> Self {
        Self {
            level_notifiers: vec![
                LevelNotifier::new(Level::High, alarm_template, repeat_interval, retry_interval),
                LevelNotifier::new(Level::Low, restored_template, None, retry_interval),
            ],
            client: Client::new(),
            url: url.to_string(),
        }
    }
}

impl Notifier for SlackNotifier {
    fn name(&self) -> &'static str {
        "slack"
    }

    fn send_notification(&mut self, ctx: &Context) -> (Vec<StatusCode>, Vec<StatusCode>) {
        let mut success_statuses = Vec::new();
        let mut failure_statuses = Vec::new();

        for notifier in &mut self.level_notifiers {
            if notifier.level != ctx.level || !notifier.should_send_now(ctx.now) {
                continue;
            }

            let payload = serde_json::json!({
                "text": format!("{}", notifier.message_template)
            });

            if ctx.dry_run {
                println!("Dry run: would otherwise have sent Slack notification.");
                println!("\n{}\n", payload);
                continue;
            }

            match self.client.post(&self.url).json(&payload).send() {
                Ok(res) => {
                    if res.status().is_success() {
                        println!("Slack notification sent successfully.");
                        success_statuses.push(res.status());
                        notifier.last_sent = Some(ctx.now);
                        notifier.last_failed = None;
                    } else {
                        eprintln!(
                            "[!] Failed to send Slack notification: HTTP {}",
                            res.status()
                        );
                        failure_statuses.push(res.status());
                        notifier.last_failed = Some(ctx.now);
                    }
                }
                Err(e) => {
                    eprintln!("[!] Could not reach Slack: {e}");
                    failure_statuses.push(StatusCode::INTERNAL_SERVER_ERROR);
                    notifier.last_failed = Some(ctx.now);
                }
            }
        }

        (success_statuses, failure_statuses)
    }
}

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
        let no_errors = statuses.iter().all(|s| s.is_success());

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
