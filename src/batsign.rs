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

    state.update_based_on_statuses(now, &statuses);

    Ok(())
}

/// Extracts email addresses from a list of Batsign URLs, returning them as a comma-separated string.
fn get_emails_from_batsign_urls(urls: &[String]) -> Option<String> {
    let emails: Vec<&str> = urls
        .iter()
        .filter_map(|u| get_email_from_single_batsign_url(u))
        .collect();

    (!emails.is_empty()).then_some(emails.join(", "))
}

/// Extracts an email address from a single Batsign URL, returning it as a `&str`.
fn get_email_from_single_batsign_url(url: &str) -> Option<&str> {
    // https://batsign.me/at/{email}/{token}
    //       ^^          ^  ^       ^       ^?
    let mut parts = url.split('/');

    while let Some(p) = parts.next() {
        if p == "at" {
            let email = parts.next()?;
            return email.contains('@').then_some(email);
        }
    }

    None
}
