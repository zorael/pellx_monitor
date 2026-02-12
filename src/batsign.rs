use reqwest::blocking::Client;
use std::time::{Duration, Instant};

/// Determines if a Batsign should be sent, based on the last successful and failed timestamps.
pub fn should_send_batsign(
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

/// Constructs a Batsign message body.
pub fn get_batsign_message(subject: &str) -> String {
    format!("Subject: {subject}\n")
}

/// Sends a batsign message to the specified URL, returning the HTTP status code or an error.
pub fn send_batsign(
    client: &Client,
    url: &str,
    message: &str,
) -> Result<reqwest::StatusCode, reqwest::Error> {
    let res = client.post(url).body(message.to_string()).send()?;
    Ok(res.status())
}
