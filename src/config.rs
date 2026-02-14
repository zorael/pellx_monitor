use confy::get_configuration_file_path;
use serde::{Deserialize, Serialize};
use std::time;

use crate::defaults;
use crate::settings::Settings;

/// Configuration file structure, which overrides default settings and is overridden by CLI args.
#[derive(Clone, Serialize, Deserialize)]
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

    /// Minimum time between sending mails.
    #[serde(with = "humantime_serde")]
    pub time_between_batsigns: Option<time::Duration>,

    /// Time to wait before retrying to send a mail after a failure.
    #[serde(with = "humantime_serde")]
    pub time_between_batsigns_retry: Option<time::Duration>,
}

/// Default values for the configuration file, required by confy but not used directly.
impl Default for FileConfig {
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

/// Reads the configuration file. If a filename is provided, it tries to read from that path. Otherwise, it uses confy's default path resolution.
pub fn read_configuration_file(filename: &Option<String>) -> Result<FileConfig, confy::ConfyError> {
    match filename {
        Some(f) => confy::load_path(f),
        None => {
            if !get_configuration_file_path(defaults::PROGRAM_ARG0, defaults::CONFIGURATION_TOML)
                .expect("configuration file path resolution")
                .exists()
            {
                return Ok(FileConfig::from(&Settings::default()));
            }

            confy::load(defaults::PROGRAM_ARG0, defaults::CONFIGURATION_TOML)
        }
    }
}

pub fn save_configuration_file(
    filename: &Option<String>,
    cfg: &FileConfig,
) -> Result<(), confy::ConfyError> {
    match filename {
        Some(f) => confy::store_path(f, cfg),
        None => confy::store(defaults::PROGRAM_ARG0, defaults::CONFIGURATION_TOML, cfg),
    }
}
