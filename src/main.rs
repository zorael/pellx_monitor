mod batsign;
mod cli;
mod defaults;
mod file_config;
mod notifications;
mod settings;
mod slack;

use clap::Parser;
use reqwest::blocking::Client;
use rppal::gpio::{Gpio, Level};
use std::sync::Arc;
use std::time::Instant;
//use std::time::Duration;
use std::{fs, process, thread};

//use crate::notifications::NotificationState;

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

        return process::ExitCode::from(defaults::exit_codes::CONFIGURATION_ERROR);
    }

    settings.print();
    println!();

    //run_dyn_monitor_loop(&settings);
    //run_monitor_loop(settings)
    run_backend_loop(settings)
}

fn run_backend_loop(settings: settings::Settings) -> process::ExitCode {
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

    //let client = Client::new();
    let client = Arc::new(Client::new());

    let mut backends: Vec<Box<dyn notifications::Notifier>> = Vec::new();

    if settings.slack.enabled {
        for url in &settings.slack.urls {
            let backend = notifications::TwoLevelNotifier::new(
                notifications::SlackBackend,
                url,
                Arc::clone(&client),
                Some(settings.slack.notification_interval),
                settings.slack.retry_interval,
                &settings.slack.alarm_message_template_body,
                &settings.slack.restored_message_template_body,
            );
            backends.push(Box::new(backend));
        }
    }

    if settings.batsign.enabled {
        for url in &settings.batsign.urls {
            let backend = notifications::TwoLevelNotifier::new(
                notifications::BatsignBackend,
                url,
                Arc::clone(&client),
                Some(settings.batsign.notification_interval),
                settings.batsign.retry_interval,
                &settings.batsign.alarm_message_template_body,
                &settings.batsign.restored_message_template_body,
            );
            backends.push(Box::new(backend));
        }
    }

    if backends.is_empty() {
        return process::ExitCode::FAILURE;
    }

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
                    //elapsed: start.elapsed(),
                    dry_run: settings.dry_run,
                };

                for b in backends.iter_mut() {
                    println!("{}", b.name());

                    if let Some(results) = b.send_notification(&ctx) {
                        println!("{}", results.num_succeeded);
                        println!("{}", results.num_failed);
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
                    //elapsed: start.elapsed(),
                    dry_run: settings.dry_run,
                };

                for b in backends.iter_mut() {
                    println!("{}", b.name());

                    if let Some(results) = b.send_notification(&ctx) {
                        println!("{}", results.num_succeeded);
                        println!("{}", results.num_failed);

                        if results.num_succeeded > 0 {
                            // Only count this as having "seen" a HIGH if we
                            // successfully sent at least one notification about it,
                            // to avoid weird edge cases where the pin is HIGH but
                            // we can't reach the notification endpoints for some reason.
                            seen_high = true;
                        }
                    }
                }
            }
        }

        thread::sleep(settings.gpio.poll_interval);
    }
}

/*fn run_dyn_monitor_loop(settings: &settings::Settings) -> process::ExitCode {
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

    //let client = Client::new();
    let client = Arc::new(Client::new());

    let mut notifiers: Vec<Box<dyn notifications::Notifier>> = Vec::new();

    if settings.slack.enabled {
        for url in &settings.slack.urls {
            let notifier = notifications::SlackNotifier::new(
                url,
                Arc::clone(&client),
                Some(settings.slack.notification_interval),
                settings.slack.retry_interval,
                &settings.slack.alarm_message_template_body,
                &settings.slack.restored_message_template_body,
            );

            notifiers.push(Box::new(notifier));
        }
    }

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
                    elapsed: start.elapsed(),
                    dry_run: settings.dry_run,
                };

                for n in notifiers.iter_mut() {
                    println!("{}", n.name());
                    let results = n.send_notification(&ctx);
                    println!("{}", results.num_succeeded);
                    println!("{}", results.num_failed);
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
                    elapsed: start.elapsed(),
                    dry_run: settings.dry_run,
                };

                for n in notifiers.iter_mut() {
                    println!("{}", n.name());
                    let results = n.send_notification(&ctx);
                    println!("{}", results.num_succeeded);
                    println!("{}", results.num_failed);

                    if results.num_succeeded > 0 {
                        // Only count this as having "seen" a HIGH if we
                        // successfully sent at least one notification about it,
                        // to avoid weird edge cases where the pin is HIGH but
                        // we can't reach the notification endpoints for some reason.
                        seen_high = true;
                    }
                }
            }
        }
    }
}*/

// Runs the main monitor loop. This function will block indefinitely, monitoring the GPIO pin and sending notifications as configured.
/*fn run_monitor_loop(settings: settings::Settings) -> process::ExitCode {
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

    let client = Client::new();

    let slack_is_correctly_configured =
        settings.dry_run || (settings.slack.enabled && !settings.slack.urls.is_empty());

    let batsign_is_correctly_configured =
        settings.dry_run || (settings.batsign.enabled && !settings.batsign.urls.is_empty());

    let mut slack_low_state = NotificationState::new(None, settings.slack.retry_interval);
    let mut slack_high_state = NotificationState::new(
        Some(settings.slack.notification_interval),
        settings.slack.retry_interval,
    );

    let mut batsign_low_state = NotificationState::new(None, settings.batsign.retry_interval);
    let mut batsign_high_state = NotificationState::new(
        Some(settings.batsign.notification_interval),
        settings.batsign.retry_interval,
    );

    let mut low_since: Option<Instant> = None;
    let mut high_since: Option<Instant> = None;
    let mut seen_alarm = false;

    loop {
        match pin.read() {
            Level::Low => {
                // OK (closed): pull-up is overridden, LOW
                let start = low_since.get_or_insert_with(Instant::now);
                let qualified = start.elapsed() >= settings.gpio.hold;

                if settings.debug {
                    println!("LOW");
                }

                if !qualified || !seen_alarm {
                    thread::sleep(settings.gpio.poll_interval);
                    continue;
                }

                let now = Instant::now();
                let low_since =
                    low_since.expect("low_since should have been set with .get_or_insert_with");
                high_since = None;

                let should_send_slack_notification = slack_is_correctly_configured
                    && notifications::should_send_notification(now, &slack_low_state);

                let should_send_batsign_notification = batsign_is_correctly_configured
                    && notifications::should_send_notification(now, &batsign_low_state);

                if should_send_slack_notification {
                    slack_high_state.reset();

                    let message = notifications::format_notification_message(
                        settings.slack.restored_message_template_body.as_str(),
                        &settings,
                        &low_since,
                    );

                    if let Err(e) = slack::send_slack_notification(
                        &client,
                        now,
                        &settings,
                        slack::SLACK_SUCCESS_EMOJI,
                        &message,
                        &mut slack_low_state,
                    ) {
                        eprintln!("[!] Failed to send Slack notification: {e}");
                    };
                }

                if should_send_batsign_notification {
                    batsign_high_state.reset();

                    let message = notifications::format_notification_message(
                        settings.batsign.restored_message_template_body.as_str(),
                        &settings,
                        &low_since,
                    );

                    if let Err(e) = batsign::send_batsign_notification(
                        &client,
                        now,
                        &settings,
                        &message,
                        &mut batsign_low_state,
                    ) {
                        eprintln!("[!] Failed to send Batsign notification: {e}");
                    };
                }
            }
            Level::High => {
                // ALARM (open): internal pull-up pulls to HIGH
                let start = high_since.get_or_insert_with(Instant::now);
                let qualified = start.elapsed() >= settings.gpio.hold;

                if settings.debug {
                    println!("HIGH");
                }

                if !qualified {
                    thread::sleep(settings.gpio.poll_interval);
                    continue;
                }

                let now = Instant::now();
                let high_since =
                    high_since.expect("high_since should have been set with .get_or_insert_with");
                low_since = None;

                let should_send_slack_notification = slack_is_correctly_configured
                    && notifications::should_send_notification(now, &slack_high_state);

                let should_send_batsign_notification = batsign_is_correctly_configured
                    && notifications::should_send_notification(now, &batsign_high_state);

                if should_send_slack_notification {
                    slack_low_state.reset();

                    let message = notifications::format_notification_message(
                        settings.slack.alarm_message_template_body.as_str(),
                        &settings,
                        &high_since,
                    );

                    match slack::send_slack_notification(
                        &client,
                        now,
                        &settings,
                        slack::SLACK_ERROR_EMOJI,
                        &message,
                        &mut slack_high_state,
                    ) {
                        Ok(()) => seen_alarm = true,
                        Err(e) => eprintln!("[!] Failed to send Slack notification: {e}"),
                    };
                }

                if should_send_batsign_notification {
                    batsign_low_state.reset();

                    let message = notifications::format_notification_message(
                        settings.batsign.alarm_message_template_body.as_str(),
                        &settings,
                        &high_since,
                    );

                    match batsign::send_batsign_notification(
                        &client,
                        now,
                        &settings,
                        &message,
                        &mut batsign_high_state,
                    ) {
                        Ok(()) => seen_alarm = true,
                        Err(e) => eprintln!("[!] Failed to send Batsign notification: {e}"),
                    };
                }
            }
        }

        thread::sleep(settings.gpio.poll_interval);
    }
}*/

/// Initializes the settings by loading defaults, applying the config file, and then applying CLI overrides. If the `--save` flag is set, it saves the resolved configuration back to disk and exits.
fn init_settings(cli: &cli::Cli) -> Result<settings::Settings, process::ExitCode> {
    let mut settings = settings::Settings::default();
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

/// Prints the program banner with version information.
fn print_banner() {
    println!(
        "{} {}\n$ git clone {}",
        defaults::PROGRAM_NAME,
        defaults::VERSION,
        defaults::SOURCE_REPOSITORY
    );
}
