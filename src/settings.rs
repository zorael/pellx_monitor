use serde::Serialize;
use std::time::Duration;

use crate::cli::Cli;
use crate::config::FileConfig;
use crate::defaults;

/// Application settings, including defaults and sanity checks.
#[derive(Serialize)]
pub struct Settings {
    /// GPIO pin number to monitor.
    pub pin_number: u8,

    /// Poll interval for checking the GPIO pin.
    pub poll_interval: Duration,

    /// Time the GPIO pin must be HIGH or LOW before qualifying as a valid change.
    pub hold: Duration,

    /// Minimum time between sending mails, to avoid spamming.
    pub time_between_batsigns: Duration,

    /// Time to wait before retrying to send a mail after a failure.
    pub time_between_batsigns_retry: Duration,

    /// URL of the Batsign API to send notifications to.
    pub batsign_url: Option<String>,

    /// Subject to use for Batsign alarm notifications.
    pub batsign_alarm_subject: Option<String>,

    /// Message template to use for Batsign alarm notifications.
    pub batsign_alarm_message_template: Option<String>,

    /// Subject to use for Batsign restored notifications.
    pub batsign_restored_subject: Option<String>,

    /// Message template to use for Batsign restored notifications.
    pub batsign_restored_message_template: Option<String>,

    /// If true, the program will not send any Batsign notifications and will only print what it would do.
    pub dry_run: bool,

    /// If true, the program will print additional debug information.
    pub debug: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            pin_number: defaults::DEFAULT_PIN,
            poll_interval: defaults::DEFAULT_POLL_INTERVAL,
            hold: defaults::DEFAULT_HOLD,
            time_between_batsigns: defaults::DEFAULT_TIME_BETWEEN_BATSIGNS,
            time_between_batsigns_retry: defaults::DEFAULT_TIME_BETWEEN_BATSIGNS_RETRY,
            batsign_url: Some(defaults::DEFAULT_BATSIGN_URL.to_string()),
            batsign_alarm_subject: Some(defaults::DEFAULT_ALARM_SUBJECT.to_string()),
            batsign_alarm_message_template: Some(
                defaults::DEFAULT_ALARM_MESSAGE_TEMPLATE.to_string(),
            ),
            batsign_restored_subject: Some(defaults::DEFAULT_RESTORED_SUBJECT.to_string()),
            batsign_restored_message_template: Some(
                defaults::DEFAULT_RESTORED_MESSAGE_TEMPLATE.to_string(),
            ),
            dry_run: false,
            debug: false,
        }
    }
}

/// Sanity check settings, returning a list of errors if any are found.
impl Settings {
    pub fn sanity_check(&self) -> Result<(), Vec<String>> {
        let mut vec = Vec::new();

        if self.pin_number > 27 {
            vec.push(format!(
                "Invalid GPIO pin number: {}. Must be between 0 and 27.",
                self.pin_number
            ));
        }

        if self.poll_interval == Duration::ZERO {
            vec.push("Poll interval must be greater than zero.".to_string());
        }

        if self.time_between_batsigns == Duration::ZERO {
            vec.push("Time between mails must be greater than zero.".to_string());
        }

        if self.time_between_batsigns_retry == Duration::ZERO {
            vec.push("Time between mails retry must be greater than zero.".to_string());
        }

        if !self.dry_run {
            match self.batsign_url.as_deref().map(str::trim) {
                Some("") => vec.push("Batsign URL must not be empty.".to_string()),
                Some(url) if !url.starts_with("http://") && !url.starts_with("https://") => {
                    vec.push("Batsign URL must start with http:// or https://.".to_string())
                }
                None => vec.push("Batsign URL is required.".to_string()),
                _ => {}
            }

            match self.batsign_alarm_subject.as_deref().map(str::trim) {
                Some("") => vec.push("Batsign alarm subject must not be empty.".to_string()),
                None => vec.push("Batsign alarm subject is required.".to_string()),
                _ => {}
            }

            match self.batsign_restored_subject.as_deref().map(str::trim) {
                Some("") => vec.push("Batsign restored subject must not be empty.".to_string()),
                None => vec.push("Batsign restored subject is required.".to_string()),
                _ => {}
            }

            match self
                .batsign_alarm_message_template
                .as_deref()
                .map(str::trim)
            {
                Some("") => {
                    vec.push("Batsign alarm message template must not be empty.".to_string())
                }
                None => vec.push("Batsign alarm message template is required.".to_string()),
                _ => {}
            }

            match self
                .batsign_restored_message_template
                .as_deref()
                .map(str::trim)
            {
                Some("") => {
                    vec.push("Batsign restored message template must not be empty.".to_string())
                }
                None => vec.push("Batsign restored message template is required.".to_string()),
                _ => {}
            }
        }

        if vec.is_empty() { Ok(()) } else { Err(vec) }
    }
}

/// Applies config file settings to the default settings, returning the resulting settings.
pub fn apply_file(mut s: Settings, file: Option<FileConfig>) -> Settings {
    if file.is_none() {
        return s;
    }

    let file = file.unwrap();

    if let Some(pin_number) = file.pin_number {
        s.pin_number = pin_number;
    }

    if let Some(poll_interval) = file.poll_interval {
        s.poll_interval = poll_interval;
    }

    if let Some(hold) = file.hold {
        s.hold = hold;
    }

    if let Some(time_between_batsigns) = file.time_between_batsigns {
        s.time_between_batsigns = time_between_batsigns;
    }

    if let Some(time_between_batsigns_retry) = file.time_between_batsigns_retry {
        s.time_between_batsigns_retry = time_between_batsigns_retry;
    }

    if file.batsign_url.is_some() {
        s.batsign_url = file.batsign_url;
    }

    if file.batsign_alarm_subject.is_some() {
        s.batsign_alarm_subject = file.batsign_alarm_subject;
    }

    if file.batsign_alarm_message_template.is_some() {
        s.batsign_alarm_message_template = file.batsign_alarm_message_template;
    }

    if file.batsign_restored_subject.is_some() {
        s.batsign_restored_subject = file.batsign_restored_subject;
    }

    if file.batsign_restored_message_template.is_some() {
        s.batsign_restored_message_template = file.batsign_restored_message_template;
    }

    s
}

/// Applies CLI settings to the given settings, returning the resulting settings.
pub fn apply_cli(mut s: Settings, cli: Cli) -> Settings {
    if let Some(pin_number) = cli.pin_number {
        s.pin_number = pin_number;
    }

    if let Some(poll_interval) = cli.poll_interval {
        s.poll_interval = poll_interval;
    }

    if let Some(hold) = cli.hold {
        s.hold = hold;
    }

    if let Some(time_between_batsigns) = cli.time_between_batsigns {
        s.time_between_batsigns = time_between_batsigns;
    }

    if let Some(time_between_batsigns_retry) = cli.time_between_batsigns_retry {
        s.time_between_batsigns_retry = time_between_batsigns_retry;
    }

    if cli.batsign_url.is_some() {
        s.batsign_url = cli.batsign_url;
    }

    s.dry_run = cli.dry_run;

    s.debug = cli.debug;

    s
}
