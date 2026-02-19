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

impl GpioSettings {
    /// Applies GPIO settings from the config file, overriding the default settings where specified.
    fn apply_file(&mut self, gpio_config: &file_config::GpioConfig) {
        if let Some(pin_number) = gpio_config.pin_number {
            self.pin_number = pin_number;
        }

        if let Some(poll_interval) = gpio_config.poll_interval {
            self.poll_interval = poll_interval;
        }

        if let Some(hold) = gpio_config.hold {
            self.hold = hold;
        }
    }

    /// Sanity check settings, returning a list of errors if any are found.
    fn sanity_check(&self, vec: &mut Vec<String>) {
        const MAX_GPIO_PIN: u8 = 27;

        if self.pin_number > MAX_GPIO_PIN {
            vec.push(format!(
                "Invalid GPIO pin number: {}. Must be between 0 and {}.",
                self.pin_number, MAX_GPIO_PIN
            ));
        }

        if self.poll_interval == Duration::ZERO {
            vec.push("GPIO poll interval must be greater than zero.".to_string());
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
    /// Applies Slack settings from the config file, overriding the default settings where specified.
    fn apply_file(&mut self, slack_config: &file_config::SlackConfig) {
        if let Some(enabled) = slack_config.enabled {
            self.enabled = enabled;
        }

        if let Some(urls) = slack_config.urls.clone() {
            self.urls = urls;
        }

        if let Some(notification_interval) = slack_config.notification_interval {
            self.notification_interval = notification_interval;
        }

        if let Some(retry_interval) = slack_config.retry_interval {
            self.retry_interval = retry_interval;
        }
    }

    /// Sanity check the Slack settings, returning true if they are valid and false if any issues are found. This is used to validate the settings before starting the monitoring loop.
    fn sanity_check(&self, vec: &mut Vec<String>) {
        if self.notification_interval == Duration::ZERO {
            vec.push("Slack notifications interval must be non-zero.".to_string());
        }

        if self.retry_interval == Duration::ZERO {
            vec.push("Slack notification retry interval must be non-zero.".to_string());
        }

        if !self.enabled {
            return;
        }

        if self.urls.is_empty() {
            vec.push(
                "Slack notifications are enabled but no webhook URLs are configured.".to_string(),
            );
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
    /// Applies Batsign settings from the config file, overriding the default settings where specified.
    fn apply_file(&mut self, batsign_config: &file_config::BatsignConfig) {
        if let Some(enabled) = batsign_config.enabled {
            self.enabled = enabled;
        }

        if let Some(urls) = batsign_config.urls.clone() {
            self.urls = urls;
        }

        if let Some(notification_interval) = batsign_config.notification_interval {
            self.notification_interval = notification_interval;
        }

        if let Some(retry_interval) = batsign_config.retry_interval {
            self.retry_interval = retry_interval;
        }
    }

    /// Sanity check the Batsign settings, returning true if they are valid and false if any issues are found. This is used to validate the settings before starting the monitoring loop.
    fn sanity_check(&self, vec: &mut Vec<String>) {
        if self.notification_interval == Duration::ZERO {
            vec.push("Batsign notifications interval must be non-zero.".to_string());
        }

        if self.retry_interval == Duration::ZERO {
            vec.push("Batsign notification retry interval must be non-zero.".to_string());
        }

        if !self.enabled {
            return;
        }

        if self.urls.is_empty() {
            vec.push("Batsign notifications are enabled but no URLs are configured.".to_string());
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
    /// Path to the configuration directory, which contains the configuration file and other resources.
    pub config_dir: PathBuf,

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
            config_dir: PathBuf::new(),
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
    /// Applies the configuration directory setting, resolving the resource paths based on the provided directory or the default. This is used to set up the resource paths before loading resources from disk.
    pub fn inherit_config_dir(&mut self, config_dir: &Option<String>) -> Result<(), String> {
        if let Some(dir) = config_dir {
            self.paths.config_dir = PathBuf::from(dir);
            return Ok(());
        }

        match file_config::resolve_default_config_directory_from_env() {
            Ok(path) => {
                self.paths.config_dir = path;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Sanity check settings, returning a list of errors if any are found.
    pub fn sanity_check(&self) -> Result<(), Vec<String>> {
        let mut vec = Vec::new();

        self.gpio.sanity_check(&mut vec);
        self.slack.sanity_check(&mut vec);
        self.batsign.sanity_check(&mut vec);

        if vec.is_empty() { Ok(()) } else { Err(vec) }
    }

    /// Print the settings in a human-readable format.
    pub fn print(&self) {
        println!(
            "Using configuration directory {}",
            self.paths.config_dir.display()
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

    /// Resolves the resource paths based on the config directory. This is used to set up the resource paths before loading resources from disk.
    pub fn resolve_resource_paths(&mut self) {
        self.paths.config_file = self.paths.config_dir.join(defaults::CONFIG_FILENAME);

        self.paths.slack_alarm_template = self
            .paths
            .config_dir
            .join(defaults::slack::ALARM_MESSAGE_TEMPLATE_FILENAME);

        self.paths.slack_restored_template = self
            .paths
            .config_dir
            .join(defaults::slack::RESTORED_MESSAGE_TEMPLATE_FILENAME);

        self.paths.batsign_alarm_template = self
            .paths
            .config_dir
            .join(defaults::batsign::ALARM_MESSAGE_TEMPLATE_FILENAME);

        self.paths.batsign_restored_template = self
            .paths
            .config_dir
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
    pub fn apply_file(&mut self, file_config: &Option<file_config::FileConfig>) {
        let Some(file_config) = file_config else {
            return;
        };

        self.gpio.apply_file(&file_config.gpio);
        self.slack.apply_file(&file_config.slack);
        self.batsign.apply_file(&file_config.batsign);
    }

    /// Applies CLI settings, returning the resulting settings.
    pub fn apply_cli(&mut self, cli: &Cli) {
        // Config directory is applied separately in `inherit_config_dir` because it affects how other settings are loaded from disk.
        self.dry_run = cli.dry_run;
        self.debug = cli.debug;
    }
}

/// Reads a file into a string, trimming whitespace, and returning an error if the file cannot be read.
fn read_to_trimmed_string(path: &Path) -> io::Result<String> {
    Ok(fs::read_to_string(path)?.trim().to_string())
}
