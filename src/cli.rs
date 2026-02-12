use clap::Parser;
use humantime;
use std::path::PathBuf;
use std::time::Duration;

use crate::defaults;

/// Command-line arguments, which override config file settings.
#[derive(Parser, Clone)]
#[command(author = defaults::AUTHOR)]
#[command(version = defaults::VERSION)]
#[command(about = defaults::ABOUT)]
pub struct Cli {
    /// GPIO pin number to monitor
    #[arg(short = 'p', long)]
    pub pin_number: Option<u8>,

    /// Poll interval for checking the GPIO pin
    #[arg(short = 'i', long, value_parser = humantime::parse_duration)]
    pub poll_interval: Option<Duration>,

    /// Duration the pin must be HIGH before qualifying as an alarm
    #[arg(short = 'q', long, value_parser = humantime::parse_duration)]
    pub qualify_high: Option<Duration>,

    /// Minimum time between sending notification mails
    #[arg(short = 't', long, value_parser = humantime::parse_duration)]
    pub time_between_mails: Option<Duration>,

    /// Time to wait before retrying to send a notification mail after a failure
    #[arg(short = 'r', long, value_parser = humantime::parse_duration)]
    pub time_between_mails_retry: Option<Duration>,

    /// Batsign URL to send the alert to (REQUIRED)
    #[arg(short = 'u', long)]
    pub batsign_url: Option<String>,

    /// Subject line for the Batsign message (REQUIRED)
    #[arg(short = 's', long)]
    pub batsign_subject: Option<String>,

    /// Override path to configuration file
    #[arg(short = 'c', long)]
    pub config: Option<PathBuf>,

    /// Write the resolved configuration to disk
    #[arg(long)]
    pub save: bool,
}
