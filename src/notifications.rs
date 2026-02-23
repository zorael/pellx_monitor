use std::sync::Arc;
use std::time::{Duration, Instant};

use reqwest::blocking::Client;
use rppal::gpio::Level;

use crate::settings::Settings;

pub trait Backend {
    fn name(&self) -> &'static str;

    fn build_message(&self, level: Level, template: &str) -> String;

    fn send(
        &self,
        client: &Client,
        url: &str,
        message: String,
    ) -> Result<reqwest::StatusCode, reqwest::Error>;
}

pub struct SlackBackend;

impl Backend for SlackBackend {
    fn name(&self) -> &'static str {
        "slack"
    }

    fn build_message(&self, level: Level, template: &str) -> String {
        pub const ERROR_EMOJI: &str = ":x:";
        pub const SUCCESS_EMOJI: &str = ":white_check_mark:";

        let emoji = match level {
            Level::High => ERROR_EMOJI,
            Level::Low => SUCCESS_EMOJI,
        };

        serde_json::json!({ "text": format!("{emoji} {template}") }).to_string()
    }

    fn send(
        &self,
        client: &Client,
        url: &str,
        message: String,
    ) -> Result<reqwest::StatusCode, reqwest::Error> {
        let v: serde_json::Value = serde_json::from_str(&message).expect("internal slack json");
        let res = client.post(url).json(&v).send()?;
        Ok(res.status())
    }
}

pub struct BatsignBackend;

impl Backend for BatsignBackend {
    fn name(&self) -> &'static str {
        "batsign"
    }

    fn build_message(&self, _level: Level, template: &str) -> String {
        template.to_owned()
    }

    fn send(
        &self,
        client: &Client,
        url: &str,
        message: String,
    ) -> Result<reqwest::StatusCode, reqwest::Error> {
        let res = client.post(url).body(message).send()?;
        Ok(res.status())
    }
}

pub struct TwoLevelNotifier<B: Backend> {
    backend: B,
    url: String,
    client: Arc<Client>,
    alarm: LevelNotifier,
    restored: LevelNotifier,
}

impl<B: Backend> TwoLevelNotifier<B> {
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

    fn send_one(
        backend: &B,
        client: Arc<Client>,
        url: &str,
        ctx: &Context,
        ln: &mut LevelNotifier,
    ) -> Option<NotificationResults> {
        if !ln.should_send_now(ctx.now) {
            return None;
        }

        let message = backend.build_message(ln.level, &ln.message_template);

        if ctx.dry_run {
            println!("[{}] DRY RUN to {}:\n{}\n", backend.name(), url, message);
            println!("{:?}", ctx.elapsed);
            return None;
        }

        let mut results = NotificationResults::default();

        match backend.send(&client, url, message) {
            Ok(status) if status.is_success() => {
                results.num_succeeded += 1;
                ln.record_success(ctx.now);
            }
            Ok(status) => {
                eprintln!("[!] {} returned HTTP {}", backend.name(), status);
                results.num_failed += 1;
                ln.record_failure(ctx.now);
            }
            Err(e) => {
                eprintln!("[!] Could not reach {}: {e}", backend.name());
                results.num_failed += 1;
                ln.record_failure(ctx.now);
            }
        }

        Some(results)
    }
}

impl<B: Backend> Notifier for TwoLevelNotifier<B> {
    fn name(&self) -> &'static str {
        self.backend.name()
    }

    fn send_notification(&mut self, ctx: &Context) -> NotificationResults {
        let ln = match ctx.level {
            Level::Low => &mut self.restored,
            Level::High => &mut self.alarm,
        };

        TwoLevelNotifier::send_one(&self.backend, Arc::clone(&self.client), &self.url, ctx, ln)
            .unwrap_or_default()
    }
}

#[derive(Debug)]
pub struct Context {
    pub level: Level,
    pub now: Instant,
    pub elapsed: Duration,
    pub dry_run: bool,
}

#[derive(Default)]
pub struct NotificationResults {
    pub num_succeeded: usize,
    pub num_failed: usize,
}

pub trait Notifier {
    fn name(&self) -> &'static str;
    fn send_notification(&mut self, ctx: &Context) -> NotificationResults;
}

struct LevelNotifier {
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

    fn record_success(&mut self, now: Instant) {
        self.last_sent = Some(now);
        self.last_failed = None;
    }

    fn record_failure(&mut self, now: Instant) {
        self.last_failed = Some(now);
    }
}

/*pub struct SlackNotifier {
    alarm_notifier: LevelNotifier,
    restored_notifier: LevelNotifier,
    url: String,
    client: Arc<Client>,
}

impl SlackNotifier {
    pub fn new(
        url: &str,
        client: Arc<Client>,
        repeat_interval: Option<Duration>,
        retry_interval: Duration,
        alarm_template: &str,
        restored_template: &str,
    ) -> Self {
        Self {
            alarm_notifier: LevelNotifier::new(
                Level::High,
                alarm_template,
                repeat_interval,
                retry_interval,
            ),
            restored_notifier: LevelNotifier::new(
                Level::Low,
                restored_template,
                None,
                retry_interval,
            ),
            url: url.to_string(),
            client,
        }
    }
}

impl Notifier for SlackNotifier {
    fn name(&self) -> &'static str {
        "slack"
    }

    fn send_notification(&mut self, ctx: &Context) -> NotificationResults {
        let notifier = match ctx.level {
            Level::Low => &mut self.restored_notifier,
            Level::High => &mut self.alarm_notifier,
        };

        send_slack_notification(&self.client, &self.url, ctx, notifier).unwrap_or_default()
    }
}

fn send_slack_notification(
    client: &Client,
    url: &str,
    ctx: &Context,
    notifier: &mut LevelNotifier,
) -> Option<NotificationResults> {
    if !notifier.should_send_now(ctx.now) {
        return None;
    }

    let emoji = match notifier.level {
        Level::High => ":x:",
        Level::Low => ":white_check_mark:",
    };

    let payload = serde_json::json!({
        "text": format!("{} {}", emoji, notifier.message_template)
    });

    if ctx.dry_run {
        println!("Dry run: would otherwise have sent Slack notification.");
        println!("\n{}\n", payload);
        return None;
    }

    println!(
        "{:?} just so clippy doesn't complain about unused variables",
        ctx.elapsed
    );

    let mut results = NotificationResults::default();

    // TODO: extract update of last_{sent,failed} into a helper function
    match client.post(url).json(&payload).send() {
        Ok(res) => {
            if res.status().is_success() {
                println!("Slack notification sent successfully.");
                results.num_succeeded += 1;
                notifier.record_success(ctx.now);
            } else {
                eprintln!(
                    "[!] Failed to send Slack notification: HTTP {}",
                    res.status()
                );
                results.num_failed += 1;
                notifier.record_failure(ctx.now);
            }
        }
        Err(e) => {
            eprintln!("[!] Could not reach Slack: {e}");
            results.num_failed += 1;
            notifier.record_failure(ctx.now);
        }
    }

    Some(results)
}

pub struct BatsignNotifier {
    alarm_notifier: LevelNotifier,
    restored_notifier: LevelNotifier,
    url: String,
    client: Arc<Client>,
}

impl BatsignNotifier {
    pub fn new(
        url: &str,
        client: Arc<Client>,
        repeat_interval: Option<Duration>,
        retry_interval: Duration,
        alarm_template: &str,
        restored_template: &str,
    ) -> Self {
        Self {
            alarm_notifier: LevelNotifier::new(
                Level::High,
                alarm_template,
                repeat_interval,
                retry_interval,
            ),
            restored_notifier: LevelNotifier::new(
                Level::Low,
                restored_template,
                None,
                retry_interval,
            ),
            url: url.to_string(),
            client,
        }
    }
}

impl Notifier for BatsignNotifier {
    fn name(&self) -> &'static str {
        "batsign"
    }

    fn send_notification(&mut self, ctx: &Context) -> NotificationResults {
        let notifier = match ctx.level {
            Level::Low => &mut self.restored_notifier,
            Level::High => &mut self.alarm_notifier,
        };

        send_batsign_notification(&self.client, &self.url, ctx, notifier).unwrap_or_default()
    }
}

fn send_batsign_notification(
    client: &Client,
    url: &str,
    ctx: &Context,
    notifier: &mut LevelNotifier,
) -> Option<NotificationResults> {
    if !notifier.should_send_now(ctx.now) {
        return None;
    }

    let message = notifier.message_template.to_owned();

    if ctx.dry_run {
        println!("Dry run: would otherwise have sent Batsign notification.");
        println!("\n{}\n", message);
        return None;
    }

    println!(
        "{:?} just so clippy doesn't complain about unused variables",
        ctx.elapsed
    );

    let mut results = NotificationResults::default();

    // TODO: extract update of last_{sent,failed} into a helper function
    match client.post(url).body(message).send() {
        Ok(res) => {
            if res.status().is_success() {
                println!("Batsign notification sent successfully.");
                results.num_succeeded += 1;
                notifier.record_success(ctx.now);
            } else {
                eprintln!(
                    "[!] Failed to send Batsign notification: HTTP {}",
                    res.status()
                );
                results.num_failed += 1;
                notifier.record_failure(ctx.now);
            }
        }
        Err(e) => {
            eprintln!("[!] Could not reach Batsign: {e}");
            results.num_failed += 1;
            notifier.record_failure(ctx.now);
        }
    }

    Some(results)
}*/

/*
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
}*/

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

/*
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
*/
