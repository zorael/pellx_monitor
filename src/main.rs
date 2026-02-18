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
use std::time::Instant;
use std::{fs, process, thread};

use crate::notifications::NotificationState;

/// Program entrypoint.
fn main() -> process::ExitCode {
    if !cfg!(target_os = "linux") {
        eprintln!("[!] This program can only be run on Linux.");
        return process::ExitCode::FAILURE;
    }

    print_banner();
    println!();

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

    let pin = match gpio.get(settings.gpio.pin_number) {
        Ok(p) => p.into_input_pullup(),
        Err(e) => {
            eprintln!(
                "[!] Failed to set mode of GPIO{}: {e}",
                settings.gpio.pin_number
            );
            return process::ExitCode::FAILURE;
        }
    };

    let client = Client::new();

    let slack_is_correctly_configured = !settings.slack.webhook_url.is_empty()
        && settings.slack.webhook_url != defaults::slack::DUMMY_WEBHOOK_URL;
    let batsign_is_correctly_configured = !settings.batsign.urls.is_empty();

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
    let mut still_in_initial_state = true;

    loop {
        match pin.read() {
            Level::Low => {
                // OK (closed): pull-up is overridden, LOW
                let start = low_since.get_or_insert_with(Instant::now);
                let qualified = start.elapsed() >= settings.gpio.hold;

                if settings.debug {
                    println!("LOW");
                }

                if !qualified || still_in_initial_state {
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
                    let message = &notifications::format_notification_message(
                        settings.slack.restored_message_template_body.as_str(),
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

                if should_send_batsign_notification {
                    let message = &notifications::format_notification_message(
                        settings.batsign.restored_message_template_body.as_str(),
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
                    let message = &notifications::format_notification_message(
                        settings.slack.alarm_message_template_body.as_str(),
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

                if should_send_batsign_notification {
                    let message = &notifications::format_notification_message(
                        settings.batsign.alarm_message_template_body.as_str(),
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

        thread::sleep(settings.gpio.poll_interval);
    }
}

/// Initializes the settings by loading defaults, applying the config file, and then applying CLI overrides. If the `--save` flag is set, it saves the resolved configuration back to disk and exits.
fn init_settings(cli: &cli::Cli) -> Result<settings::Settings, process::ExitCode> {
    let mut settings = settings::Settings::default().with_resource_dir(&cli.resource_dir);

    if !settings.paths.resource_dir.exists() && !cli.save {
        eprintln!(
            "[!] Resource directory `{}` does not exist. Create it or run with `--save` to generate default configuration and resources.",
            settings.paths.resource_dir.display()
        );
        return Err(process::ExitCode::FAILURE);
    }

    settings.resolve_resource_paths();

    let resource_load_results = settings.load_resources_from_disk();

    if !cli.save {
        for (pathbuf, e) in &resource_load_results {
            eprintln!("[!] Failed to load resource `{}`: {e}", pathbuf.display());
        }

        if !resource_load_results.is_empty() {
            return Err(process::ExitCode::FAILURE);
        }
    }

    let config = match config::deserialize_config_file(&settings) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!(
                "[!] Failed to read configuration file `{}`: {e}",
                settings.paths.config_file.display()
            );
            return Err(process::ExitCode::FAILURE);
        }
    };

    settings = settings::apply_file(settings, &config);
    settings = settings::apply_cli(settings, cli);

    if cli.save {
        if !settings.paths.resource_dir.exists() {
            match fs::create_dir_all(&settings.paths.resource_dir) {
                Ok(()) => {
                    println!(
                        "Resource directory `{}` created.",
                        settings.paths.resource_dir.display()
                    );
                }
                Err(e) => {
                    eprintln!(
                        "[!] Failed to create resource directory `{}`: {e}",
                        settings.paths.resource_dir.display()
                    );
                    return Err(process::ExitCode::FAILURE);
                }
            };
        }

        let config = config::FileConfig::from(&settings);

        if confy::store_path(&settings.paths.config_file, config).is_err() {
            eprintln!("[!] Failed to write configuration file.");
            return Err(process::ExitCode::FAILURE);
        };

        if fs::write(
            settings.paths.slack_alarm_template,
            &settings.slack.alarm_message_template_body,
        )
        .is_err()
        {
            eprintln!("[!] Failed to write Slack alarm template file.");
            return Err(process::ExitCode::FAILURE);
        }

        if fs::write(
            settings.paths.slack_restored_template,
            &settings.slack.restored_message_template_body,
        )
        .is_err()
        {
            eprintln!("[!] Failed to write Slack restored template file.");
            return Err(process::ExitCode::FAILURE);
        }

        if fs::write(
            settings.paths.batsign_urls,
            settings.batsign.urls.join("\n"),
        )
        .is_err()
        {
            eprintln!("[!] Failed to write Batsigns URL file.");
            return Err(process::ExitCode::FAILURE);
        }

        if fs::write(
            settings.paths.batsign_alarm_template,
            &settings.batsign.alarm_message_template_body,
        )
        .is_err()
        {
            eprintln!("[!] Failed to write Batsign alarm template file.");
            return Err(process::ExitCode::FAILURE);
        }

        if fs::write(
            settings.paths.batsign_restored_template,
            &settings.batsign.restored_message_template_body,
        )
        .is_err()
        {
            eprintln!("[!] Failed to write Batsign restored template file.");
            return Err(process::ExitCode::FAILURE);
        }

        println!(
            "Configuration and resources written successfully to `{}`.",
            settings.paths.resource_dir.display()
        );
        return Err(process::ExitCode::SUCCESS);
    }

    Ok(settings)
}

/// Prints the program banner with version information.
fn print_banner() {
    let banner = format!("{} {}", defaults::PROGRAM_NAME, defaults::VERSION);
    println!("{}\n{}", banner, "=".repeat(banner.len()));
}
