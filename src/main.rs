mod batsign;
mod cli;
mod config;
mod defaults;
mod notifications;
mod settings;
mod slack;

use clap::Parser;
use reqwest::blocking::Client;
use rppal::gpio::{Gpio, Level};
use std::time::{Duration, Instant};
use std::{fs, process, thread};

use crate::notifications::NotificationState;

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

    if cli.show {
        if settings.debug {
            println!("{:#?}", &settings);
        } else {
            settings.print();
        }

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

    if settings.debug {
        println!("{:#?}", &settings);
    } else {
        settings.print();
    }

    println!();

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

    let mut slack_low_state = NotificationState::new(
        settings.time_between_slack_notifications,
        Duration::from_secs(3600 * 24 * 365 * 999), // effectively disable retries for restored notifications
    );
    let mut slack_high_state = NotificationState::new(
        settings.time_between_slack_notifications,
        settings.time_between_slack_notification_retries,
    );
    let mut batsign_low_state = NotificationState::new(
        settings.time_between_batsigns,
        Duration::from_secs(3600 * 24 * 365 * 999), // as above
    );
    let mut batsign_high_state = NotificationState::new(
        settings.time_between_batsigns,
        settings.time_between_batsign_retries,
    );

    let mut low_since: Option<Instant> = None;
    let mut high_since: Option<Instant> = None;
    let mut still_in_initial_state = true;

    loop {
        match pin.read() {
            Level::Low => {
                // OK (closed): pull-up is overridden, LOW
                let start = low_since.get_or_insert_with(Instant::now);
                let qualified = start.elapsed() >= settings.hold;

                if settings.debug {
                    println!("LOW");
                }

                if !qualified || still_in_initial_state {
                    thread::sleep(settings.poll_interval);
                    continue;
                }

                let now = Instant::now();
                let low_since = low_since.expect("low_since should be set with .get_or_insert");
                high_since = None;

                if slack::should_send_slack_notification(now, &settings, &slack_low_state) {
                    let message = &notifications::format_notification_message(
                        settings.slack_restored_template_body.as_str(),
                        &settings,
                        &low_since,
                    );

                    match slack::send_slack_notification(
                        &client,
                        now,
                        &settings,
                        slack::SLACK_SUCCESS_EMOJI,
                        message,
                        &slack_low_state,
                    ) {
                        Ok(state) => {
                            slack_low_state = state;
                            slack_high_state.reset();
                        }
                        Err(e) => {
                            eprintln!("[!] Failed to send Slack notification: {e}");
                        }
                    };
                }

                if batsign::should_send_batsign_notification(now, &settings, &batsign_low_state) {
                    let message = &notifications::format_notification_message(
                        settings.batsign_restored_template_body.as_str(),
                        &settings,
                        &low_since,
                    );

                    match batsign::send_batsign_notification(
                        &client,
                        now,
                        &settings,
                        message,
                        &batsign_low_state,
                    ) {
                        Ok(state) => {
                            batsign_low_state = state;
                            batsign_high_state.reset();
                        }
                        Err(e) => {
                            eprintln!("[!] Failed to send Batsign notification: {e}");
                        }
                    };
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
                let high_since = high_since.expect("high_since should be set with .get_or_insert");
                low_since = None;

                if slack::should_send_slack_notification(now, &settings, &slack_high_state) {
                    let message = &notifications::format_notification_message(
                        settings.slack_alarm_template_body.as_str(),
                        &settings,
                        &high_since,
                    );

                    match slack::send_slack_notification(
                        &client,
                        now,
                        &settings,
                        slack::SLACK_ERROR_EMOJI,
                        message,
                        &slack_high_state,
                    ) {
                        Ok(state) => {
                            slack_high_state = state;
                            slack_low_state.reset();
                        }
                        Err(e) => {
                            eprintln!("[!] Failed to send Slack notification: {e}");
                        }
                    };
                }

                if batsign::should_send_batsign_notification(now, &settings, &batsign_high_state) {
                    let message = &notifications::format_notification_message(
                        settings.batsign_alarm_template_body.as_str(),
                        &settings,
                        &high_since,
                    );

                    match batsign::send_batsign_notification(
                        &client,
                        now,
                        &settings,
                        message,
                        &batsign_high_state,
                    ) {
                        Ok(state) => {
                            batsign_high_state = state;
                            batsign_low_state.reset();
                        }
                        Err(e) => {
                            eprintln!("[!] Failed to send Batsign notification: {e}");
                        }
                    };
                }

                still_in_initial_state = false;
            }
        }

        thread::sleep(settings.poll_interval);
    }
}

/// Initializes the settings by loading defaults, applying the config file, and then applying CLI overrides. If the `--save` flag is set, it saves the resolved configuration back to disk and exits.
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
            settings.slack_alarm_template_pathbuf,
            &settings.slack_alarm_template_body,
        ) {
            Ok(()) => {}
            Err(_) => {
                eprintln!("[!] Failed to write Slack alarm template file.");
                return Err(process::ExitCode::FAILURE);
            }
        }

        match fs::write(
            settings.slack_restored_template_pathbuf,
            &settings.slack_restored_template_body,
        ) {
            Ok(()) => {}
            Err(_) => {
                eprintln!("[!] Failed to write Slack restored template file.");
                return Err(process::ExitCode::FAILURE);
            }
        }

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
            settings.batsign_alarm_template_pathbuf,
            &settings.batsign_alarm_template_body,
        ) {
            Ok(()) => {}
            Err(_) => {
                eprintln!("[!] Failed to write Batsign alarm template file.");
                return Err(process::ExitCode::FAILURE);
            }
        }

        match fs::write(
            settings.batsign_restored_template_pathbuf,
            &settings.batsign_restored_template_body,
        ) {
            Ok(()) => {}
            Err(_) => {
                eprintln!("[!] Failed to write Batsign restored template file.");
                return Err(process::ExitCode::FAILURE);
            }
        }

        println!(
            "Configuration and resources written successfully to `{}`.",
            settings.resource_dir_pathbuf.to_str().unwrap()
        );
        return Err(process::ExitCode::SUCCESS);
    }

    settings = settings::apply_file(settings, &config); //.clone());
    settings = settings::apply_cli(settings, cli);
    Ok(settings)
}

/// Prints the program banner with version information.
fn print_banner() {
    let banner = format!("{} {}", defaults::PROGRAM_NAME, defaults::VERSION);
    println!("{}\n{}\n", banner, "=".repeat(banner.len()));
}
