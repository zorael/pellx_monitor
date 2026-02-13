use confy::get_configuration_file_path;
use serde::{Deserialize, Serialize};
use std::time;

use crate::settings::Settings;

/// Configuration file structure, which overrides default settings and is overridden by CLI args.
#[derive(Default, Serialize, Deserialize)]
pub struct FileConfig {
    /// Batsign URL to send the alert to.
    pub batsign_url: Option<String>,

    /// GPIO pin number to monitor.
    pub pin_number: Option<u8>,

    /// Poll interval for checking the GPIO pin.
    #[serde(with = "humantime_serde")]
    pub poll_interval: Option<time::Duration>,

    /// Duration the pin must be HIGH or LOW before qualifying as a valid change.
    #[serde(with = "humantime_serde")]
    pub hold: Option<time::Duration>,

    /// Minimum time between sending mails.
    #[serde(with = "humantime_serde")]
    pub time_between_batsigns: Option<time::Duration>,

    /// Time to wait before retrying to send a mail after a failure.
    #[serde(with = "humantime_serde")]
    pub time_between_batsigns_retry: Option<time::Duration>,

    /// Subject line for the Batsign alarm message.
    pub batsign_alarm_subject: Option<String>,

    /// Batsign alarm message template string.
    pub batsign_alarm_message_template: Option<String>,

    /// Subject line for the Batsign alarm message.
    pub batsign_restored_subject: Option<String>,

    /// Batsign alarm message template string.
    pub batsign_restored_message_template: Option<String>,
}

impl From<&Settings> for FileConfig {
    fn from(s: &Settings) -> Self {
        Self {
            batsign_url: s.batsign_url.clone(),
            batsign_alarm_subject: s.batsign_alarm_subject.clone(),
            batsign_alarm_message_template: s.batsign_alarm_message_template.clone(),
            batsign_restored_subject: s.batsign_restored_subject.clone(),
            batsign_restored_message_template: s.batsign_restored_message_template.clone(),
            pin_number: Some(s.pin_number),
            poll_interval: Some(s.poll_interval),
            hold: Some(s.hold),
            time_between_batsigns: Some(s.time_between_batsigns),
            time_between_batsigns_retry: Some(s.time_between_batsigns_retry),
        }
    }
}

/// Reads the configuration file and returns a `FileConfig` struct if successful, or `None` if there was an error.
pub fn read_configuration_file() -> Option<FileConfig> {
    if !get_configuration_file_path("pellx_monitor", "config")
        .expect("configuration file path resolution")
        .exists()
    {
        return None;
    }

    match confy::load("pellx_monitor", "config") {
        Ok(cfg) => Some(cfg),
        Err(e) => {
            eprintln!("[!] Failed to load configuration: {e}");
            None
        }
    }
}
