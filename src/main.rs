mod batsign;
mod cli;
mod config;
mod defaults;
mod settings;
mod slack;

use clap::Parser;
use reqwest::blocking::Client;
use rppal::gpio::{Gpio, Level};
use std::fs;
use std::time::{Duration, Instant};
use std::{process, thread};

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

    let mut high_since: Option<Instant> = None;
    let mut low_since: Option<Instant> = None;
    let mut last_alarm_slack: Option<Instant> = None;
    let mut last_restored_slack: Option<Instant> = None;
    let mut last_alarm_batsign: Option<Instant> = None;
    let mut last_failed_alarm_batsign: Option<Instant> = None;
    let mut last_restored_batsign: Option<Instant> = None;
    let mut last_failed_restored_batsign: Option<Instant> = None;
    let mut flips: u32 = 0;

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

                if last_restored_slack.is_none() {
                    if settings.dry_run {
                        println!("Dry run: would otherwise have sent alarm Slack notification");

                        last_restored_slack = Some(now);
                        last_alarm_slack = None;
                        thread::sleep(settings.poll_interval);
                        continue;
                    }

                    match &settings.slack_webhook_url {
                        url if url.is_empty() => {}
                        url if url == defaults::SLACK_WEBHOOK_URL_PLACEHOLDER => {}
                        url => {
                            match slack::send_slack_notification(
                                &client,
                                url,
                                "Pellets tycks vid liv igen",
                                slack::SLACK_SUCCESS_EMOJI,
                            ) {
                                Ok(()) => println!("Sent Slack notification"),
                                Err(e) => eprintln!("[!] Failed to send Slack notification: {e}"),
                            }

                            last_restored_slack = Some(now);
                            last_alarm_slack = None;
                            flips += 1;
                        }
                    }
                }

                if should_send_restored_notification(
                    now,
                    last_restored_batsign,
                    last_failed_restored_batsign,
                    settings.time_between_batsigns_retry,
                ) {
                    flips += 1;

                    if settings.debug {
                        println!("...should send restored notification!");
                    }

                    if settings.dry_run {
                        println!("Dry run: would otherwise have sent restored notification");
                        last_restored_batsign = Some(now);
                        last_failed_restored_batsign = None;
                        last_alarm_batsign = None;
                        last_failed_alarm_batsign = None;
                        thread::sleep(settings.poll_interval);
                        continue;
                    }

                    let batsign_restored_message = batsign::format_batsign_message(
                        &settings.batsign_restored_template_body,
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

                if last_alarm_slack.is_none() {
                    if settings.dry_run {
                        println!("Dry run: would otherwise have sent alarm Slack notification");

                        last_alarm_slack = Some(now);
                        last_restored_slack = None;
                        thread::sleep(settings.poll_interval);
                        continue;
                    }

                    match &settings.slack_webhook_url {
                        url if url.is_empty() => {}
                        url if url == defaults::SLACK_WEBHOOK_URL_PLACEHOLDER => {}
                        url => {
                            match slack::send_slack_notification(
                                &client,
                                url,
                                "Pellets död?",
                                slack::SLACK_ERROR_EMOJI,
                            ) {
                                Ok(()) => println!("Sent Slack notification"),
                                Err(e) => eprintln!("[!] Failed to send Slack notification: {e}"),
                            }

                            last_alarm_slack = Some(now);
                            last_restored_slack = None;
                            flips += 1;
                        }
                    }
                }

                if should_send_alarm_notification(
                    now,
                    last_alarm_batsign,
                    last_failed_alarm_batsign,
                    settings.time_between_batsigns,
                    settings.time_between_batsigns_retry,
                ) {
                    flips += 1;

                    if settings.debug {
                        println!("...should send notification!");
                    }

                    if settings.dry_run {
                        println!("Dry run: would otherwise have sent alarm notification");

                        last_alarm_batsign = Some(now);
                        last_failed_alarm_batsign = None;
                        last_restored_batsign = None;
                        last_failed_restored_batsign = None;
                        thread::sleep(settings.poll_interval);
                        continue;
                    }

                    let batsign_alarm_message = batsign::format_batsign_message(
                        &settings.batsign_alarm_template_body,
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

/// Determines if an alarm Batsign should be sent, based on the last successful and failed timestamps.
pub fn should_send_alarm_notification(
    now: Instant,
    last: Option<Instant>,
    last_failed: Option<Instant>,
    time_between_batsigns: Duration,
    time_between_batsigns_retry: Duration,
) -> bool {
    if let Some(last_failed) = last_failed {
        return now.duration_since(last_failed) >= time_between_batsigns_retry;
    }

    if let Some(last) = last {
        now.duration_since(last) >= time_between_batsigns
    } else {
        true
    }
}

/// Determines if a restored Batsign should be sent, based on the last successful and failed timestamps.
pub fn should_send_restored_notification(
    now: Instant,
    last: Option<Instant>,
    last_failed: Option<Instant>,
    time_between_batsigns_retry: Duration,
) -> bool {
    if let Some(last_failed) = last_failed {
        return now.duration_since(last_failed) >= time_between_batsigns_retry;
    }

    last.is_none()
}
