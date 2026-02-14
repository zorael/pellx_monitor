use constcat::concat;
use std::time::Duration;

/// Default GPIO pin to use.
pub const DEFAULT_PIN: u8 = 24; // GPIO24, physical pin 18 on Raspberry Pi

/// Default poll interval for checking the GPIO pin.
pub const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(1);

/// Default duration the GPIO pin must be HIGH or LOW before qualifying as a valid change.
pub const DEFAULT_HOLD: Duration = Duration::from_secs(10);

/// Default minimum time between sending mails.
pub const DEFAULT_TIME_BETWEEN_BATSIGNS: Duration = Duration::from_secs(6 * 3600); // 6 hours

/// Default time to wait before retrying to send a mail after a failure.
pub const DEFAULT_TIME_BETWEEN_BATSIGNS_RETRY: Duration = Duration::from_secs(5 * 60); // 5 min

/// Default subject line for the Batsign message.
pub const DEFAULT_ALARM_SUBJECT: &str = "PellX Alarm";

/// Default message template for the Batsign message.
pub const DEFAULT_ALARM_MESSAGE_TEMPLATE: &str =
    "Triggered at {timestamp}. Pin has been HIGH for {duration}.";

/// Default subject line for the Batsign message when the alarm is restored.
pub const DEFAULT_RESTORED_SUBJECT: &str = "PellX Restored";

/// Default message template for the Batsign message when the alarm is restored.
pub const DEFAULT_RESTORED_MESSAGE_TEMPLATE: &str = "Restored at {timestamp}. Pin is now LOW.";

/// Default and dummy Batsign API URL.
pub const DEFAULT_BATSIGN_URL: &str = "<your-unique-url>";

/// Program name string.
pub const PROGRAM_NAME: &str = "PellX Monitor";

/// Configuration file name, used by confy.
pub const PROGRAM_ARG0: &str = "pellx_monitor";

/// Author string.
pub const AUTHOR: &str = "jr <zorael@gmail.com>";

/// Semantic version patch string.
pub const SEMVER_PATCH: &str = "-alpha.01";

/// Version string, automatically derived from Cargo.toml.
pub const VERSION: &str = concat!("v", env!("CARGO_PKG_VERSION"), SEMVER_PATCH);

/// About string, shown in CLI help.
pub const ABOUT: &str = concat!(
    PROGRAM_NAME,
    " ",
    VERSION,
    "\n$ git clone https://github.com/zorael/pellx_monitor"
);

/// Configuration file name, used by confy.
pub const CONFIGURATION_TOML: &str = "config";
