use serde::Serialize;
use std::io;
use std::time::Duration;

use crate::cli::Cli;
use crate::config; //::{FileConfig, read_resource_file};
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

    /// Minimum time between sending notifications, to avoid spamming.
    pub time_between_batsigns: Duration,

    /// Time to wait before retrying to send a notification after a failure.
    pub time_between_batsigns_retry: Duration,

    /// List of Batsign URLs to send notifications to.
    pub batsign_urls: Vec<String>,

    /// Path to the Batsign alarm message template file.
    pub batsign_alarm_template: String,

    /// Path to the Batsign restored message template file.
    pub batsign_restored_template: String,

    pub alarm_template_filename: String,

    pub restored_template_filename: String,

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
            batsign_urls: Vec::new(),
            batsign_alarm_template: String::from(defaults::ALARM_TEMPLATE),
            batsign_restored_template: String::from(defaults::RESTORED_TEMPLATE),
            alarm_template_filename: defaults::ALARM_TEMPLATE_FILENAME.to_string(),
            restored_template_filename: defaults::RESTORED_TEMPLATE_FILENAME.to_string(),
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
            vec.push("Time between notifications must be greater than zero.".to_string());
        }

        if self.time_between_batsigns_retry == Duration::ZERO {
            vec.push("Time between notification retries must be greater than zero.".to_string());
        }

        if !self.dry_run {
            if self.batsign_urls.is_empty() {
                vec.push("At least one Batsign URL is required.".to_string());
            } else {
                for url in self.batsign_urls.iter() {
                    match url.trim() {
                        url if !url.starts_with("https://") => vec.push(format!(
                            "Batsign URL \"{url}\" does not seem to be a valid URL."
                        )),
                        _ => {}
                    }
                }
            }
        }

        if vec.is_empty() { Ok(()) } else { Err(vec) }
    }

    /// Print the settings in a human-readable format.
    pub fn print(&self) {
        println!("GPIO pin number:              {}", self.pin_number);

        println!(
            "Poll interval:                {}",
            humantime::format_duration(self.poll_interval)
        );

        println!(
            "Hold:                         {}",
            humantime::format_duration(self.hold)
        );

        println!(
            "Time between notifications:   {}",
            humantime::format_duration(self.time_between_batsigns)
        );

        println!(
            "Notification retry time:      {}",
            humantime::format_duration(self.time_between_batsigns_retry)
        );

        println!("Batsign URLs:                 {:?}", self.batsign_urls);
    }

    pub fn resolve_template_paths(&mut self) {
        self.alarm_template_filename =
            config::resolve_resource_file(defaults::ALARM_TEMPLATE_FILENAME).0;
        self.restored_template_filename =
            config::resolve_resource_file(defaults::RESTORED_TEMPLATE_FILENAME).0;
    }

    pub fn load_resources(&mut self) -> io::Result<()> {
        self.batsign_urls = config::read_batsigns_file()?;

        self.batsign_alarm_template = config::read_resource_file(&self.alarm_template_filename)?;

        self.batsign_restored_template =
            config::read_resource_file(&self.restored_template_filename)?;

        Ok(())
    }
}

/// Applies config file settings to the default settings, returning the resulting settings.
pub fn apply_file(mut s: Settings, file: Option<config::FileConfig>) -> Settings {
    let Some(file) = file else {
        return s;
    };

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

    s
}

/// Applies CLI settings to the given settings, returning the resulting settings.
pub fn apply_cli(mut s: Settings, cli: &Cli) -> Settings {
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

    s.dry_run = cli.dry_run;

    s.debug = cli.debug;

    s
}
