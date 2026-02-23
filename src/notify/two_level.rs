use crate::backend::Backend;
use crate::notify::Context;
use crate::notify::LevelNotifier;
use crate::notify::NotificationResult;
use crate::notify::Notifier;
use rppal::gpio::Level;
use std::time::Duration;

/// A notifier that manages two levels of notifications (alarm and restored) using a specified backend, handling the logic for when to send notifications based on the GPIO level and timing.
pub struct TwoLevelNotifier<B: Backend> {
    /// The backend used to send notifications (e.g., Slack or Batsign).
    backend: B,

    /// The `LevelNotifier` responsible for managing notifications when the GPIO level is High (alarm state).
    alarm: LevelNotifier,

    /// The `LevelNotifier` responsible for managing notifications when the GPIO level is Low (restored state).
    restored: LevelNotifier,

    /// Indicates whether the notifier should operate in dry run mode, where notifications are printed to the console instead of being sent via the backend.
    dry_run: bool,
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

impl<B: Backend> TwoLevelNotifier<B> {
    /// Creates a new `TwoLevelNotifier`.
    pub fn new(
        backend: B,
        repeat_interval: Option<Duration>,
        retry_interval: Duration,
        alarm_template: &str,
        restored_template: &str,
        dry_run: bool,
    ) -> Self {
        Self {
            backend,
            alarm: LevelNotifier::new(Level::High, alarm_template, repeat_interval, retry_interval),
            restored: LevelNotifier::new(Level::Low, restored_template, None, retry_interval),
            dry_run,
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

        let msg = self.backend.build_message(ln.level, &ln.message_template);

        if self.dry_run {
            println!("[{}] DRY RUN:\n{}\n", self.backend.name(), msg);
            return NotificationResult::DryRun;
        }

        match self.backend.send_message(&msg) {
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
