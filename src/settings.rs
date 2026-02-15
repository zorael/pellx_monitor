use serde::Serialize;
use std::path::PathBuf;
use std::time::Duration;
use std::{fs, io};

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
    pub alarm_template_body: String,

    /// Path to the Batsign restored message template file.
    pub restored_template_body: String,

    /// Path to the resource directory, which contains the configuration file and other resources.
    pub resource_dir_pathbuf: PathBuf,

    /// Path to the Batsign URLs file, resolved at runtime.
    pub batsign_urls_pathbuf: PathBuf,

    /// Path to the configuration file, resolved at runtime.
    pub config_file_pathbuf: PathBuf,

    /// Path to the alarm message template file, resolved at runtime.
    pub alarm_template_pathbuf: PathBuf,

    /// Path to the restored message template file, resolved at runtime.
    pub restored_template_pathbuf: PathBuf,

    /// If true, the program will not send any Batsign notifications and will only print what it would do.
    pub dry_run: bool,

    /// If true, the program will print additional debug information.
    pub debug: bool,
}

impl Default for Settings {
    /// Default values for settings, used as a base for applying config file and CLI overrides.
    fn default() -> Self {
        Self {
            pin_number: defaults::DEFAULT_PIN,
            poll_interval: defaults::DEFAULT_POLL_INTERVAL,
            hold: defaults::DEFAULT_HOLD,
            time_between_batsigns: defaults::DEFAULT_TIME_BETWEEN_BATSIGNS,
            time_between_batsigns_retry: defaults::DEFAULT_TIME_BETWEEN_BATSIGNS_RETRY,
            batsign_urls: Vec::new(),
            alarm_template_body: String::from(defaults::ALARM_TEMPLATE),
            restored_template_body: String::from(defaults::RESTORED_TEMPLATE),
            resource_dir_pathbuf: PathBuf::new(),
            config_file_pathbuf: PathBuf::new(),
            batsign_urls_pathbuf: PathBuf::new(),
            alarm_template_pathbuf: PathBuf::new(),
            restored_template_pathbuf: PathBuf::new(),
            dry_run: false,
            debug: false,
        }
    }
}

impl Settings {
    /// Applies the resource directory setting, resolving the resource paths based on the provided directory or the default. This is used to set up the resource paths before loading resources from disk.
    pub fn with_resource_dir(mut self, resource_dir: &Option<String>) -> Self {
        match resource_dir {
            Some(dir) => self.resource_dir_pathbuf = PathBuf::from(dir),
            None => self.resource_dir_pathbuf = config::resolve_default_resource_directory(),
        }

        self
    }

    /// Sanity check settings, returning a list of errors if any are found.
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
        println!(
            "Resource directory:           {:?}",
            self.resource_dir_pathbuf
        );
    }

    /// Resolves the resource paths based on the resource directory. This is used to set up the resource paths before loading resources from disk.
    pub fn resolve_resource_paths(&mut self) {
        self.config_file_pathbuf = self.resource_dir_pathbuf.join(defaults::CONFIG_FILENAME);
        self.batsign_urls_pathbuf = self.resource_dir_pathbuf.join(defaults::BATSIGNS_FILENAME);
        self.alarm_template_pathbuf = self
            .resource_dir_pathbuf
            .join(defaults::ALARM_TEMPLATE_FILENAME);
        self.restored_template_pathbuf = self
            .resource_dir_pathbuf
            .join(defaults::RESTORED_TEMPLATE_FILENAME);
    }

    /// Loads the Batsign URLs and message templates from disk, returning an error if any of the files cannot be read. This is used to load the resources after resolving the resource paths.
    pub fn load_resources_from_disk(&mut self) -> io::Result<()> {
        self.batsign_urls = config::read_file_lines_into_vec(&self.batsign_urls_pathbuf)?;
        self.alarm_template_body = fs::read_to_string(&self.alarm_template_pathbuf)?;
        self.restored_template_body = fs::read_to_string(&self.restored_template_pathbuf)?;
        Ok(())
    }
}

/// Applies config file settings to the default settings, returning the resulting settings.
pub fn apply_file(mut s: Settings, file: &Option<config::FileConfig>) -> Settings {
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
