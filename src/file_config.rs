use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::{env, time};
use users::get_current_uid;

use crate::defaults;
use crate::settings::Settings;

#[derive(Clone, Serialize, Deserialize)]
pub struct GpioConfig {
    /// GPIO pin number to monitor.
    pub pin_number: Option<u8>,

    /// Poll interval for checking the GPIO pin.
    #[serde(with = "humantime_serde")]
    pub poll_interval: Option<time::Duration>,

    /// Duration the pin must be HIGH or LOW before qualifying as a valid change.
    #[serde(with = "humantime_serde")]
    pub hold: Option<time::Duration>,
}

impl Default for GpioConfig {
    /// Default values for the GPIO settings.
    fn default() -> Self {
        Self {
            pin_number: None,
            poll_interval: None,
            hold: None,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SlackConfig {
    /// Whether Slack notifications are enabled.
    pub enabled: Option<bool>,

    /// Optional Slack webhook URL for sending notifications to Slack.
    pub urls: Option<Vec<String>>,

    /// Minimum time between sending Slack notifications.
    #[serde(with = "humantime_serde")]
    pub notification_interval: Option<time::Duration>,

    /// Time to wait before retrying to send a Slack notification after a failure.
    #[serde(with = "humantime_serde")]
    pub retry_interval: Option<time::Duration>,
}

impl Default for SlackConfig {
    /// Default values for the Slack settings.
    fn default() -> Self {
        Self {
            enabled: None,
            urls: None,
            notification_interval: None,
            retry_interval: None,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BatsignConfig {
    /// Whether Batsign notifications are enabled.
    pub enabled: Option<bool>,

    /// List of URLs to send Batsign notifications to.
    pub urls: Option<Vec<String>>,

    /// Minimum time between sending Batsign notifications.
    #[serde(with = "humantime_serde")]
    pub notification_interval: Option<time::Duration>,

    /// Time to wait before retrying to send a Batsign notification after a failure.
    #[serde(with = "humantime_serde")]
    pub retry_interval: Option<time::Duration>,
}

impl Default for BatsignConfig {
    /// Default values for the Batsign settings.
    fn default() -> Self {
        Self {
            enabled: None,
            urls: None,
            notification_interval: None,
            retry_interval: None,
        }
    }
}

/// Configuration file structure, which overrides default settings and is overridden by CLI args.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FileConfig {
    /// GPIO settings loaded from the configuration file.
    pub gpio: GpioConfig,

    /// Slack settings loaded from the configuration file.
    pub slack: SlackConfig,

    /// Batsign settings loaded from the configuration file.
    pub batsign: BatsignConfig,
}

impl Default for FileConfig {
    /// Default values for the configuration file.
    fn default() -> Self {
        Self {
            gpio: GpioConfig::default(),
            slack: SlackConfig::default(),
            batsign: BatsignConfig::default(),
        }
    }
}

impl From<&Settings> for FileConfig {
    /// Converts the resolved settings into a FileConfig, which can be saved to disk. This is used when the user wants to save the current configuration.
    fn from(s: &Settings) -> Self {
        Self {
            gpio: GpioConfig {
                pin_number: Some(s.gpio.pin_number),
                poll_interval: Some(s.gpio.poll_interval),
                hold: Some(s.gpio.hold),
            },

            slack: SlackConfig {
                enabled: Some(s.slack.enabled),
                urls: Some(s.slack.urls.clone()),
                notification_interval: Some(s.slack.notification_interval),
                retry_interval: Some(s.slack.retry_interval),
            },

            batsign: BatsignConfig {
                enabled: Some(s.batsign.enabled),
                urls: Some(s.batsign.urls.clone()),
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
pub fn resolve_default_resource_directory_from_env() -> Result<PathBuf, String> {
    if let Some(path) = env::var_os("PELLX_MONITOR_RESOURCE_DIR").map(PathBuf::from) {
        return Ok(path);
    }

    if get_current_uid() == 0 {
        return Ok(PathBuf::from("/etc/pellx_monitor"));
    }

    if let Some(path) = env::var_os("XDG_CONFIG_HOME").map(PathBuf::from) {
        return Ok(path.join(defaults::PROGRAM_ARG0));
    }

    if let Some(path) = env::var_os("HOME").map(PathBuf::from) {
        return Ok(path.join(".config").join(defaults::PROGRAM_ARG0));
    }

    Err("could not resolve default resource directory from environment variables".to_string())
}
