mod backend;
mod cli;
mod defaults;
mod file_config;
mod notifications;
mod settings;

use clap::Parser;
use reqwest::blocking::Client;
use rppal::gpio::{Gpio, InputPin, Level};
use std::sync::Arc;
use std::time::Instant;
use std::{fs, process, thread};

use crate::settings::Settings;

/// Prints the program banner with version information.
fn print_banner() {
    println!(
        "{} {}\n$ git clone {}",
        defaults::PROGRAM_NAME,
        defaults::VERSION,
        defaults::SOURCE_REPOSITORY
    );
}

/// Program entrypoint.
fn main() -> process::ExitCode {
    if !cfg!(target_os = "linux") {
        eprintln!("[!] This program can only be run on Linux.");
        return process::ExitCode::from(defaults::exit_codes::WRONG_PLATFORM);
    }

    print_banner();
    println!();

    let cli = cli::Cli::parse();

    if cli.version {
        // This is the only way to get a neat --version output.
        // The banner with version is already printed just prior to this before clap parses arguments,
        // so we can just exit successfully here after echoing the licenses.
        println!(
            "This project is dual-licensed under the MIT License and the Apache License (Version 2.0)."
        );
        return process::ExitCode::SUCCESS;
    }

    let settings = match init_settings(&cli) {
        Ok(s) => s,
        Err(code) => return code,
    };

    if cli.show {
        settings.print();
        return process::ExitCode::SUCCESS;
    }

    if let Err(vec) = settings.sanity_check() {
        eprintln!("[!] Configuration has errors:");

        for error in vec {
            eprintln!("  * {error}");
        }

        if settings.dry_run {
            println!("[!] Continuing anyway because --dry-run is set.");
            println!();
        } else {
            return process::ExitCode::from(defaults::exit_codes::CONFIGURATION_ERROR);
        }
    }

    settings.print();
    println!();

    let gpio = match Gpio::new() {
        Ok(g) => g,
        Err(e) => {
            eprintln!("[!] Failed to initialize GPIO: {e}");
            return process::ExitCode::from(defaults::exit_codes::FAILED_TO_INITIALISE_GPIO);
        }
    };

    let pin = match gpio.get(settings.gpio.pin_number) {
        Ok(p) => p.into_input_pullup(),
        Err(e) => {
            eprintln!(
                "[!] Failed to set mode of GPIO{}: {e}",
                settings.gpio.pin_number
            );
            return process::ExitCode::from(defaults::exit_codes::FAILED_TO_SET_PIN_MODE);
        }
    };

    let notifiers = build_notifiers(&settings);

    if notifiers.is_empty() && !settings.dry_run {
        eprintln!("[!] No notifiers are configured.");
        return process::ExitCode::from(defaults::exit_codes::NO_NOTIFIERS_CONFIGURED);
    }

    run_loop(pin, notifiers, settings)
}

/// Builds the list of notifiers based on the resolved settings, creating instances of `TwoLevelNotifier` for each enabled backend (Slack and Batsign) with the appropriate configuration.
fn build_notifiers(settings: &Settings) -> Vec<Box<dyn notifications::Notifier>> {
    let client = Arc::new(Client::new());
    let mut notifiers: Vec<Box<dyn notifications::Notifier>> = Vec::new();

    if settings.slack.enabled {
        for url in &settings.slack.urls {
            let slack = notifications::TwoLevelNotifier::new(
                backend::slack::SlackBackend::new(Arc::clone(&client), url),
                Some(settings.slack.notification_interval),
                settings.slack.retry_interval,
                &settings.slack.alarm_message_template_body,
                &settings.slack.restored_message_template_body,
            );

            notifiers.push(Box::new(slack));
        }
    }

    if settings.batsign.enabled {
        for url in &settings.batsign.urls {
            let batsign = notifications::TwoLevelNotifier::new(
                backend::batsign::BatsignBackend::new(Arc::clone(&client), url),
                Some(settings.batsign.notification_interval),
                settings.batsign.retry_interval,
                &settings.batsign.alarm_message_template_body,
                &settings.batsign.restored_message_template_body,
            );

            notifiers.push(Box::new(batsign));
        }
    }

    notifiers
}

/// The main loop that monitors the GPIO pin and sends notifications based on the configured notifiers and settings.
fn run_loop(
    pin: InputPin,
    mut notifiers: Vec<Box<dyn notifications::Notifier>>,
    settings: Settings,
) -> process::ExitCode {
    let mut low_since: Option<Instant> = None;
    let mut high_since: Option<Instant> = None;
    let mut seen_high = false;

    loop {
        let now = Instant::now();

        match pin.read() {
            Level::Low => {
                let start = low_since.get_or_insert(now);
                let qualified = start.elapsed() >= settings.gpio.hold;

                if settings.debug {
                    println!("LOW");
                }

                if !qualified || !seen_high {
                    thread::sleep(settings.gpio.poll_interval);
                    continue;
                }

                high_since = None;

                let ctx = notifications::Context {
                    level: Level::Low,
                    now,
                    dry_run: settings.dry_run,
                };

                for n in notifiers.iter_mut() {
                    println!("{}", n.name());

                    match n.send_notification(&ctx) {
                        notifications::NotificationResult::NotYetTime => {}
                        notifications::NotificationResult::DryRun => {}
                        notifications::NotificationResult::Success => {
                            println!("Success");
                        }
                        notifications::NotificationResult::Failure(message) => {
                            println!("Failure: {message}");
                        }
                    }
                }
            }
            Level::High => {
                let start = high_since.get_or_insert(now);
                let qualified = start.elapsed() >= settings.gpio.hold;

                if settings.debug {
                    println!("HIGH");
                }

                if !qualified {
                    thread::sleep(settings.gpio.poll_interval);
                    continue;
                }

                low_since = None;

                let ctx = notifications::Context {
                    level: Level::High,
                    now,
                    dry_run: settings.dry_run,
                };

                for n in notifiers.iter_mut() {
                    println!("{}", n.name());

                    match n.send_notification(&ctx) {
                        notifications::NotificationResult::NotYetTime => {}
                        notifications::NotificationResult::DryRun => {}
                        notifications::NotificationResult::Success => {
                            println!("Success");
                        }
                        notifications::NotificationResult::Failure(message) => {
                            println!("Failure: {message}");
                        }
                    }
                }

                if settings.dry_run && notifiers.is_empty() {
                    // In dry run mode, we consider the notification "successful" even if there are no backends configured, since the user just wants to see what would happen.
                    seen_high = true;
                }
            }
        }

        thread::sleep(settings.gpio.poll_interval)
    }
}

/// Initializes the settings by loading defaults, applying the config file, and then applying CLI overrides. If the `--save` flag is set, it saves the resolved configuration back to disk and exits.
fn init_settings(cli: &cli::Cli) -> Result<Settings, process::ExitCode> {
    let mut settings = Settings::default();

    if let Err(e) = settings.inherit_config_dir(&cli.config_dir) {
        eprintln!("[!] Error resolving default configuration directory: {}", e);
        return Err(process::ExitCode::from(
            defaults::exit_codes::FAILED_TO_RESOLVE_CONFIG_DIR,
        ));
    }

    if !settings.paths.config_dir.exists() && !cli.save {
        eprintln!(
            "[!] Configuration directory {} does not exist. Create it or run with `--save` to generate default configuration and resources.",
            settings.paths.config_dir.display()
        );
        return Err(process::ExitCode::from(
            defaults::exit_codes::CONFIG_DIR_DOES_NOT_EXIST,
        ));
    }

    settings.resolve_resource_paths();

    let resource_load_results = settings.load_resources_from_disk();

    if !cli.save && !resource_load_results.is_empty() {
        eprintln!("[!] Failed to load resouces from disk:");

        for (pathbuf, e) in &resource_load_results {
            eprintln!("  * {}: {e}", pathbuf.display());
        }

        if !resource_load_results.is_empty() {
            return Err(process::ExitCode::from(
                defaults::exit_codes::FAILED_TO_LOAD_RESOURCES,
            ));
        }
    }

    let config = match file_config::deserialize_config_file(&settings.paths.config_file) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!(
                "[!] Failed to read configuration file {}: {e}",
                settings.paths.config_file.display()
            );
            return Err(process::ExitCode::from(
                defaults::exit_codes::FAILED_TO_READ_CONFIG_FILE,
            ));
        }
    };

    if !cli.save && config.is_none() {
        eprintln!(
            "[!] No configuration file found at {}. Create it or run with `--save` to generate default configuration and resources.",
            settings.paths.config_file.display()
        );
        return Err(process::ExitCode::from(
            defaults::exit_codes::CONFIG_FILE_DOES_NOT_EXIST,
        ));
    }

    settings.apply_file(&config);
    settings.apply_cli(cli);
    settings.clean_up();

    if cli.save {
        if !settings.paths.config_dir.exists() {
            match fs::create_dir_all(&settings.paths.config_dir) {
                Ok(()) => {
                    println!(
                        "Configuration directory {} created.",
                        settings.paths.config_dir.display()
                    );
                }
                Err(e) => {
                    eprintln!(
                        "[!] Failed to create configuration directory {}: {e}",
                        settings.paths.config_dir.display()
                    );
                    return Err(process::ExitCode::from(
                        defaults::exit_codes::FAILED_TO_CREATE_CONFIG_DIR,
                    ));
                }
            };
        }

        let config = file_config::FileConfig::from(&settings);

        if let Err(e) = confy::store_path(&settings.paths.config_file, config) {
            eprintln!(
                "[!] Failed to write configuration file {}: {e}",
                settings.paths.config_file.display()
            );
            return Err(process::ExitCode::from(
                defaults::exit_codes::FAILED_TO_WRITE_CONFIG_FILE,
            ));
        };

        if fs::write(
            settings.paths.slack_alarm_template,
            &settings.slack.alarm_message_template_body,
        )
        .is_err()
        {
            eprintln!("[!] Failed to write Slack alarm template file.");
            return Err(process::ExitCode::from(
                defaults::exit_codes::FAILED_TO_WRITE_SLACK_ALARM_TEMPLATE,
            ));
        }

        if fs::write(
            settings.paths.slack_restored_template,
            &settings.slack.restored_message_template_body,
        )
        .is_err()
        {
            eprintln!("[!] Failed to write Slack restored template file.");
            return Err(process::ExitCode::from(
                defaults::exit_codes::FAILED_TO_WRITE_SLACK_RESTORED_TEMPLATE,
            ));
        }

        if fs::write(
            settings.paths.batsign_alarm_template,
            &settings.batsign.alarm_message_template_body,
        )
        .is_err()
        {
            eprintln!("[!] Failed to write Batsign alarm template file.");
            return Err(process::ExitCode::from(
                defaults::exit_codes::FAILED_TO_WRITE_BATSIGN_ALARM_TEMPLATE,
            ));
        }

        if fs::write(
            settings.paths.batsign_restored_template,
            &settings.batsign.restored_message_template_body,
        )
        .is_err()
        {
            eprintln!("[!] Failed to write Batsign restored template file.");
            return Err(process::ExitCode::from(
                defaults::exit_codes::FAILED_TO_WRITE_BATSIGN_RESTORED_TEMPLATE,
            ));
        }

        println!(
            "Configuration and resources written successfully to {}.",
            settings.paths.config_dir.display()
        );
        return Err(process::ExitCode::SUCCESS);
    }

    Ok(settings)
}
