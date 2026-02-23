use constcat::concat;

pub mod gpio {
    use std::time::Duration;

    /// GPIO pin number to monitor.
    pub const PIN_NUMBER: u8 = 24;

    /// Poll interval for checking the GPIO pin.
    pub const POLL_INTERVAL: Duration = Duration::from_secs(1);

    /// Duration the pin must be HIGH or LOW before qualifying as a valid change.
    pub const HOLD: Duration = Duration::from_secs(10);
}

pub mod slack {
    use std::time::Duration;

    /// Resource file name for the Slack alarm message template.
    pub const ALARM_MESSAGE_TEMPLATE_FILENAME: &str = "slack_alarm.txt";

    /// Resource file name for the Slack restored message template.
    pub const RESTORED_MESSAGE_TEMPLATE_FILENAME: &str = "slack_restored.txt";

    /// Default Slack alarm message template.
    pub const ALARM_MESSAGE_TEMPLATE_BODY: &str = ":x: Pellets burner is in an error state.";

    /// Default Slack restored message template.
    pub const RESTORED_MESSAGE_TEMPLATE_BODY: &str =
        ":white_check_mark: Pellets burner has been restored.";

    /// Default minimum time between sending Slack notifications.
    pub const NOTIFICATION_INTERVAL: Duration = Duration::from_secs(12 * 3600); // 12 hours

    /// Default time to wait before retrying to send a Slack notification after a failure.
    pub const RETRY_INTERVAL: Duration = Duration::from_secs(5 * 60); // 5 min
}

pub mod batsign {
    use std::time::Duration;

    /// Resource file name for the alarm message template.
    pub const ALARM_MESSAGE_TEMPLATE_FILENAME: &str = "batsign_alarm.txt";

    /// Resource file name for the restored message template.
    pub const RESTORED_MESSAGE_TEMPLATE_FILENAME: &str = "batsign_restored.txt";

    /// Default alarm message template.
    pub const ALARM_MESSAGE_TEMPLATE_BODY: &str =
        "Subject: PellX Alarm\nPellets burner is in an error state.\n";

    /// Default restored message template.
    pub const RESTORED_MESSAGE_TEMPLATE_BODY: &str =
        "Subject: PellX Restored\nPellets burner has been restored.\n";

    /// Default minimum time between sending mails.
    pub const NOTIFICATION_INTERVAL: Duration = Duration::from_secs(6 * 3600); // 6 hours

    /// Default time to wait before retrying to send a mail after a failure.
    pub const RETRY_INTERVAL: Duration = Duration::from_secs(5 * 60); // 5 min
}

/// Program name string.
pub const PROGRAM_NAME: &str = "PellX Monitor";

/// Program argument 0 string.
pub const PROGRAM_ARG0: &str = "pellx_monitor";

/// Configuration file name.
pub const CONFIG_FILENAME: &str = "config.toml";

/// Author string.
pub const AUTHOR: &str = "jr <zorael@gmail.com>";

/// Semantic version patch string.
pub const SEMVER_PATCH: &str = "-alpha.01";

/// Version string, automatically derived from Cargo.toml.
pub const VERSION: &str = concat!("v", env!("CARGO_PKG_VERSION"), SEMVER_PATCH);

/// Source repository URL.
pub const SOURCE_REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");

pub mod exit_codes {
    pub const WRONG_PLATFORM: u8 = 100;
    pub const FAILED_TO_CREATE_CONFIG_DIR: u8 = 10;
    pub const FAILED_TO_WRITE_CONFIG_FILE: u8 = 11;
    pub const FAILED_TO_WRITE_SLACK_ALARM_TEMPLATE: u8 = 12;
    pub const FAILED_TO_WRITE_SLACK_RESTORED_TEMPLATE: u8 = 13;
    pub const FAILED_TO_WRITE_BATSIGN_ALARM_TEMPLATE: u8 = 14;
    pub const FAILED_TO_WRITE_BATSIGN_RESTORED_TEMPLATE: u8 = 15;
    pub const CONFIGURATION_ERROR: u8 = 20;
    pub const NO_NOTIFIERS_CONFIGURED: u8 = 21;
    pub const FAILED_TO_INITIALISE_GPIO: u8 = 30;
    pub const FAILED_TO_SET_PIN_MODE: u8 = 31;
    pub const FAILED_TO_RESOLVE_CONFIG_DIR: u8 = 40;
    pub const CONFIG_DIR_DOES_NOT_EXIST: u8 = 41;
    pub const FAILED_TO_LOAD_RESOURCES: u8 = 42;
    pub const FAILED_TO_READ_CONFIG_FILE: u8 = 43;
    pub const CONFIG_FILE_DOES_NOT_EXIST: u8 = 44;
}
