use reqwest::blocking::Client;
use std::time::Instant;

use crate::notifications::NotificationState;
use crate::settings::Settings;

/// Sends a batsign message to the specified URL, returning the HTTP status code or an error.
pub fn send_batsign_notification(
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

pub fn maybe_send_batsign_notification(
    client: &Client,
    now: Instant,
    settings: &Settings,
    body: &str,
    state: &NotificationState,
) -> Result<NotificationState, reqwest::Error> {
    if settings.batsign_urls.is_empty() {
        return Ok(state.clone());
    }

    match state.previous_failure {
        Some(failure_time) if now.duration_since(failure_time) < state.retry_delay => {
            return Ok(state.clone());
        }
        _ => {},
    }

    match state.previous {
        Some(last) if now.duration_since(last) < state.repeat_interval => {
            return Ok(state.clone());
        },
        _ => {},
    }

    if settings.debug {
        println!("...should send Batsign notification!");
    }

    let mut state = state.clone();
    state.reset();

    if settings.dry_run {
        println!("Dry run: would otherwise have sent Batsign notification");
        state.previous = Some(now);
        return Ok(state.clone());
    }

    let statuses = match send_batsign_notification(
        client,
        &settings.batsign_urls,
        body,
    ) {
        Ok(statuses) => statuses,
        Err(e) => {
            eprintln!("[!] Could not reach Batsign: {e}");
            state.previous_failure = Some(now);
            return Err(e);
        }
    };

    for status in statuses {
        if status.is_success() {
            println!("Batsign sent; HTTP {status}");
            state.previous = Some(now);
            state.previous_failure = None;
        } else {
            eprintln!("[!] Batsign returned error; HTTP {status}");
            state.previous_failure = Some(now);
        }
    }

    Ok(state.clone())
}
