use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::{env, time};

use crate::defaults;
use crate::settings::Settings;

#[derive(Clone, Serialize, Deserialize)]
pub struct GpioSettings {
    /// GPIO pin number to monitor.
    pub pin_number: u8,

    /// Poll interval for checking the GPIO pin.
    #[serde(with = "humantime_serde")]
    pub poll_interval: time::Duration,

    /// Duration the pin must be HIGH or LOW before qualifying as a valid change.
    #[serde(with = "humantime_serde")]
    pub hold: time::Duration,
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

#[derive(Clone, Serialize, Deserialize)]
pub struct SlackSettings {
    /// Whether Slack notifications are enabled.
    pub enabled: bool,

    /// Optional Slack webhook URL for sending notifications to Slack.
    pub urls: Vec<String>,

    /// Minimum time between sending Slack notifications.
    #[serde(with = "humantime_serde")]
    pub notification_interval: Option<time::Duration>,

    /// Time to wait before retrying to send a Slack notification after a failure.
    #[serde(with = "humantime_serde")]
    pub retry_interval: Option<time::Duration>,
}

impl Default for SlackSettings {
    /// Default values for the Slack settings.
    fn default() -> Self {
        Self {
            enabled: true,
            urls: Vec::new(),
            notification_interval: Some(defaults::slack::NOTIFICATION_INTERVAL),
            retry_interval: Some(defaults::slack::RETRY_INTERVAL),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BatsignSettings {
    /// Whether Batsign notifications are enabled.
    pub enabled: bool,

    /// List of URLs to send Batsign notifications to.
    pub urls: Vec<String>,

    /// Minimum time between sending Batsign notifications.
    #[serde(with = "humantime_serde")]
    pub notification_interval: Option<time::Duration>,

    /// Time to wait before retrying to send a Batsign notification after a failure.
    #[serde(with = "humantime_serde")]
    pub retry_interval: Option<time::Duration>,
}

impl Default for BatsignSettings {
    /// Default values for the Batsign settings.
    fn default() -> Self {
        Self {
            enabled: true,
            urls: Vec::new(),
            notification_interval: Some(defaults::batsign::NOTIFICATION_INTERVAL),
            retry_interval: Some(defaults::batsign::RETRY_INTERVAL),
        }
    }
}

/// Configuration file structure, which overrides default settings and is overridden by CLI args.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FileConfig {
    /// GPIO settings loaded from the configuration file.
    pub gpio: GpioSettings,

    /// Slack settings loaded from the configuration file.
    pub slack: SlackSettings,

    /// Batsign settings loaded from the configuration file.
    pub batsign: BatsignSettings,
}

impl Default for FileConfig {
    /// Default values for the configuration file.
    fn default() -> Self {
        Self {
            gpio: GpioSettings::default(),
            slack: SlackSettings::default(),
            batsign: BatsignSettings::default(),
        }
    }
}

impl From<&Settings> for FileConfig {
    /// Converts the resolved settings into a FileConfig, which can be saved to disk. This is used when the user wants to save the current configuration.
    fn from(s: &Settings) -> Self {
        Self {
            gpio: GpioSettings {
                pin_number: s.gpio.pin_number,
                poll_interval: s.gpio.poll_interval,
                hold: s.gpio.hold,
            },

            slack: SlackSettings {
                enabled: s.slack.enabled,
                urls: s.slack.urls.clone(),
                notification_interval: Some(s.slack.notification_interval),
                retry_interval: Some(s.slack.retry_interval),
            },

            batsign: BatsignSettings {
                enabled: s.batsign.enabled,
                urls: s.batsign.urls.clone(),
                notification_interval: Some(s.batsign.notification_interval),
                retry_interval: Some(s.batsign.retry_interval),
            },
        }
    }
}

/// Deserializes the configuration file from disk, returning an optional FileConfig. This is used to load the configuration file at startup.
pub fn deserialize_config_file(
    settings: &Settings,
) -> Result<Option<FileConfig>, confy::ConfyError> {
    let config_pathbuf = settings.paths.resource_dir.join(defaults::CONFIG_FILENAME);

    if !config_pathbuf.exists() {
        return Ok(None);
    }

    match confy::load_path(config_pathbuf) {
        Ok(cfg) => Ok(Some(cfg)),
        Err(e) => Err(e),
    }
}

/// Resolves the configuration directory path, returning the directory as a string and an optional PathBuf. This is used for operations that need to know the config directory, such as saving the config file.
pub fn resolve_default_resource_directory() -> PathBuf {
    if let Some(path) = env::var_os("PELLX_MONITOR_RESOURCE_DIR") {
        return PathBuf::from(path);
    }

    let base = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));

    base.join(defaults::PROGRAM_ARG0)
}
