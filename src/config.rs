use humantime_serde;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time;
use std::{env, fs};

use crate::settings::Settings;

/// Configuration file structure, which overrides default settings and is overridden by CLI args.
#[derive(Serialize, Deserialize)]
pub struct FileConfig {
    /// Batsign URL to send the alert to.
    pub batsign_url: Option<String>,

    /// Subject line for the Batsign message.
    pub batsign_subject: Option<String>,

    /// Batsign message template string.
    pub batsign_message_template: Option<String>,

    /// GPIO pin number to monitor.
    pub pin_number: Option<u8>,

    /// Poll interval for checking the GPIO pin.
    #[serde(with = "humantime_serde")]
    pub poll_interval: Option<time::Duration>,

    /// Duration the pin must be HIGH before qualifying as an alarm.
    #[serde(with = "humantime_serde")]
    pub qualify_high: Option<time::Duration>,

    /// Minimum time between sending mails.
    #[serde(with = "humantime_serde")]
    pub time_between_mails: Option<time::Duration>,

    /// Time to wait before retrying to send a mail after a failure.
    #[serde(with = "humantime_serde")]
    pub time_between_mails_retry: Option<time::Duration>,
}

impl From<&Settings> for FileConfig {
    fn from(s: &Settings) -> Self {
        Self {
            batsign_url: s.batsign_url.clone(),
            batsign_subject: s.batsign_subject.clone(),
            batsign_message_template: s.batsign_message_template.clone(),
            pin_number: Some(s.pin_number),
            poll_interval: Some(s.poll_interval),
            qualify_high: Some(s.qualify_high),
            time_between_mails: Some(s.time_between_mails),
            time_between_mails_retry: Some(s.time_between_mails_retry),
        }
    }
}

/// Resolves the default config path according to XDG Base Directory Specification.
pub fn resolve_default_config_path() -> Option<PathBuf> {
    let base = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));

    let base = base.join("pellx_monitor").join("config.toml");

    Some(base)
}

/// Loads config from TOML.
pub fn read_config_file(path: &PathBuf) -> Result<FileConfig, String> {
    let txt = fs::read_to_string(path)
        .map_err(|e| format!("failed to read config `{}`: {e}", path.display()))?;

    toml::from_str::<FileConfig>(&txt)
        .map_err(|e| format!("failed to parse TOML `{}`: {e}", path.display()))
}

/// Saves the resolved config to a TOML file, creating parent directories if necessary.
pub fn save_config(path: &PathBuf, settings: &Settings) -> Result<(), String> {
    let cfg = FileConfig::from(settings);
    let toml =
        toml::to_string_pretty(&cfg).map_err(|e| format!("failed to serialize config: {e}"))?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create config dir `{}`: {e}", parent.display()))?;
    }

    fs::write(path, toml)
        .map_err(|e| format!("failed to write config `{}`: {e}", path.display()))?;

    Ok(())
}
