use reqwest::blocking::Client;
use std::time::{Duration, Instant};

use crate::settings::Settings;

/// Determines if an alarm Batsign should be sent, based on the last successful and failed timestamps.
pub fn should_send_alarm_batsign(
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
pub fn should_send_restored_batsign(
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

/// Constructs a Batsign message body.
pub fn format_batsign_message(template: &str, settings: &Settings, since: &Instant) -> String {
    template
        .replace(
            "{since}",
            &humantime::format_duration(since.elapsed()).to_string(),
        )
        .replace("{pin_number}", &settings.pin_number.to_string())
        .replace(
            "{poll_interval}",
            &humantime::format_duration(settings.poll_interval).to_string(),
        )
        .replace(
            "{hold}",
            &humantime::format_duration(settings.hold).to_string(),
        )
        .replace(
            "{time_between_batsigns}",
            &humantime::format_duration(settings.time_between_batsigns).to_string(),
        )
        .replace(
            "{time_between_batsigns_retry}",
            &humantime::format_duration(settings.time_between_batsigns_retry).to_string(),
        )
}

/// Sends a batsign message to the specified URL, returning the HTTP status code or an error.
pub fn send_batsign(
    client: &Client,
    urls: &Vec<String>,
    message: String,
) -> Result<Vec<reqwest::StatusCode>, reqwest::Error> {
    let mut statuses = Vec::new();

    for url in urls {
        let res = client.post(url).body(message.clone()).send()?;
        statuses.push(res.status());
    }

    Ok(statuses)
}
