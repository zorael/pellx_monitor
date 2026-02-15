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
    /// Raspberry Pi GPIO pin number to monitor
    #[arg(short = 'p', long, value_name = "pin")]
    pub pin_number: Option<u8>,

    /// Poll interval between GPIO pin reads
    #[arg(short = 'i', long, value_name = "duration", value_parser = humantime::parse_duration)]
    pub poll_interval: Option<Duration>,

    /// Duration the pin must be HIGH or LOW before qualifying as a valid change
    #[arg(short = 'H', long, value_name = "duration", value_parser = humantime::parse_duration)]
    pub hold: Option<Duration>,

    /// Minimum time between sending notifications
    #[arg(short = 't', long, value_name = "duration", value_parser = humantime::parse_duration)]
    pub time_between_batsigns: Option<Duration>,

    /// Time to wait before retrying to send a notification after a failure
    #[arg(short = 'r', long, value_name = "duration", value_parser = humantime::parse_duration)]
    pub time_between_batsigns_retry: Option<Duration>,

    /// Perform a dry run without sending any notifications
    #[arg(long)]
    pub dry_run: bool,

    /// Print additional debug information
    #[arg(long)]
    pub debug: bool,

    /// Show the resolved configuration and exit
    #[arg(long)]
    pub show: bool,

    /// Specify an alternate resource directory
    #[arg(short = 'R', long, value_name = "path to directory")]
    pub resource_dir: Option<String>,

    /// Write the resolved configuration to disk
    #[arg(long)]
    pub save: bool,
}
