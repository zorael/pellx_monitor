use reqwest::blocking::Client;
use std::time::Instant;

use crate::notifications::NotificationState;
use crate::settings::Settings;

/// Sends a batsign message to the specified URL, returning the HTTP status code or an error.
pub fn send_batsign_notification_impl(
    client: &Client,
    urls: &[String],
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
    state: &mut NotificationState,
) -> Result<(), reqwest::Error> {
    //state.reset();

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

    if !statuses.is_empty() {
        if let Some(emails) = get_emails_from_batsign_urls(&settings.batsign.urls) {
            println!("Batsigns sent to: {:?}", emails);
        } else {
            println!("Batsigns sent to URLs: {:?}", settings.batsign.urls);
        }

        println!("HTTP statuses: {:?}", statuses);
    }

    let mut num_errors: usize = 0;

    for status in statuses {
        if !status.is_success() {
            num_errors += 1;
        }
    }

    if num_errors == 0 {
        state.previous = Some(now);
        state.previous_failure = None;
    } else {
        state.previous_failure = Some(now);
    }

    Ok(())
}

/// Extracts email addresses from a list of Batsign URLs, returning them as a comma-separated string.
fn get_emails_from_batsign_urls(urls: &[String]) -> Option<String> {
    let mut emails = Vec::new();

    for url in urls {
        if let Some(email) = get_email_from_single_batsign_url(url) {
            emails.push(email);
        }
    }

    if emails.is_empty() {
        None
    } else {
        Some(emails.join(", "))
    }
}

/// Extracts an email address from a single Batsign URL, returning it as a string. This assumes that the Batsign URL is in the format "https://batsign.io/{email}/{token}".
fn get_email_from_single_batsign_url(url: &str) -> Option<String> {
    let s = String::from(url);
    let splits = s.split('/').collect::<Vec<&str>>();
    // https://batsign.me/at/{email}/{token}
    //       ^^          ^  ^       ^       ^?
    //       01          2  3       4       5
    //                      ---[4]---

    if splits.len() < 6 {
        return None;
    }

    // Verify that the email has an '@' symbol in it
    match splits[4].contains("@") {
        true => Some(splits[4].to_string()),
        false => None,
    }
}
