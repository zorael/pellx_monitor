use std::time::Duration;

/// Default GPIO pin to use.
pub const DEFAULT_PIN: u8 = 24;  // GPIO24, physical pin 18 on Raspberry Pi

/// Default poll interval for checking the GPIO pin.
pub const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(1);

/// Default duration the pin must be HIGH before qualifying as an alarm.
pub const DEFAULT_QUALIFY_HIGH: Duration = Duration::from_secs(10);

/// Default minimum time between sending mails.
pub const DEFAULT_TIME_BETWEEN_MAILS: Duration = Duration::from_secs(30 * 60);  // 30 min

/// Default time to wait before retrying to send a mail after a failure.
pub const DEFAULT_TIME_BETWEEN_MAILS_RETRY: Duration = Duration::from_secs(5 * 60);  // 5 min

/// Default subject line for the Batsign message.
pub const DEFAULT_SUBJECT: &'static str = "PellX Alarm";

/// Author string.
pub const AUTHOR: &str = "jr <zorael@gmail.com>";

/// Version string, automatically derived from Cargo.toml.
pub const VERSION: &str = concat!("v", env!("CARGO_PKG_VERSION"), "-alpha.01");

/// About string, shown in CLI help.
pub const ABOUT: &str = "pellX monitor\n$ git clone https://github.com/zorael/pellx_monitor";
