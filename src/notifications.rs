use std::sync::Arc;
use std::time::{Duration, Instant};

use reqwest::blocking::Client;
use rppal::gpio::Level;

use crate::backend;

pub trait Notifier {
    fn name(&self) -> &'static str;
    fn send_notification(&mut self, ctx: &Context) -> NotificationResult;
}

#[derive(Debug)]
pub struct Context {
    pub level: Level,
    pub now: Instant,
    pub dry_run: bool,
}

pub enum NotificationResult {
    NotYetTime,
    DryRun,
    Success(reqwest::StatusCode),
    Failure(reqwest::StatusCode),
    Error(reqwest::Error),
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

pub struct TwoLevelNotifier<B: backend::Backend> {
    backend: B,
    url: String,
    client: Arc<Client>,
    alarm: LevelNotifier,
    restored: LevelNotifier,
}

impl<B: backend::Backend> TwoLevelNotifier<B> {
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

impl<B: backend::Backend> Notifier for TwoLevelNotifier<B> {
    fn name(&self) -> &'static str {
        self.backend.name()
    }

    fn send_notification(&mut self, ctx: &Context) -> NotificationResult {
        let ln = match ctx.level {
            Level::Low => &mut self.restored,
            Level::High => &mut self.alarm,
        };

        TwoLevelNotifier::<B>::send_one(&self.backend, Arc::clone(&self.client), &self.url, ctx, ln)
    }
}
