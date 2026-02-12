use clap::Parser;
use humantime;
use humantime_serde;
use reqwest::blocking::Client;
use rppal::gpio::{Gpio, Level};
use serde::{Deserialize, Serialize};
use std::{env, fs, thread, process, path::PathBuf, time};
use std::time::{Duration, Instant};

const DEFAULT_PIN: u8 = 24;  // GPIO24, physical pin 18 on Raspberry Pi
const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(1);
const DEFAULT_QUALIFY_HIGH: Duration = Duration::from_secs(10);
const DEFAULT_TIME_BETWEEN_MAILS: Duration = Duration::from_secs(30 * 60);  // 30 min
const DEFAULT_TIME_BETWEEN_MAILS_RETRY: Duration = Duration::from_secs(5 * 60);  // 5 min
const DEFAULT_SUBJECT: &'static str = "PellX Alarm";
const VERSION: &'static str = concat!("v", env!("CARGO_PKG_VERSION"), "-alpha.01");
const ABOUT: &'static str = "pellX monitor\n$ git clone https://github.com/zorael/pellx_monitor";

/// Application settings, including defaults and sanity checks.
#[derive(Clone)]
struct Settings {
    pin_number: u8,
    poll_interval: Duration,
    qualify_high: Duration,
    time_between_mails: Duration,
    time_between_mails_retry: Duration,
    batsign_url: Option<String>,
    batsign_subject: Option<String>,
    config_path: Option<PathBuf>,
}

/// Default settings, which can be overridden by config file and CLI args.
impl Default for Settings {
    fn default() -> Self {
        Self {
            pin_number: DEFAULT_PIN,
            poll_interval: DEFAULT_POLL_INTERVAL,
            qualify_high: DEFAULT_QUALIFY_HIGH,
            time_between_mails: DEFAULT_TIME_BETWEEN_MAILS,
            time_between_mails_retry: DEFAULT_TIME_BETWEEN_MAILS_RETRY,
            batsign_url: None,
            batsign_subject: Some(DEFAULT_SUBJECT.to_string()),
            config_path: None,
        }
    }
}

/// Sanity check settings, returning a list of errors if any are found.
impl Settings {
    fn sanity_check(&self) -> Result<(), Vec<String>> {
        let mut vec = Vec::new();

        if self.pin_number > 27 {
            vec.push(format!("Invalid GPIO pin number: {}. Must be between 0 and 27.", self.pin_number));
        }

        if self.poll_interval == Duration::ZERO {
            vec.push("Poll interval must be greater than zero.".to_string());
        }

        if self.time_between_mails == Duration::ZERO {
            vec.push("Time between mails must be greater than zero.".to_string());
        }

        if self.time_between_mails_retry == Duration::ZERO {
            vec.push("Time between mails retry must be greater than zero.".to_string());
        }

        match self.batsign_url.as_deref().map(str::trim) {
            Some(url) if url.is_empty() => vec.push("Batsign URL cannot be empty.".to_string()),
            Some(url) if !url.starts_with("http://") && !url.starts_with("https://") =>
                vec.push("Batsign URL must start with http:// or https://.".to_string()),
            None => vec.push("Batsign URL is required.".to_string()),
            _ => {}
        }

        match self.batsign_subject.as_deref().map(str::trim) {
            Some(subject) if subject.is_empty() => vec.push("Batsign subject cannot be empty.".to_string()),
            None => vec.push("Batsign subject is required.".to_string()),
            _ => {}
        }

        if vec.is_empty() {
            Ok(())
        } else {
            Err(vec)
        }
    }
}

/// Command-line arguments, which override config file settings.
#[derive(Parser, Clone)]
#[command(name = "pellx_monitor")]
#[command(author = "jr <zorael@gmail.com>")]
#[command(version = VERSION)]
#[command(about = ABOUT)]
struct Cli {
    /// GPIO pin number to monitor
    #[arg(short = 'p', long)]
    pin_number: Option<u8>,

    /// Poll interval for checking the GPIO pin
    #[arg(short = 'i', long, value_parser = humantime::parse_duration)]
    poll_interval: Option<Duration>,

    /// Duration the pin must be HIGH before qualifying as an alarm
    #[arg(short = 'q', long, value_parser = humantime::parse_duration)]
    qualify_high: Option<Duration>,

    /// Minimum time between sending mails
    #[arg(short = 't', long, value_parser = humantime::parse_duration)]
    time_between_mails: Option<Duration>,

    /// Time to wait before retrying to send a mail after a failure
    #[arg(short = 'r', long, value_parser = humantime::parse_duration)]
    time_between_mails_retry: Option<Duration>,

    /// Batsign URL to send the alert to (REQUIRED)
    #[arg(short = 'u', long)]
    batsign_url: Option<String>,

    /// Subject line for the Batsign message (REQUIRED)
    #[arg(short = 's', long)]
    batsign_subject: Option<String>,

    /// Override path to configuration file
    #[arg(short = 'c', long)]
    config: Option<PathBuf>,

    /// Write the resolved configuration (defaults + config file + CLI) to disk and exit
    #[arg(long)]
    save: bool,
}

/// Configuration file structure, which overrides default settings and is overridden by CLI args.
#[derive(Serialize, Deserialize)]
struct FileConfig {
    batsign_url: Option<String>,
    batsign_subject: Option<String>,
    pin_number: Option<u8>,

    #[serde(with = "humantime_serde")]
    poll_interval: Option<time::Duration>,

    #[serde(with = "humantime_serde")]
    qualify_high: Option<time::Duration>,

    #[serde(with = "humantime_serde")]
    time_between_mails: Option<time::Duration>,

    #[serde(with = "humantime_serde")]
    time_between_mails_retry: Option<time::Duration>,
}

impl From<&Settings> for FileConfig {
    fn from(s: &Settings) -> Self {
        Self {
            batsign_url: s.batsign_url.clone(),
            batsign_subject: s.batsign_subject.clone(),
            pin_number: Some(s.pin_number),
            poll_interval: Some(s.poll_interval),
            qualify_high: Some(s.qualify_high),
            time_between_mails: Some(s.time_between_mails),
            time_between_mails_retry: Some(s.time_between_mails_retry),
        }
    }
}

/// Resolves the default config path according to XDG Base Directory Specification.
fn resolve_default_config_path() -> Option<PathBuf> {
    let base = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));

    let base = base
        .join("pellx_monitor")
        .join("config.toml");

    Some(base)
}

/// Loads config from TOML.
fn read_config_file(path: &PathBuf) -> Result<FileConfig, String> {
    let txt = fs::read_to_string(path)
        .map_err(|e| format!("failed to read config `{}`: {e}", path.display()))?;

    toml::from_str::<FileConfig>(&txt)
        .map_err(|e| format!("failed to parse TOML `{}`: {e}", path.display()))
}

fn save_config(path: &PathBuf, settings: &Settings) -> Result<(), String> {
    let cfg = FileConfig::from(settings);
    let toml = toml::to_string_pretty(&cfg)
        .map_err(|e| format!("failed to serialize config: {e}"))?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create config dir `{}`: {e}", parent.display()))?;
    }

    fs::write(path, toml)
        .map_err(|e| format!("failed to write config `{}`: {e}", path.display()))?;

    Ok(())
}


/// Applies config file settings to the default settings, returning the resulting settings.
fn apply_file(mut s: Settings, f: FileConfig) -> Settings {
    if let Some(pin_number) = f.pin_number { s.pin_number = pin_number; }
    if let Some(poll_interval) = f.poll_interval { s.poll_interval = poll_interval; }
    if let Some(qualify_high) = f.qualify_high { s.qualify_high = qualify_high; }
    if let Some(time_between_mails) = f.time_between_mails { s.time_between_mails = time_between_mails; }
    if let Some(time_between_mails_retry) = f.time_between_mails_retry { s.time_between_mails_retry = time_between_mails_retry; }
    if f.batsign_url.is_some() { s.batsign_url = f.batsign_url; }
    if f.batsign_subject.is_some() { s.batsign_subject = f.batsign_subject; }
    s
}

/// Applies CLI settings to the given settings, returning the resulting settings.
fn apply_cli(mut s: Settings, c: Cli) -> Settings {
    if let Some(pin_number) = c.pin_number { s.pin_number = pin_number; }
    if let Some(poll_interval) = c.poll_interval { s.poll_interval = poll_interval; }
    if let Some(qualify_high) = c.qualify_high { s.qualify_high = qualify_high; }
    if let Some(time_between_mails) = c.time_between_mails { s.time_between_mails = time_between_mails; }
    if let Some(time_between_mails_retry) = c.time_between_mails_retry { s.time_between_mails_retry = time_between_mails_retry; }
    if c.batsign_url.is_some() { s.batsign_url = c.batsign_url; }
    if c.batsign_subject.is_some() { s.batsign_subject = c.batsign_subject; }
    s
}

/// Program entrypoint.
fn main() -> process::ExitCode {
    let cli = Cli::parse();
    let mut settings = Settings::default();

    settings.config_path = cli.config.clone().or_else(resolve_default_config_path);

    if let Some(path) = &settings.config_path {
        if path.exists() {
            match read_config_file(&path) {
                Ok(f) => settings = apply_file(settings, f),
                Err(e) => {
                    eprintln!("{e}");
                    return process::ExitCode::FAILURE;
                }
            }
        } else if cli.config.is_some() {
            eprintln!("Config file not found at {:?}.", path);
            return process::ExitCode::FAILURE;
        }
    }

    settings = apply_cli(settings, cli.clone());

    if let Err(vec) = settings.sanity_check() {
        eprintln!("Configuration errors:");

        for error in vec {
            eprintln!("* {error}");
        }

        return process::ExitCode::FAILURE;
    }

    if cli.save {
        let path = settings
            .config_path
            .clone()
            .ok_or_else(|| "could not resolve config path".to_string());

        let path = match path {
            Ok(p) => p,
            Err(e) => {
                eprintln!("error: {e}");
                return process::ExitCode::FAILURE;
            }
        };

        match save_config(&path, &settings) {
            Ok(()) => {
                println!("Wrote config to {}", path.display());
                return process::ExitCode::SUCCESS;
            }
            Err(e) => {
                eprintln!("error: {e}");
                return process::ExitCode::FAILURE;
            }
        }
    }

    let gpio = match Gpio::new() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("error: failed to initialize GPIO (rppal): {e}");
            return process::ExitCode::FAILURE;
        }
    };

    let pin = match gpio.get(settings.pin_number) {
        Ok(p) => p.into_input_pullup(),
        Err(e) => {
            eprintln!("error: failed to access GPIO{}: {e}", settings.pin_number);
            return process::ExitCode::FAILURE;
        }
    };

    let batsign_message = get_batsign_message(settings.batsign_subject.as_deref().unwrap());
    let batsign_url = settings.batsign_url.as_deref().unwrap();
    let client = Client::new();

    let mut high_since: Option<Instant> = None;
    let mut last_mail: Option<Instant> = None;
    let mut last_failed_mail: Option<Instant> = None;
    let mut printed_alarm = false;

    println!("PellX monitor starting...");
    println!("GPIO pin number:    {}", settings.pin_number);
    println!("Poll interval:      {}", humantime::format_duration(settings.poll_interval));
    println!("Qualify HIGH:       {}", humantime::format_duration(settings.qualify_high));
    println!("Time between mails: {}", humantime::format_duration(settings.time_between_mails));
    println!("Batsign URL:        {}", batsign_url);

    loop {
        match pin.read() {
            Level::Low => {
                // OK (closed): pull-up is overridden, LOW
                if printed_alarm {
                    println!("Reset to LOW.");
                    printed_alarm = false;
                }

                high_since = None;
                last_mail = None;
                last_failed_mail = None;
            },
            Level::High => {
                // ALARM (open): internal pull-up pulls to HIGH
                let start = high_since.get_or_insert_with(Instant::now);
                let qualified = start.elapsed() >= settings.qualify_high;

                if !qualified {
                    continue;
                }

                if !printed_alarm {
                    // Print alarm only once
                    println!("ALARM qualified: HIGH i >= {}.", humantime::format_duration(settings.qualify_high));
                    printed_alarm = true;
                }

                let now = Instant::now();

                if should_send_mail(now, last_mail, last_failed_mail, settings.time_between_mails, settings.time_between_mails_retry) {
                    match send_batsign(&client, &batsign_url, &batsign_message) {
                        Ok(status) if status.is_success() => {
                            println!("Batsign sent; HTTP {status}");
                            last_mail = Some(now);
                            last_failed_mail = None;
                        },
                        Ok(status) => {
                            eprintln!("Batsign returned error; HTTP {status}");
                            last_failed_mail = Some(now);
                        },
                        Err(e) => {
                            eprintln!("Could not reach Batsign: {e}");
                            last_failed_mail = Some(now);
                        }
                    }
                }
            },
        }

        thread::sleep(settings.poll_interval);
    }
}

/// Determines if a mail should be sent, based on the last successful and failed mail times.
fn should_send_mail(
    now: Instant,
    last: Option<Instant>,
    last_failed: Option<Instant>,
    time_between_mails: Duration,
    time_between_mails_retry: Duration
) -> bool {
    if let Some(last_failed) = last_failed {
        return now.duration_since(last_failed) >= time_between_mails_retry;
    }

    if let Some(last) = last {
        now.duration_since(last) >= time_between_mails
    } else {
        true
    }
}

/// Constructs the Batsign message body, including the subject and the time at which the pin went HIGH.
fn get_batsign_message(subject: &str) -> String {
    format!("Subject: {subject}\n")
}

/// Sends a batsign message to the specified URL, returning the HTTP status code or an error.
fn send_batsign(client: &Client, url: &str, message: &str) -> Result<reqwest::StatusCode, reqwest::Error> {
    let res = client.post(url).body(message.to_owned()).send()?;
    Ok(res.status())
}
