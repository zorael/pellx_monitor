use clap::Parser;
use humantime;
use reqwest::blocking::Client;
use rppal::gpio::{Gpio, Level};
use serde::Deserialize;
use std::{env, fs, thread, process, path::PathBuf, time};
use std::time::{Duration, Instant};

const DEFAULT_PIN: u8 = 24;  // GPIO24, physical pin 18 on Raspberry Pi
const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(1);
const DEFAULT_QUALIFY_HIGH: Duration = Duration::from_secs(10);
const DEFAULT_TIME_BETWEEN_MAILS: Duration = Duration::from_secs(30 * 60);  // 30 min
const DEFAULT_TIME_BETWEEN_MAILS_RETRY: Duration = Duration::from_secs(5 * 60);  // 5 min
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
    batsign_url: String,
    batsign_subject: String,
    config_path: PathBuf,
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
            batsign_url: String::new(),
            batsign_subject: String::new(), //DEFAULT_BATSIGN_SUBJECT.to_string(),
            config_path: PathBuf::new(),
        }
    }
}

/// Sanity check settings, returning a list of errors if any are found.
impl Settings {
    fn sanity_check(&self) -> Result<(), Vec<String>> {
        let mut vec: Vec<String> = Vec::new();

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

        if self.batsign_url.trim().is_empty() {
            vec.push("Requires a Batsign URL to send alerts to.".to_string());
        }

        if self.batsign_subject.trim().is_empty() {
            vec.push("Requires a Batsign subject to send alerts with.".to_string());
        }

        if vec.is_empty() {
            Ok(())
        } else {
            Err(vec)
        }
    }
}

/// Command-line arguments, which override config file settings.
#[derive(Parser)]
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
}

/// Configuration file structure, which overrides default settings and is overridden by CLI args.
#[derive(Deserialize)]
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

/// Resolves the default config path according to XDG Base Directory Specification.
fn resolve_default_config_path() -> PathBuf {
    let base = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));

    base
        .join("pellx_monitor")
        .join("config.toml")
}

/// Loads config from TOML.
fn read_config_file(path: &PathBuf) -> Result<FileConfig, String> {
    let txt = fs::read_to_string(path)
        .map_err(|e| format!("failed to read config `{}`: {e}", path.display()))?;

    toml::from_str::<FileConfig>(&txt)
        .map_err(|e| format!("failed to parse TOML `{}`: {e}", path.display()))
}

/// Applies config file settings to the default settings, returning the resulting settings.
fn apply_file(mut s: Settings, f: FileConfig) -> Settings {
    if let Some(pin_number) = f.pin_number { s.pin_number = pin_number; }
    if let Some(poll_interval) = f.poll_interval { s.poll_interval = poll_interval; }
    if let Some(qualify_high) = f.qualify_high { s.qualify_high = qualify_high; }
    if let Some(time_between_mails) = f.time_between_mails { s.time_between_mails = time_between_mails; }
    if let Some(time_between_mails_retry) = f.time_between_mails_retry { s.time_between_mails_retry = time_between_mails_retry; }
    if let Some(batsign_url) = f.batsign_url { s.batsign_url = batsign_url; }
    if let Some(batsign_subject) = f.batsign_subject { s.batsign_subject = batsign_subject; }
    s
}

/// Applies CLI settings to the given settings, returning the resulting settings.
fn apply_cli(mut s: Settings, c: Cli) -> Settings {
    if let Some(pin_number) = c.pin_number { s.pin_number = pin_number; }
    if let Some(poll_interval) = c.poll_interval { s.poll_interval = poll_interval; }
    if let Some(qualify_high) = c.qualify_high { s.qualify_high = qualify_high; }
    if let Some(time_between_mails) = c.time_between_mails { s.time_between_mails = time_between_mails; }
    if let Some(time_between_mails_retry) = c.time_between_mails_retry { s.time_between_mails_retry = time_between_mails_retry; }
    if let Some(batsign_url) = c.batsign_url { s.batsign_url = batsign_url; }
    if let Some(batsign_subject) = c.batsign_subject { s.batsign_subject = batsign_subject; }
    s
}

/// Program entrypoint.
fn main() -> process::ExitCode {
    let cli = Cli::parse();
    let mut settings = Settings::default();

    match &cli.config {
        Some(path) => settings.config_path = path.clone(),
        None => settings.config_path = resolve_default_config_path(),
    };

    // Only apply config file if it exists
    if settings.config_path.exists() {
        match read_config_file(&settings.config_path) {
            Ok(f) => settings = apply_file(settings, f),
            Err(e) => {
                eprintln!("{e}");
                return process::ExitCode::FAILURE;
            }
        }
    }

    settings = apply_cli(settings, cli);

    settings.sanity_check().unwrap_or_else(|errors| {
        eprintln!("Configuration errors:");
        for error in errors {
            eprintln!("- {error}");
        }
        process::exit(1);
    });

    let gpio = Gpio::new().expect("GPIO init");
    let pin = gpio.get(settings.pin_number).expect("GPIO pin get").into_input_pullup();
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
    println!("Batsign URL:        {}", settings.batsign_url);

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

                if should_send_mail(last_mail, last_failed_mail, settings.time_between_mails, settings.time_between_mails_retry) {
                    let now = Instant::now();
                    let message = get_batsign_message(&settings.batsign_subject, &high_since);

                    match send_batsign(&client, &settings.batsign_url, message) {
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
fn should_send_mail(last: Option<Instant>, last_failed: Option<Instant>, time_between_mails: Duration, time_between_mails_retry: Duration) -> bool {
    let now = Instant::now();

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
fn get_batsign_message(subject: &String, high_since: &Option<Instant>) -> String {
    format!("Subject: {}\n{:?}",
        subject,
        high_since.expect("high_since when getting batsign message"),
    )
}

/// Sends a batsign message to the specified URL, returning the HTTP status code or an error.
fn send_batsign(client: &Client, url: &String, message: String) -> Result<reqwest::StatusCode, reqwest::Error> {
    let res = client.post(url).body(message).send()?;
    Ok(res.status())
}
