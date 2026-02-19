use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::{fs, io};

use crate::cli::Cli;
use crate::defaults;
use crate::file_config;

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
    /// Default values for the GPIO settings.
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
    /// Whether Slack notifications are enabled.
    pub enabled: bool,

    /// Optional Slack webhook URL for sending notifications to Slack.
    pub urls: Vec<String>,

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
    /// Default values for the Slack settings.
    fn default() -> Self {
        Self {
            enabled: true,
            urls: Vec::new(),
            alarm_message_template_body: String::from(defaults::slack::ALARM_MESSAGE_TEMPLATE_BODY),
            restored_message_template_body: String::from(
                defaults::slack::RESTORED_MESSAGE_TEMPLATE_BODY,
            ),
            notification_interval: defaults::slack::NOTIFICATION_INTERVAL,
            retry_interval: defaults::slack::RETRY_INTERVAL,
        }
    }
}

impl SlackSettings {
    /// Sanity check the Slack settings, returning true if they are valid and false if any issues are found. This is used to validate the settings before starting the monitoring loop.
    pub fn sanity_check(&self, vec: &mut Vec<String>) {
        if self.urls.is_empty() {
            return;
        }

        for url in self.urls.iter() {
            match url.trim() {
                url if !url.starts_with("https://") => {
                    vec.push(format!(
                        "Slack webhook URL \"{url}\" does not seem to be a valid URL."
                    ));
                }
                _ => {}
            }
        }
    }
}

#[derive(Debug, Serialize)]
pub struct BatsignSettings {
    /// Whether Batsign notifications are enabled.
    pub enabled: bool,

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
    /// Default values for the Batsign settings.
    fn default() -> Self {
        Self {
            enabled: true,
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

impl BatsignSettings {
    /// Sanity check the Batsign settings, returning true if they are valid and false if any issues are found. This is used to validate the settings before starting the monitoring loop.
    pub fn sanity_check(&self, vec: &mut Vec<String>) {
        if self.urls.is_empty() {
            return;
        }

        for url in self.urls.iter() {
            match url.trim() {
                url if !url.starts_with("https://") => {
                    vec.push(format!(
                        "Batsign URL \"{url}\" does not seem to be a valid URL."
                    ));
                }
                _ => {}
            }
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
            None => self.paths.resource_dir = file_config::resolve_default_resource_directory(),
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

        self.slack.sanity_check(&mut vec);
        self.batsign.sanity_check(&mut vec);

        if vec.is_empty() { Ok(()) } else { Err(vec) }
    }

    /// Print the settings in a human-readable format.
    pub fn print(&self) {
        println!(
            "Using resource directory {}",
            self.paths.resource_dir.display()
        );

        println!();
        println!("-- GPIO --");
        println!("Pin number                   {}", self.gpio.pin_number);
        println!(
            "Poll interval                {}",
            humantime::format_duration(self.gpio.poll_interval)
        );
        println!(
            "Hold                         {}",
            humantime::format_duration(self.gpio.hold)
        );

        println!();
        println!("-- Slack --");
        println!("Enabled                      {}", self.slack.enabled);
        println!("Webhook URLs                 {:?}", self.slack.urls);
        println!(
            "Notification interval        {}",
            humantime::format_duration(self.slack.notification_interval)
        );

        println!(
            "Notification retry interval  {}",
            humantime::format_duration(self.slack.retry_interval)
        );

        println!();
        println!("-- Batsign --");
        println!("Enabled                      {}", self.batsign.enabled);
        println!("URLs                         {:?}", self.batsign.urls);
        println!(
            "Notification interval        {}",
            humantime::format_duration(self.batsign.notification_interval)
        );
        println!(
            "Notification retry interval  {}",
            humantime::format_duration(self.batsign.retry_interval)
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

    /// Applies config file settings to the default settings, returning the resulting settings.
    pub fn apply_file(&mut self, file: &Option<file_config::FileConfig>) {
        let Some(file) = file else {
            return;
        };

        // FileConfig
        if let Some(pin_number) = file.gpio.pin_number {
            self.gpio.pin_number = pin_number;
        }

        if let Some(poll_interval) = file.gpio.poll_interval {
            self.gpio.poll_interval = poll_interval;
        }

        if let Some(hold) = file.gpio.hold {
            self.gpio.hold = hold;
        }

        // SlackConfig
        if let Some(enabled) = file.slack.enabled {
            self.slack.enabled = enabled;
        }

        if let Some(urls) = file.slack.urls.clone() {
            self.slack.urls = urls;
        }

        if let Some(slack_notification_interval) = file.slack.notification_interval {
            self.slack.notification_interval = slack_notification_interval;
        }

        if let Some(slack_retry_interval) = file.slack.retry_interval {
            self.slack.retry_interval = slack_retry_interval;
        }

        // BatsignConfig
        if let Some(enabled) = file.batsign.enabled {
            self.batsign.enabled = enabled;
        }

        if let Some(urls) = file.batsign.urls.clone() {
            self.batsign.urls = urls;
        }

        if let Some(batsign_notification_interval) = file.batsign.notification_interval {
            self.batsign.notification_interval = batsign_notification_interval;
        }

        if let Some(batsign_retry_interval) = file.batsign.retry_interval {
            self.batsign.retry_interval = batsign_retry_interval;
        }
    }

    /// Applies CLI settings, returning the resulting settings.
    pub fn apply_cli(&mut self, cli: &Cli) {
        // Resource directory is applied separately in Settings::with_resource_dir, since it needs to be applied before resolving resource paths and loading resources from disk.
        self.dry_run = cli.dry_run;
        self.debug = cli.debug;
    }
}

/// Reads a file into a string, trimming whitespace, and returning an error if the file cannot be read.
fn read_to_trimmed_string(path: &Path) -> io::Result<String> {
    Ok(fs::read_to_string(path)?.trim().to_string())
}
