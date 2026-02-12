use std::path::PathBuf;
use std::time::Duration;

use crate::cli::Cli;
use crate::config::FileConfig;
use crate::defaults;

/// Application settings, including defaults and sanity checks.
#[derive(Clone)]
pub struct Settings {
    /// GPIO pin number to monitor.
    pub pin_number: u8,

    /// Poll interval for checking the GPIO pin.
    pub poll_interval: Duration,

    /// Time the GPIO pin must be high before qualifying as a valid event.
    pub qualify_high: Duration,

    /// Minimum time between sending mails, to avoid spamming.
    pub time_between_mails: Duration,

    /// Time to wait before retrying to send a mail after a failure.
    pub time_between_mails_retry: Duration,

    /// URL of the Batsign API to send notifications to.
    pub batsign_url: Option<String>,

    /// Subject to use for Batsign notifications.
    pub batsign_subject: Option<String>,

    /// Optional path to the config file used, for logging purposes.
    pub config_path: Option<PathBuf>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            pin_number: defaults::DEFAULT_PIN,
            poll_interval: defaults::DEFAULT_POLL_INTERVAL,
            qualify_high: defaults::DEFAULT_QUALIFY_HIGH,
            time_between_mails: defaults::DEFAULT_TIME_BETWEEN_MAILS,
            time_between_mails_retry: defaults::DEFAULT_TIME_BETWEEN_MAILS_RETRY,
            batsign_url: None,
            batsign_subject: Some(defaults::DEFAULT_SUBJECT.to_string()),
            config_path: None,
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

        if self.time_between_mails == Duration::ZERO {
            vec.push("Time between mails must be greater than zero.".to_string());
        }

        if self.time_between_mails_retry == Duration::ZERO {
            vec.push("Time between mails retry must be greater than zero.".to_string());
        }

        match self.batsign_url.as_deref().map(str::trim) {
            Some(url) if url.is_empty() => vec.push("Batsign URL cannot be empty.".to_string()),
            Some(url) if !url.starts_with("http://") && !url.starts_with("https://") => {
                vec.push("Batsign URL must start with http:// or https://.".to_string())
            }
            None => vec.push("Batsign URL is required.".to_string()),
            _ => {}
        }

        match self.batsign_subject.as_deref().map(str::trim) {
            Some(subject) if subject.is_empty() => {
                vec.push("Batsign subject cannot be empty.".to_string())
            }
            None => vec.push("Batsign subject is required.".to_string()),
            _ => {}
        }

        if vec.is_empty() { Ok(()) } else { Err(vec) }
    }
}

/// Applies config file settings to the default settings, returning the resulting settings.
pub fn apply_file(mut s: Settings, f: FileConfig) -> Settings {
    if let Some(pin_number) = f.pin_number {
        s.pin_number = pin_number;
    }

    if let Some(poll_interval) = f.poll_interval {
        s.poll_interval = poll_interval;
    }

    if let Some(qualify_high) = f.qualify_high {
        s.qualify_high = qualify_high;
    }

    if let Some(time_between_mails) = f.time_between_mails {
        s.time_between_mails = time_between_mails;
    }

    if let Some(time_between_mails_retry) = f.time_between_mails_retry {
        s.time_between_mails_retry = time_between_mails_retry;
    }

    if f.batsign_url.is_some() {
        s.batsign_url = f.batsign_url;
    }

    if f.batsign_subject.is_some() {
        s.batsign_subject = f.batsign_subject;
    }

    s
}

/// Applies CLI settings to the given settings, returning the resulting settings.
pub fn apply_cli(mut s: Settings, c: Cli) -> Settings {
    if let Some(pin_number) = c.pin_number {
        s.pin_number = pin_number;
    }

    if let Some(poll_interval) = c.poll_interval {
        s.poll_interval = poll_interval;
    }

    if let Some(qualify_high) = c.qualify_high {
        s.qualify_high = qualify_high;
    }

    if let Some(time_between_mails) = c.time_between_mails {
        s.time_between_mails = time_between_mails;
    }

    if let Some(time_between_mails_retry) = c.time_between_mails_retry {
        s.time_between_mails_retry = time_between_mails_retry;
    }

    if c.batsign_url.is_some() {
        s.batsign_url = c.batsign_url;
    }

    if c.batsign_subject.is_some() {
        s.batsign_subject = c.batsign_subject;
    }

    s
}
