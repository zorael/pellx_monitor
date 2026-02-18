use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::{fs, io};

use crate::cli::Cli;
use crate::config; //::{FileConfig, read_resource_file};
use crate::defaults;

#[derive(Debug, Serialize)]
pub struct GpioSettings {
    /// GPIO pin number to monitor.
    pub pin_number: u8,

    /// Poll interval for checking the GPIO pin.
    pub poll_interval: Duration,

    /// Time the GPIO pin must be HIGH or LOW before qualifying as a valid change.
    pub hold: Duration,
}

impl Default for GpioSettings {
    fn default() -> Self {
        Self {
            pin_number: defaults::gpio::PIN_NUMBER,
            poll_interval: defaults::gpio::POLL_INTERVAL,
            hold: defaults::gpio::HOLD,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SlackSettings {
    /// Optional Slack webhook URL for sending notifications to Slack.
    pub webhook_url: String,

    /// Text body of the Slack alarm message template.
    pub alarm_message_template_body: String,

    /// Text body of the Slack restored message template.
    pub restored_message_template_body: String,

    /// Minimum time between sending Slack notifications, to avoid spamming.
    pub notification_interval: Duration,

    /// Time to wait before retrying to send a Slack notification after a failure.
    pub retry_interval: Duration,
}

impl Default for SlackSettings {
    fn default() -> Self {
        Self {
            webhook_url: String::from(defaults::slack::DUMMY_WEBHOOK_URL),
            alarm_message_template_body: String::from(defaults::slack::ALARM_MESSAGE_TEMPLATE_BODY),
            restored_message_template_body: String::from(
                defaults::slack::RESTORED_MESSAGE_TEMPLATE_BODY,
            ),
            notification_interval: defaults::slack::NOTIFICATION_INTERVAL,
            retry_interval: defaults::slack::RETRY_INTERVAL,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct BatsignSettings {
    /// List of Batsign URLs to send notifications to.
    pub urls: Vec<String>,

    /// Path to the Batsign alarm message template file.
    pub alarm_message_template_body: String,

    /// Path to the Batsign restored message template file.
    pub restored_message_template_body: String,

    /// Minimum time between sending notifications, to avoid spamming.
    pub notification_interval: Duration,

    /// Time to wait before retrying to send a notification after a failure.
    pub retry_interval: Duration,
}

impl Default for BatsignSettings {
    fn default() -> Self {
        Self {
            urls: Vec::new(),
            alarm_message_template_body: String::from(
                defaults::batsign::ALARM_MESSAGE_TEMPLATE_BODY,
            ),
            restored_message_template_body: String::from(
                defaults::batsign::RESTORED_MESSAGE_TEMPLATE_BODY,
            ),
            notification_interval: defaults::batsign::NOTIFICATION_INTERVAL,
            retry_interval: defaults::batsign::RETRY_INTERVAL,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PathBufs {
    /// Path to the resource directory, which contains the configuration file and other resources.
    pub resource_dir: PathBuf,

    /// Path to the configuration file, resolved at runtime.
    pub config_file: PathBuf,

    /// Path to the Slack alarm message template file.
    pub slack_alarm_template: PathBuf,

    /// Path to the Slack restored message template file.
    pub slack_restored_template: PathBuf,

    /// Path to the Batsign URLs file, resolved at runtime.
    pub batsign_urls: PathBuf,

    /// Path to the alarm message template file, resolved at runtime.
    pub batsign_alarm_template: PathBuf,

    /// Path to the restored message template file, resolved at runtime.
    pub batsign_restored_template: PathBuf,
}

impl Default for PathBufs {
    fn default() -> Self {
        Self {
            resource_dir: PathBuf::new(),
            config_file: PathBuf::new(),
            slack_alarm_template: PathBuf::new(),
            slack_restored_template: PathBuf::new(),
            batsign_urls: PathBuf::new(),
            batsign_alarm_template: PathBuf::new(),
            batsign_restored_template: PathBuf::new(),
        }
    }
}

/// Application settings, including defaults and sanity checks.
#[derive(Debug, Serialize)]
pub struct Settings {
    /// GPIO settings.
    pub gpio: GpioSettings,

    /// Slack settings.
    pub slack: SlackSettings,

    /// Batsign settings.
    pub batsign: BatsignSettings,

    /// Paths to resources, resolved at runtime.
    pub paths: PathBufs,

    /// If true, the program will not send any Batsign notifications and will only print what it would do.
    pub dry_run: bool,

    /// If true, the program will print additional debug information.
    pub debug: bool,
}

impl Default for Settings {
    /// Default values for settings, used as a base for applying config file and CLI overrides.
    fn default() -> Self {
        Self {
            gpio: GpioSettings::default(),
            slack: SlackSettings::default(),
            batsign: BatsignSettings::default(),
            paths: PathBufs::default(),
            dry_run: false,
            debug: false,
        }
    }
}

impl Settings {
    /// Applies the resource directory setting, resolving the resource paths based on the provided directory or the default. This is used to set up the resource paths before loading resources from disk.
    pub fn with_resource_dir(mut self, resource_dir: &Option<String>) -> Self {
        match resource_dir {
            Some(dir) => self.paths.resource_dir = PathBuf::from(dir),
            None => self.paths.resource_dir = config::resolve_default_resource_directory(),
        }

        self
    }

    /// Sanity check settings, returning a list of errors if any are found.
    pub fn sanity_check(&self) -> Result<(), Vec<String>> {
        const MAX_GPIO_PIN: u8 = 27;

        let mut vec = Vec::new();

        if self.gpio.pin_number > MAX_GPIO_PIN {
            vec.push(format!(
                "Invalid GPIO pin number: {}. Must be between 0 and {}.",
                self.gpio.pin_number, MAX_GPIO_PIN
            ));
        }

        if self.gpio.poll_interval == Duration::ZERO {
            vec.push("Poll interval must be greater than zero.".to_string());
        }

        if self.batsign.notification_interval == Duration::ZERO {
            vec.push("Time between notifications must be greater than zero.".to_string());
        }

        if self.batsign.retry_interval == Duration::ZERO {
            vec.push("Time between notification retries must be greater than zero.".to_string());
        }

        if self.slack.notification_interval == Duration::ZERO {
            vec.push("Time between Slack notifications must be greater than zero.".to_string());
        }

        if self.slack.retry_interval == Duration::ZERO {
            vec.push(
                "Time between Slack notification retries must be greater than zero.".to_string(),
            );
        }

        if !self.batsign.urls.is_empty() {
            for url in self.batsign.urls.iter() {
                match url.trim() {
                    url if !url.starts_with("https://") => vec.push(format!(
                        "Batsign URL \"{url}\" does not seem to be a valid URL."
                    )),
                    _ => {}
                }
            }
        }

        if !self.slack.webhook_url.is_empty()
            && self.slack.webhook_url != defaults::slack::DUMMY_WEBHOOK_URL
            && !self.slack.webhook_url.starts_with("https://")
        {
            vec.push("Slack webhook URL does not seem to be a valid URL.".to_string());
        }

        if vec.is_empty() { Ok(()) } else { Err(vec) }
    }

    /// Print the settings in a human-readable format.
    pub fn print(&self) {
        println!("GPIO pin number:              {}", self.gpio.pin_number);

        println!(
            "Poll interval:                {}",
            humantime::format_duration(self.gpio.poll_interval)
        );

        println!(
            "Hold:                         {}",
            humantime::format_duration(self.gpio.hold)
        );

        println!(
            "Time between notifications:   {}",
            humantime::format_duration(self.batsign.notification_interval)
        );

        println!(
            "Notification retry time:      {}",
            humantime::format_duration(self.batsign.retry_interval)
        );

        println!("Batsign URLs:                 {:?}", self.batsign.urls);
        println!(
            "Resource directory:           {:?}",
            self.paths.resource_dir,
        );
    }

    /// Resolves the resource paths based on the resource directory. This is used to set up the resource paths before loading resources from disk.
    pub fn resolve_resource_paths(&mut self) {
        self.paths.config_file = self.paths.resource_dir.join(defaults::CONFIG_FILENAME);

        self.paths.slack_alarm_template = self
            .paths
            .resource_dir
            .join(defaults::slack::ALARM_MESSAGE_TEMPLATE_FILENAME);

        self.paths.slack_restored_template = self
            .paths
            .resource_dir
            .join(defaults::slack::RESTORED_MESSAGE_TEMPLATE_FILENAME);

        self.paths.batsign_urls = self
            .paths
            .resource_dir
            .join(defaults::batsign::URLS_FILENAME);

        self.paths.batsign_alarm_template = self
            .paths
            .resource_dir
            .join(defaults::batsign::ALARM_MESSAGE_TEMPLATE_FILENAME);

        self.paths.batsign_restored_template = self
            .paths
            .resource_dir
            .join(defaults::batsign::RESTORED_MESSAGE_TEMPLATE_FILENAME);
    }

    /// Loads the Batsign URLs and message templates from disk, returning an error if any of the files cannot be read. This is used to load the resources after resolving the resource paths.
    pub fn load_resources_from_disk(&mut self) -> Vec<(&PathBuf, io::Error)> {
        let mut vec = Vec::new();

        match read_to_trimmed_string(&self.paths.slack_alarm_template) {
            Ok(s) => self.slack.alarm_message_template_body = s,
            Err(e) => vec.push((&self.paths.slack_alarm_template, e)),
        };

        match read_to_trimmed_string(&self.paths.slack_restored_template) {
            Ok(s) => self.slack.restored_message_template_body = s,
            Err(e) => vec.push((&self.paths.slack_restored_template, e)),
        };

        match config::read_file_lines_into_vec(&self.paths.batsign_urls) {
            Ok(vec) => self.batsign.urls = vec,
            Err(e) => vec.push((&self.paths.batsign_urls, e)),
        };

        match read_to_trimmed_string(&self.paths.batsign_alarm_template) {
            Ok(s) => self.batsign.alarm_message_template_body = s,
            Err(e) => vec.push((&self.paths.batsign_alarm_template, e)),
        };

        match read_to_trimmed_string(&self.paths.batsign_restored_template) {
            Ok(s) => self.batsign.restored_message_template_body = s,
            Err(e) => vec.push((&self.paths.batsign_restored_template, e)),
        };

        vec
    }
}

/// Applies config file settings to the default settings, returning the resulting settings.
pub fn apply_file(mut s: Settings, file: &Option<config::FileConfig>) -> Settings {
    let Some(file) = file else {
        return s;
    };

    s.gpio.pin_number = file.gpio.pin_number;
    s.gpio.poll_interval = file.gpio.poll_interval;
    s.gpio.hold = file.gpio.hold;

    if let Some(slack_webhook_url) = &file.slack.webhook_url {
        s.slack.webhook_url = slack_webhook_url.clone();
    }

    if let Some(slack_notification_interval) = file.slack.notification_interval {
        s.slack.notification_interval = slack_notification_interval;
    }

    if let Some(slack_retry_interval) = file.slack.retry_interval {
        s.slack.retry_interval = slack_retry_interval;
    }

    if let Some(batsign_notification_interval) = file.batsign.notification_interval {
        s.batsign.notification_interval = batsign_notification_interval;
    }

    if let Some(batsign_retry_interval) = file.batsign.retry_interval {
        s.batsign.retry_interval = batsign_retry_interval;
    }

    s
}

/// Applies CLI settings to the given settings, returning the resulting settings.
pub fn apply_cli(mut s: Settings, cli: &Cli) -> Settings {
    // Resource directory is applied separately in Settings::with_resource_dir, since it needs to be applied before resolving resource paths and loading resources from disk.
    /*if let Some(resource_dir) = &cli.resource_dir {
        s.paths.resource_dir = PathBuf::from(resource_dir);
    };*/

    s.dry_run = cli.dry_run;
    s.debug = cli.debug;
    s
}

/// Reads a file into a string, trimming whitespace, and returning an error if the file cannot be read.
fn read_to_trimmed_string(path: &Path) -> io::Result<String> {
    Ok(fs::read_to_string(path)?.trim().to_string())
}
