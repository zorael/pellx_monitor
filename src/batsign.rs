use reqwest::blocking::Client;
use std::time::Instant;

use crate::notifications::NotificationState;
use crate::settings::Settings;

/// Sends a batsign message to the specified URL, returning the HTTP status code or an error.
pub fn send_batsign_notification_impl(
    client: &Client,
    urls: &Vec<String>,
    message: &str,
) -> Result<Vec<reqwest::StatusCode>, reqwest::Error> {
    let mut statuses = Vec::new();

    for url in urls {
        let res = client.post(url).body(message.to_string()).send()?;
        statuses.push(res.status());
    }

    Ok(statuses)
}

pub fn should_send_batsign_notification(
    now: Instant,
    settings: &Settings,
    state: &NotificationState,
) -> bool {
    if settings.batsign.urls.is_empty() {
        return false;
    }

    match state.previous_failure {
        Some(failure_time) if now.duration_since(failure_time) < state.retry_delay => {
            return false;
        }
        _ => {}
    }

    match state.previous {
        Some(last) if now.duration_since(last) < state.repeat_interval => {
            return false;
        }
        _ => {}
    }

    if settings.debug {
        println!("...should send Batsign notification!");
    }

    true
}

/// Sends a Batsign notification if it should. Returns the updated notification state.
pub fn send_batsign_notification(
    client: &Client,
    now: Instant,
    settings: &Settings,
    message: &str,
    state: &NotificationState,
) -> Result<NotificationState, reqwest::Error> {
    let mut state = state.clone();
    state.reset();

    if settings.dry_run {
        println!("Dry run: would otherwise have sent Batsign notification");
        state.previous = Some(now);
        return Ok(state);
    }

    let statuses = match send_batsign_notification_impl(client, &settings.batsign.urls, message) {
        Ok(statuses) => statuses,
        Err(e) => {
            eprintln!("[!] Could not reach Batsign: {e}");
            state.previous_failure = Some(now);
            return Err(e);
        }
    };

    println!("Batsigns sent; HTTP statuses: {:?}", statuses);

    let mut num_errors: u8 = 0;

    for status in statuses {
        if !status.is_success() {
            num_errors += 1;
        }
    }

    if num_errors == 0 {
        state.previous = Some(now);
    } else {
        state.previous_failure = Some(now);
    }

    Ok(state)
}
