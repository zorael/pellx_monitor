use clap::Parser;

use crate::defaults;

/// Command-line arguments, which override config file settings.
#[derive(Parser, Clone)]
#[command(name = defaults::PROGRAM_NAME)]
#[command(version = defaults::VERSION)]
#[command(author = defaults::AUTHOR)]
pub struct Cli {
    /// Specify an alternate configuration directory
    #[arg(short = 'c', long, value_name = "path to directory")]
    pub config_dir: Option<String>,

    /// Show the resolved configuration and exit
    #[arg(long)]
    pub show: bool,

    /// Print additional debug information
    #[arg(short = 'd', long)]
    pub debug: bool,

    /// Perform a dry run without sending any notifications
    #[arg(long)]
    pub dry_run: bool,

    /// Write the resolved configuration to disk
    #[arg(long)]
    pub save: bool,
}
