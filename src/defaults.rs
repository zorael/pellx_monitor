use constcat::concat;
use std::time::Duration;

/// Default GPIO pin to use.
pub const DEFAULT_PIN: u8 = 24; // GPIO24, physical pin 18 on Raspberry Pi

/// Default poll interval for checking the GPIO pin.
pub const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(1);

/// Default duration the GPIO pin must be HIGH or LOW before qualifying as a valid change.
pub const DEFAULT_HOLD: Duration = Duration::from_secs(10);

/// Default minimum time between sending mails.
pub const DEFAULT_TIME_BETWEEN_BATSIGNS: Duration = Duration::from_secs(30 * 60); // 30 min

/// Default time to wait before retrying to send a mail after a failure.
pub const DEFAULT_TIME_BETWEEN_BATSIGNS_RETRY: Duration = Duration::from_secs(5 * 60); // 5 min

/// Default subject line for the Batsign message.
pub const DEFAULT_ALARM_SUBJECT: &'static str = "PellX Alarm";

/// Default message template for the Batsign message.
pub const DEFAULT_ALARM_MESSAGE_TEMPLATE: &'static str =
    "Triggered at {timestamp}. Pin has been HIGH for {duration}.";

/// Default subject line for the Batsign message when the alarm is restored.
pub const DEFAULT_RESTORED_SUBJECT: &'static str = "PellX Restored";

/// Default message template for the Batsign message when the alarm is restored.
pub const DEFAULT_RESTORED_MESSAGE_TEMPLATE: &'static str =
    "Restored at {timestamp}. Pin is now LOW.";

/// Program name string.
pub const PROGRAM_NAME: &'static str = "PellX Monitor";

/// Author string.
pub const AUTHOR: &'static str = "jr <zorael@gmail.com>";

/// Semantic version patch string.
pub const SEMVER_PATCH: &'static str = "-alpha.01";

/// Version string, automatically derived from Cargo.toml.
pub const VERSION: &'static str = concat!("v", env!("CARGO_PKG_VERSION"), SEMVER_PATCH);

/// About string, shown in CLI help.
pub const ABOUT: &'static str = concat!(
    PROGRAM_NAME,
    " ",
    VERSION,
    "\n$ git clone https://github.com/zorael/pellx_monitor"
);
