use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::{env, fs, io, time};

use crate::defaults;
use crate::settings::Settings;

/// Configuration file structure, which overrides default settings and is overridden by CLI args.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FileConfig {
    /// GPIO pin number to monitor.
    pub pin_number: Option<u8>,

    /// Poll interval for checking the GPIO pin.
    #[serde(with = "humantime_serde")]
    pub poll_interval: Option<time::Duration>,

    /// Duration the pin must be HIGH or LOW before qualifying as a valid change.
    #[serde(with = "humantime_serde")]
    pub hold: Option<time::Duration>,

    /// Optional Slack webhook URL for sending notifications to Slack.
    pub slack_webhook_url: Option<String>,

    pub time_between_slack_notifications: Option<time::Duration>,

    pub time_between_slack_notification_retries: Option<time::Duration>,

    /// Minimum time between sending notifications.
    #[serde(with = "humantime_serde")]
    pub time_between_batsigns: Option<time::Duration>,

    /// Time to wait before retrying to send a notification after a failure.
    #[serde(with = "humantime_serde")]
    pub time_between_batsign_retries: Option<time::Duration>,
}

impl Default for FileConfig {
    /// Default values for the configuration file, required by confy but not used directly.
    fn default() -> Self {
        Self {
            pin_number: None,
            poll_interval: None,
            hold: None,
            slack_webhook_url: None,
            time_between_slack_notifications: None,
            time_between_slack_notification_retries: None,
            time_between_batsigns: None,
            time_between_batsign_retries: None,
        }
    }
}

impl From<&Settings> for FileConfig {
    /// Converts the resolved settings into a FileConfig, which can be saved to disk. This is used when the user wants to save the current configuration.
    fn from(s: &Settings) -> Self {
        Self {
            pin_number: Some(s.pin_number),
            poll_interval: Some(s.poll_interval),
            hold: Some(s.hold),
            slack_webhook_url: Some(s.slack_webhook_url.clone()),
            time_between_slack_notifications: Some(s.time_between_slack_notifications),
            time_between_slack_notification_retries: Some(
                s.time_between_slack_notification_retries,
            ),
            time_between_batsigns: Some(s.time_between_batsigns),
            time_between_batsign_retries: Some(s.time_between_batsign_retries),
        }
    }
}

/// Deserializes the configuration file from disk, returning an optional FileConfig. This is used to load the configuration file at startup.
pub fn deserialize_config_file(
    settings: &Settings,
) -> Result<Option<FileConfig>, confy::ConfyError> {
    let config_pathbuf = settings
        .resource_dir_pathbuf
        .join(defaults::CONFIG_FILENAME);

    match confy::load_path(config_pathbuf) {
        Ok(cfg) => Ok(Some(cfg)),
        Err(e) => Err(e),
    }
}

/// Resolves the configuration directory path, returning the directory as a string and an optional PathBuf. This is used for operations that need to know the config directory, such as saving the config file.
pub fn resolve_default_resource_directory() -> PathBuf {
    let base = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));

    base.join(defaults::PROGRAM_ARG0)
}

/// Reads a text file and returns its non-empty, non-comment lines as a vector of strings.
pub fn read_file_lines_into_vec(pathbuf: &PathBuf) -> io::Result<Vec<String>> {
    let s = fs::read_to_string(pathbuf)?;
    let v = s
        .split('\n')
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();
    Ok(v)
}
