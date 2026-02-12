mod batsign;
mod cli;
mod config;
mod defaults;
mod settings;

use clap::Parser;
use reqwest::blocking::Client;
use rppal::gpio::{Gpio, Level};
use std::time::Instant;
use std::{process, thread};

use crate::batsign::{get_batsign_message, send_batsign, should_send_batsign};
use crate::cli::Cli;
use crate::config::{read_config_file, resolve_default_config_path, save_config};
use crate::settings::{Settings, apply_cli, apply_file};

/// Program entrypoint.
fn main() -> process::ExitCode {
    let cli = Cli::parse();
    let mut settings = Settings::default();

    settings.config_path = cli.config.clone().or_else(resolve_default_config_path);

    if let Some(path) = &settings.config_path {
        if path.exists() {
            match read_config_file(path) {
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
    let mut last_batsign: Option<Instant> = None;
    let mut last_failed_batsign: Option<Instant> = None;
    let mut printed_alarm = false;

    println!("PellX monitor starting...");
    println!("GPIO pin number:    {}", settings.pin_number);
    println!(
        "Poll interval:      {}",
        humantime::format_duration(settings.poll_interval)
    );
    println!(
        "Qualify HIGH:       {}",
        humantime::format_duration(settings.qualify_high)
    );
    println!(
        "Time between mails: {}",
        humantime::format_duration(settings.time_between_mails)
    );
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
                last_batsign = None;
                last_failed_batsign = None;
            }
            Level::High => {
                // ALARM (open): internal pull-up pulls to HIGH
                let start = high_since.get_or_insert_with(Instant::now);
                let qualified = start.elapsed() >= settings.qualify_high;

                if !qualified {
                    thread::sleep(settings.poll_interval);
                    continue;
                }

                if !printed_alarm {
                    // Print alarm only once
                    println!(
                        "ALARM qualified: HIGH i >= {}.",
                        humantime::format_duration(settings.qualify_high)
                    );
                    printed_alarm = true;
                }

                let now = Instant::now();

                if should_send_batsign(
                    now,
                    last_batsign,
                    last_failed_batsign,
                    settings.time_between_mails,
                    settings.time_between_mails_retry,
                ) {
                    match send_batsign(&client, &batsign_url, &batsign_message) {
                        Ok(status) if status.is_success() => {
                            println!("Batsign sent; HTTP {status}");
                            last_batsign = Some(now);
                            last_failed_batsign = None;
                        }
                        Ok(status) => {
                            eprintln!("Batsign returned error; HTTP {status}");
                            last_failed_batsign = Some(now);
                        }
                        Err(e) => {
                            eprintln!("Could not reach Batsign: {e}");
                            last_failed_batsign = Some(now);
                        }
                    }
                }
            }
        }

        thread::sleep(settings.poll_interval);
    }
}
