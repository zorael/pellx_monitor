use clap::Parser;
use std::time::Duration;

use crate::defaults;

/// Command-line arguments, which override config file settings.
#[derive(Parser, Clone)]
#[command(name = defaults::PROGRAM_NAME)]
#[command(version = defaults::VERSION)]
#[command(author = defaults::AUTHOR)]
#[command(about = defaults::ABOUT)]
pub struct Cli {
    /// GPIO pin number to monitor
    #[arg(short = 'p', long)]
    pub pin_number: Option<u8>,

    /// Poll interval for checking the GPIO pin
    #[arg(short = 'i', long, value_parser = humantime::parse_duration)]
    pub poll_interval: Option<Duration>,

    /// Duration the pin must be HIGH or LOW before qualifying as a valid change
    #[arg(short = 'H', long, value_parser = humantime::parse_duration)]
    pub hold: Option<Duration>,

    /// Minimum time between sending notification mails
    #[arg(short = 't', long, value_parser = humantime::parse_duration)]
    pub time_between_batsigns: Option<Duration>,

    /// Time to wait before retrying to send a notification mail after a failure
    #[arg(short = 'r', long, value_parser = humantime::parse_duration)]
    pub time_between_batsigns_retry: Option<Duration>,

    /// Batsign URL to send the alert to (REQUIRED)
    #[arg(short = 'u', long)]
    pub batsign_url: Option<String>,

    /// Perform a dry run without sending any mails
    #[arg(long)]
    pub dry_run: bool,

    /// Write the resolved configuration to disk
    #[arg(long)]
    pub save: bool,
}
