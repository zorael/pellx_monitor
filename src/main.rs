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

/// Program entrypoint.
fn main() -> process::ExitCode {
    let cli = cli::Cli::parse();

    let cfg = match config::read_config_file(&cli.config) {
        Ok(c) => c,
        Err(e) => {
            let (filename, _) = config::resolve_config_file(&cli.config);
            eprintln!("[!] Failed to read configuration file `{filename}`: {e}");
            return process::ExitCode::FAILURE;
        }
    };

    let settings = settings::Settings::default();
    let settings = settings::apply_file(settings, cfg); //.clone());
    let settings = settings::apply_cli(settings, &cli);

    if cli.show {
        settings.print();
        return process::ExitCode::SUCCESS;
    }

    if cli.save {
        let cfg = config::FileConfig::from(&settings);
        let (filename, _) = config::resolve_config_file(&cli.config);

        match config::save_config_file(&cli.config, &cfg) {
            Ok(()) => {
                println!("Configuration file written to `{filename}`.");
                return process::ExitCode::SUCCESS;
            }
            Err(e) => {
                eprintln!("[!] Failed to write configuration to `{filename}`: {e}");
                return process::ExitCode::FAILURE;
            }
        };
    }

    if let Err(vec) = settings.sanity_check() {
        eprintln!("[!] Configuration has errors:");

        for error in vec {
            eprintln!("  * {error}");
        }

        return process::ExitCode::FAILURE;
    }

    print_banner();

    let gpio = match Gpio::new() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("[!] Failed to initialize GPIO: {e}");
            return process::ExitCode::FAILURE;
        }
    };

    let pin = match gpio.get(settings.pin_number) {
        Ok(p) => p.into_input_pullup(),
        Err(e) => {
            eprintln!("[!] Failed to setup GPIO{}: {e}", settings.pin_number);
            return process::ExitCode::FAILURE;
        }
    };

    let batsign_url = settings.batsign_url.as_deref().unwrap();
    let client = Client::new();

    let mut high_since: Option<Instant> = None;
    let mut low_since: Option<Instant> = None;
    let mut last_alarm_batsign: Option<Instant> = None;
    let mut last_failed_alarm_batsign: Option<Instant> = None;
    let mut last_restored_batsign: Option<Instant> = None;
    let mut last_failed_restored_batsign: Option<Instant> = None;

    settings.print();
    println!();

    loop {
        match pin.read() {
            Level::Low => {
                // OK (closed): pull-up is overridden, LOW
                let start = low_since.get_or_insert_with(Instant::now);
                let qualified = start.elapsed() >= settings.hold;

                if settings.debug {
                    println!("LOW");
                }

                if !qualified {
                    thread::sleep(settings.poll_interval);
                    continue;
                }

                let now = Instant::now();
                high_since = None;

                if batsign::should_send_restored_batsign(
                    now,
                    last_restored_batsign,
                    last_failed_restored_batsign,
                    settings.time_between_batsigns_retry,
                ) {
                    if settings.debug {
                        println!("...should send restored batsign!");
                    }

                    let batsign_restored_message = batsign::get_batsign_message(
                        settings.batsign_restored_subject.as_deref().unwrap(),
                    );

                    if settings.dry_run {
                        println!(
                            "Dry run: would send restored Batsign with subject '{}'",
                            settings.batsign_restored_subject.as_deref().unwrap()
                        );

                        last_restored_batsign = Some(now);
                        last_failed_restored_batsign = None;
                        last_alarm_batsign = None;
                        last_failed_alarm_batsign = None;
                        thread::sleep(settings.poll_interval);
                        continue;
                    }

                    match batsign::send_batsign(&client, batsign_url, batsign_restored_message) {
                        Ok(status) if status.is_success() => {
                            println!("Batsign sent; HTTP {status}");
                            last_restored_batsign = Some(now);
                            last_failed_restored_batsign = None;
                            last_alarm_batsign = None;
                            last_failed_alarm_batsign = None;
                        }
                        Ok(status) => {
                            eprintln!("[!] Batsign returned error; HTTP {status}");
                            last_failed_restored_batsign = Some(now);
                        }
                        Err(e) => {
                            eprintln!("[!] Could not reach Batsign: {e}");
                            last_failed_restored_batsign = Some(now);
                        }
                    }
                }
            }
            Level::High => {
                // ALARM (open): internal pull-up pulls to HIGH
                let start = high_since.get_or_insert_with(Instant::now);
                let qualified = start.elapsed() >= settings.hold;

                if settings.debug {
                    println!("HIGH");
                }

                if !qualified {
                    thread::sleep(settings.poll_interval);
                    continue;
                }

                let now = Instant::now();
                low_since = None;

                if batsign::should_send_batsign(
                    now,
                    last_alarm_batsign,
                    last_failed_alarm_batsign,
                    settings.time_between_batsigns,
                    settings.time_between_batsigns_retry,
                ) {
                    if settings.debug {
                        println!("...should send batsign!");
                    }

                    let batsign_alarm_message = batsign::get_batsign_message(
                        settings.batsign_alarm_subject.as_deref().unwrap(),
                    );

                    if settings.dry_run {
                        println!(
                            "Dry run: would send alarm Batsign with subject '{}'",
                            settings.batsign_alarm_subject.as_deref().unwrap()
                        );

                        last_alarm_batsign = Some(now);
                        last_failed_alarm_batsign = None;
                        last_restored_batsign = None;
                        last_failed_restored_batsign = None;
                        thread::sleep(settings.poll_interval);
                        continue;
                    }

                    match batsign::send_batsign(&client, batsign_url, batsign_alarm_message) {
                        Ok(status) if status.is_success() => {
                            println!("Batsign sent; HTTP {status}");
                            last_alarm_batsign = Some(now);
                            last_failed_alarm_batsign = None;
                            last_restored_batsign = None;
                            last_failed_restored_batsign = None;
                        }
                        Ok(status) => {
                            eprintln!("[!] Batsign returned error; HTTP {status}");
                            last_failed_alarm_batsign = Some(now);
                        }
                        Err(e) => {
                            eprintln!("[!] Could not reach Batsign: {e}");
                            last_failed_alarm_batsign = Some(now);
                        }
                    }
                }
            }
        }

        thread::sleep(settings.poll_interval);
    }
}

/// Prints the program banner with version information.
fn print_banner() {
    let banner = format!("{} {}", defaults::PROGRAM_NAME, defaults::VERSION);
    println!("{}\n{}\n", banner, "=".repeat(banner.len()));
}
