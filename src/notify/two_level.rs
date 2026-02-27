use rppal::gpio::Level;
use std::time::Duration;

use crate::backend::Backend;
use crate::notify::Context;
use crate::notify::LevelNotifier;
use crate::notify::NotificationResult;
use crate::notify::Notifier;

/// A notifier that manages two levels of notifications (alarm and restored)
/// using a specified backend, handling the logic for when to send notifications
/// based on the GPIO level and timing.
pub struct TwoLevelNotifier<B: Backend> {
    /// The backend used to send notifications (e.g., Slack or Batsign).
    backend: B,

    /// The `LevelNotifier` responsible for managing notifications in alarm states.
    alarm: LevelNotifier,

    /// The `LevelNotifier` responsible for managing notifications in restored states.
    restored: LevelNotifier,

    /// Indicates whether the notifier should operate in dry run mode.
    dry_run: bool,
}

impl<B: Backend> Notifier for TwoLevelNotifier<B> {
    /// Returns the name of the backend used by this notifier.
    fn name(&self) -> String {
        self.name()
    }

    /// Sends a notification based on the current GPIO level and the
    /// configured backend, while managing timing for repeats and retries.
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
    pub fn name(&self) -> String {
        self.backend.name()
    }

    /// Sends a notification based on the current GPIO level and the
    /// configured backend, while managing timing for repeats and retries.
    pub fn send_notification(&mut self, ctx: &Context) -> NotificationResult {
        let (current, other) = match ctx.level {
            Level::Low => (&mut self.restored, &mut self.alarm),
            Level::High => (&mut self.alarm, &mut self.restored),
        };

        if !current.should_send_now(ctx.now) {
            return NotificationResult::NotYetTime;
        }

        let msg = self
            .backend
            .build_message(current.level, &current.message_template);

        if self.dry_run {
            println!("[{}] DRY RUN:\n{}\n", self.backend.name(), msg);
            current.record_success(ctx.now);
            other.reset();
            return NotificationResult::DryRun;
        }

        match self.backend.send_message(&msg) {
            Ok(()) => {
                current.record_success(ctx.now);
                other.reset();
                NotificationResult::Success
            }
            Err(e) => {
                eprintln!("[!] {} failed: {e}", self.backend.name());
                current.record_failure(ctx.now);
                NotificationResult::Failure(e)
            }
        }
    }
}
