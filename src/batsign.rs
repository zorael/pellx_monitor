use reqwest::blocking::Client;
use std::time::Instant;

use crate::notifications::NotificationState;
use crate::settings::Settings;

/// Sends a batsign message to the specified URL, returning the HTTP status code or an error.
pub fn send_batsign_notification_impl(
    client: &Client,
    urls: &Vec<String>,
    message: &str,
    dry_run: bool,
) -> Result<Vec<reqwest::StatusCode>, reqwest::Error> {
    let mut statuses = Vec::new();

    if dry_run {
        println!(
            "Dry run: would otherwise have sent Batsign notification to {} URLs:",
            urls.len()
        );
        println!("\n{}\n", message);
        return Ok(statuses);
    }

    for url in urls {
        let res = client.post(url).body(message.to_string()).send()?;
        statuses.push(res.status());
    }

    Ok(statuses)
}

/// Sends a Batsign notification if it should. Returns the updated notification state.
pub fn send_batsign_notification(
    client: &Client,
    now: Instant,
    settings: &Settings,
    message: &str,
    state: &NotificationState,
) -> Result<NotificationState, reqwest::Error> {
    let mut state = state.clone().reset();

    let statuses = match send_batsign_notification_impl(
        client,
        &settings.batsign.urls,
        message,
        settings.dry_run,
    ) {
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
