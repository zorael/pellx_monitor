use confy::get_configuration_file_path;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::{fs, io, time};

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

/// Resolves the configuration directory path, returning the directory as a string and an optional PathBuf. This is used for operations that need to know the config directory, such as saving the config file.
pub fn resolve_config_directory() -> (String, PathBuf) {
    let (_, pathbuf) = resolve_config_file(&None);
    let mut pathbuf = pathbuf.expect("config file path resolution");

    pathbuf.pop();

    let path_string = pathbuf.to_string_lossy().into_owned();

    (path_string, pathbuf)
}

/// Reads lines from a file, returning a vector of non-empty, non-comment lines. This is used for reading the Batsigns file.
pub fn read_lines(filename: &str) -> io::Result<Vec<String>> {
    let s = fs::read_to_string(filename)?;
    let v = s
        .split('\n')
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();
    Ok(v)
}

/// Resolves the path to a resource file (such as the Batsigns file or message templates), returning the filename as a string and an optional PathBuf. The file is expected to be located in the same directory as the configuration file.
pub fn resolve_resource_file(filename: &str) -> (String, PathBuf) {
    let (_, config_dir) = resolve_config_directory();
    let file_pathbuf = config_dir.join(filename);
    let file_path_str = file_pathbuf.to_str().unwrap_or(filename).to_string();

    (file_path_str, file_pathbuf)
}

/// Reads the Batsigns file, which contains a list of URLs to send notifications to. The file is expected to be located in the same directory as the configuration file.
pub fn read_batsigns_file() -> io::Result<Vec<String>> {
    let (batsigns_file_path_str, _) = resolve_resource_file(defaults::BATSIGNS_FILENAME);
    let mut v = Vec::new();

    match read_lines(&batsigns_file_path_str) {
        Ok(lines) => {
            for line in lines {
                v.push(line);
            }

            Ok(v)
        }
        Err(e) => Err(e),
    }
}

/// Reads a resource file (such as the message templates) and returns its contents as a string. The file is expected to be located in the same directory as the configuration file.
pub fn read_resource_file(filename: &str) -> io::Result<String> {
    let (file_path_str, _) = resolve_resource_file(filename);
    fs::read_to_string(&file_path_str)
}

/// Saves a resource file (such as the message templates) with the given contents. The file is expected to be located in the same directory as the configuration file.
pub fn save_resource_file(filename: &str, contents: &str) -> io::Result<()> {
    let (_, file_pathbuf) = resolve_resource_file(filename);
    fs::write(file_pathbuf, contents)
}
