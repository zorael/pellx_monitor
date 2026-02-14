use confy::get_configuration_file_path;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time;

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

    /// Batsign URL to send the alert to.
    pub batsign_url: Option<String>,

    /// Subject line for the Batsign alarm message.
    pub batsign_alarm_subject: Option<String>,

    /// Batsign alarm message template string.
    pub batsign_alarm_message_template: Option<String>,

    /// Subject line for the Batsign restored message.
    pub batsign_restored_subject: Option<String>,

    /// Batsign restored message template string.
    pub batsign_restored_message_template: Option<String>,

    /// Minimum time between sending notifications.
    #[serde(with = "humantime_serde")]
    pub time_between_batsigns: Option<time::Duration>,

    /// Time to wait before retrying to send a notification after a failure.
    #[serde(with = "humantime_serde")]
    pub time_between_batsigns_retry: Option<time::Duration>,
}

impl Default for FileConfig {
    /// Default values for the configuration file, required by confy but not used directly.
    fn default() -> Self {
        Self {
            pin_number: None,
            poll_interval: None,
            hold: None,
            batsign_url: None,
            batsign_alarm_subject: None,
            batsign_alarm_message_template: None,
            batsign_restored_subject: None,
            batsign_restored_message_template: None,
            time_between_batsigns: None,
            time_between_batsigns_retry: None,
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
            batsign_url: s.batsign_url.clone(),
            batsign_alarm_subject: s.batsign_alarm_subject.clone(),
            batsign_alarm_message_template: s.batsign_alarm_message_template.clone(),
            batsign_restored_subject: s.batsign_restored_subject.clone(),
            batsign_restored_message_template: s.batsign_restored_message_template.clone(),
            time_between_batsigns: Some(s.time_between_batsigns),
            time_between_batsigns_retry: Some(s.time_between_batsigns_retry),
        }
    }
}

/// Resolves the configuration file path, returning the filename and an optional PathBuf. If a filename is provided, it uses that. Otherwise, it uses confy's default path resolution.
pub fn resolve_config_file(filename: &Option<String>) -> (String, Option<PathBuf>) {
    let pathbuf = match filename {
        Some(f) => PathBuf::from(f),
        None => {
            get_configuration_file_path(defaults::PROGRAM_ARG0, defaults::CONFIG_FILENAME_SANS_TOML)
                .expect("configuration file path resolution")
        }
    };

    let path_string = pathbuf.to_string_lossy().into_owned();

    (path_string, Some(pathbuf))
}

/// Reads the configuration file. If a filename is provided, it tries to read from that path. Otherwise, it uses confy's default path resolution.
pub fn read_config_file(
    filename: &Option<String>,
) -> Result<Option<FileConfig>, confy::ConfyError> {
    match filename {
        Some(f) => {
            let cfg = confy::load_path(f)?;
            Ok(Some(cfg))
        }
        None => {
            let (_, pathbuf) = resolve_config_file(filename);
            let pathbuf = pathbuf.expect("config file path resolution");

            if pathbuf.exists() {
                let cfg = confy::load(defaults::PROGRAM_ARG0, defaults::CONFIG_FILENAME_SANS_TOML)?;
                Ok(Some(cfg))
            } else {
                Ok(None)
            }
        }
    }
}

/// Saves the configuration file. If a filename is provided, it tries to save to that path. Otherwise, it uses confy's default path resolution.
pub fn save_config_file(
    filename: &Option<String>,
    cfg: &FileConfig,
) -> Result<(), confy::ConfyError> {
    match filename {
        Some(f) => confy::store_path(f, cfg),
        None => confy::store(
            defaults::PROGRAM_ARG0,
            defaults::CONFIG_FILENAME_SANS_TOML,
            cfg,
        ),
    }
}
