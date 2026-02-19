use clap::Parser;

use crate::defaults;

// Don't have the below be a documenting /// or it will pollute the --help screen.
// Command-line arguments, which override config file settings.
#[derive(Parser, Clone)]
#[command(name = defaults::PROGRAM_NAME)]
#[command(author = defaults::AUTHOR)]
//#[command(version = defaults::VERSION)]
pub struct Cli {
    /// Specify an alternate configuration directory
    #[arg(short = 'c', long, value_name = "path")]
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

    /// Write configuration to disk
    #[arg(long)]
    pub save: bool,

    /// Display version information and exit
    #[arg(short = 'V', long)]
    pub version: bool,
}
