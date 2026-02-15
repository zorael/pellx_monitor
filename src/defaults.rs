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

/// Program name string.
pub const PROGRAM_NAME: &str = "PellX Monitor";

/// Configuration file name, used by confy.
pub const PROGRAM_ARG0: &str = "pellx_monitor";

/// Author string.
pub const AUTHOR: &str = "jr <zorael@gmail.com>";

/// Semantic version patch string.
pub const SEMVER_PATCH: &str = "-alpha.02";

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
pub const CONFIG_FILENAME_SANS_TOML: &str = "config";

/// Resource file name for Batsign URLs.
pub const BATSIGNS_FILENAME: &str = "batsigns.url";

/// Resource file name for the alarm message template.
pub const ALARM_TEMPLATE_FILENAME: &str = "alarm_message_template.txt";

/// Resource file name for the restored message template.
pub const RESTORED_TEMPLATE_FILENAME: &str = "restored_message_template.txt";

/// Default alarm message template.
pub const ALARM_TEMPLATE: &str =
    "Subject: PellX Alarm\nPellets burner has been in an error state for {since}.\n";

/// Default restored message template.
pub const RESTORED_TEMPLATE: &str = "Subject: PellX Restored\nPellets burner has been restored.\n";
