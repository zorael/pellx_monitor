mod batsign;
mod cli;
mod config;
mod defaults;
mod settings;

use clap::Parser;
use reqwest::blocking::Client;
use rppal::gpio::{Gpio, Level};
use std::fs;
use std::time::Instant;
use std::{process, thread};

fn init_settings(cli: &cli::Cli) -> Result<settings::Settings, process::ExitCode> {
    let mut settings = settings::Settings::default().with_resource_dir(&cli.resource_dir);

    if !settings.resource_dir_pathbuf.exists() {
        match fs::create_dir(&settings.resource_dir_pathbuf) {
            Ok(()) => {
                println!(
                    "Resource directory `{}` created.",
                    settings.resource_dir_pathbuf.to_str().unwrap()
                );
            }
            Err(e) => {
                eprintln!(
                    "[!] Failed to create resource directory `{}`: {e}",
                    settings.resource_dir_pathbuf.to_str().unwrap()
                );
                return Err(process::ExitCode::FAILURE);
            }
        };
    }

    println!("{:?}", settings.resource_dir_pathbuf);

    settings.resolve_resource_paths();

    match settings.load_resources_from_disk() {
        Ok(()) => {}
        Err(_) if cli.save => {}
        Err(e) => {
            eprintln!("[!] Failed to load resource files: {e}");
            return Err(process::ExitCode::FAILURE);
        }
    }

    let config = match config::deserialize_config_file(&settings) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!(
                "[!] Failed to read configuration file `{}`: {e}",
                settings.config_file_pathbuf.to_str().unwrap()
            );
            return Err(process::ExitCode::FAILURE);
        }
    };

    settings = settings::apply_file(settings, &config);
    settings = settings::apply_cli(settings, cli);

    if cli.save {
        let config = config::FileConfig::from(&settings);
        match confy::store_path(&settings.config_file_pathbuf, config) {
            Ok(()) => {}
            Err(_) => {
                eprintln!("[!] Failed to write configuration file.");
                return Err(process::ExitCode::FAILURE);
            }
        };

        match fs::write(
            settings.batsign_urls_pathbuf,
            settings.batsign_urls.join("\n"),
        ) {
            Ok(()) => {}
            Err(_) => {
                eprintln!("[!] Failed to write Batsigns URL file.");
                return Err(process::ExitCode::FAILURE);
            }
        }

        match fs::write(
            settings.alarm_template_pathbuf,
            &settings.batsign_alarm_template,
        ) {
            Ok(()) => {}
            Err(_) => {
                eprintln!("[!] Failed to write alarm template file.");
                return Err(process::ExitCode::FAILURE);
            }
        }

        match fs::write(
            settings.restored_template_pathbuf,
            &settings.batsign_restored_template,
        ) {
            Ok(()) => {}
            Err(_) => {
                eprintln!("[!] Failed to write restored template file.");
                return Err(process::ExitCode::FAILURE);
            }
        }

        return Err(process::ExitCode::SUCCESS);
    }

    settings = settings::apply_file(settings, &config); //.clone());
    settings = settings::apply_cli(settings, cli);
    Ok(settings)
}

/// Program entrypoint.
fn main() -> process::ExitCode {
    if !cfg!(target_os = "linux") {
        eprintln!("[!] This program can only be run on Linux.");
        return process::ExitCode::FAILURE;
    }

    let cli = cli::Cli::parse();
    let settings = match init_settings(&cli) {
        Ok(s) => s,
        Err(code) => return code,
    };

    /*let mut settings = settings::Settings::default().with_resource_dir(&cli.resource_dir);

    if !settings.resource_dir_pathbuf.exists() {
        match fs::create_dir(&settings.resource_dir_pathbuf) {
            Ok(()) => {
                println!(
                    "Resource directory `{}` created.",
                    settings.resource_dir_pathbuf.to_str().unwrap()
                );
            }
            Err(e) => {
                eprintln!(
                    "[!] Failed to create resource directory `{}`: {e}",
                    settings.resource_dir_pathbuf.to_str().unwrap()
                );
                return process::ExitCode::FAILURE;
            }
        };
    }

    println!("{:?}", settings.resource_dir_pathbuf);

    settings.resolve_resource_paths();

    match settings.load_resources_from_disk() {
        Ok(()) => {}
        Err(_) if cli.save => {}
        Err(e) => {
            eprintln!("[!] Failed to load resource files: {e}");
            return process::ExitCode::FAILURE;
        },
    }

    let config = match config::deserialize_config_file(&settings) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!(
                "[!] Failed to read configuration file `{}`: {e}",
                settings.config_file_pathbuf.to_str().unwrap()
            );
            return process::ExitCode::FAILURE;
        }
    };

    settings = settings::apply_file(settings, &config);
    settings = settings::apply_cli(settings, &cli);

    if cli.save {
        let config = config::FileConfig::from(&settings);
        match confy::store_path(&settings.config_file_pathbuf, config) {
            Ok(()) => {}
            Err(_) => {
                eprintln!("[!] Failed to write configuration file.");
                return process::ExitCode::FAILURE;
            }
        };

        match fs::write(
            settings.batsign_urls_pathbuf,
            settings.batsign_urls.join("\n"),
        ) {
            Ok(()) => {}
            Err(_) => {
                eprintln!("[!] Failed to write Batsigns URL file.");
                return process::ExitCode::FAILURE;
            }
        }

        match fs::write(
            settings.alarm_template_pathbuf,
            &settings.batsign_alarm_template,
        ) {
            Ok(()) => {}
            Err(_) => {
                eprintln!("[!] Failed to write alarm template file.");
                return process::ExitCode::FAILURE;
            }
        }

        match fs::write(
            settings.restored_template_pathbuf,
            &settings.batsign_restored_template,
        ) {
            Ok(()) => {}
            Err(_) => {
                eprintln!("[!] Failed to write restored template file.");
                return process::ExitCode::FAILURE;
            }
        }

        return process::ExitCode::SUCCESS;
    }

    settings = settings::apply_file(settings, &config); //.clone());
    settings = settings::apply_cli(settings, &cli);
    */

    if cli.show {
        settings.print();
        return process::ExitCode::SUCCESS;
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

    let client = Client::new();

    let mut high_since: Option<Instant> = None;
    let mut low_since: Option<Instant> = None;
    let mut last_alarm_batsign: Option<Instant> = None;
    let mut last_failed_alarm_batsign: Option<Instant> = None;
    let mut last_restored_batsign: Option<Instant> = None;
    let mut last_failed_restored_batsign: Option<Instant> = None;
    let mut flips: u32 = 0;

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

                if !qualified || flips == 0 {
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
                    flips += 1;

                    if settings.debug {
                        println!("...should send restored batsign!");
                    }

                    if settings.dry_run {
                        println!("Dry run: would otherwise have sent restored Batsign");

                        last_restored_batsign = Some(now);
                        last_failed_restored_batsign = None;
                        last_alarm_batsign = None;
                        last_failed_alarm_batsign = None;
                        thread::sleep(settings.poll_interval);
                        continue;
                    }

                    let batsign_restored_message = batsign::format_batsign_message(
                        &settings.batsign_restored_template,
                        &settings,
                        &low_since.unwrap_or_else(Instant::now),
                    );

                    let statuses = match batsign::send_batsign(
                        &client,
                        &settings.batsign_urls,
                        batsign_restored_message,
                    ) {
                        Ok(statuses) => statuses,
                        Err(e) => {
                            eprintln!("[!] Could not reach Batsign: {e}");
                            last_failed_restored_batsign = Some(now);
                            thread::sleep(settings.poll_interval);
                            continue;
                        }
                    };

                    for status in statuses {
                        if status.is_success() {
                            println!("Batsign sent; HTTP {status}");
                            last_restored_batsign = Some(now);
                            last_failed_restored_batsign = None;
                            last_alarm_batsign = None;
                            last_failed_alarm_batsign = None;
                        } else {
                            eprintln!("[!] Batsign returned error; HTTP {status}");
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

                if batsign::should_send_alarm_batsign(
                    now,
                    last_alarm_batsign,
                    last_failed_alarm_batsign,
                    settings.time_between_batsigns,
                    settings.time_between_batsigns_retry,
                ) {
                    flips += 1;

                    if settings.debug {
                        println!("...should send batsign!");
                    }

                    if settings.dry_run {
                        println!("Dry run: would otherwise have sent alarm Batsign");

                        last_alarm_batsign = Some(now);
                        last_failed_alarm_batsign = None;
                        last_restored_batsign = None;
                        last_failed_restored_batsign = None;
                        thread::sleep(settings.poll_interval);
                        continue;
                    }

                    let batsign_alarm_message = batsign::format_batsign_message(
                        &settings.batsign_alarm_template,
                        &settings,
                        &high_since.unwrap_or_else(Instant::now),
                    );

                    let statuses = match batsign::send_batsign(
                        &client,
                        &settings.batsign_urls,
                        batsign_alarm_message,
                    ) {
                        Ok(statuses) => statuses,
                        Err(e) => {
                            eprintln!("[!] Could not reach Batsign: {e}");
                            last_failed_alarm_batsign = Some(now);
                            thread::sleep(settings.poll_interval);
                            continue;
                        }
                    };

                    for status in statuses {
                        if status.is_success() {
                            println!("Batsign success, response was HTTP {status}");
                            last_alarm_batsign = Some(now);
                            last_failed_alarm_batsign = None;
                            last_restored_batsign = None;
                            last_failed_restored_batsign = None;
                        } else {
                            eprintln!("[!] Batsign error, response was HTTP {status}");
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
