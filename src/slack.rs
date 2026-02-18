use reqwest::blocking::Client;
use std::time::Instant;

use crate::notifications::NotificationState;
use crate::settings::Settings;

pub const SLACK_ERROR_EMOJI: &str = ":x:";
pub const SLACK_SUCCESS_EMOJI: &str = ":white_check_mark:";

/// Sends a Slack notification.
fn send_slack_notification_impl(
    client: &Client,
    urls: &[String],
    message: &str,
    emoji: &str,
    dry_run: bool,
) -> Result<Vec<reqwest::StatusCode>, reqwest::Error> {
    let mut statuses = Vec::new();

    let payload = serde_json::json!({
        "text": format!("{} {}", emoji, message)
    });

    if dry_run {
        println!(
            "Dry run: would otherwise have sent Slack notification to {} URLs:",
            urls.len()
        );

        println!("\n{}\n", payload);
        return Ok(statuses);
    }

    for url in urls {
        let res = client.post(url).json(&payload).send()?;
        statuses.push(res.status());
    }

    Ok(statuses)
}

/// Sends a Slack notification if it should. Returns the updated notification state.
pub fn send_slack_notification(
    client: &Client,
    now: Instant,
    settings: &Settings,
    emoji: &str,
    message: &str,
    state: &mut NotificationState,
) -> Result<(), reqwest::Error> {
    state.reset();

    let statuses = match send_slack_notification_impl(
        client,
        &settings.slack.urls,
        message,
        emoji,
        settings.dry_run,
    ) {
        Ok(statuses) => statuses,
        Err(e) => {
            eprintln!("[!] Could not reach Slack: {e}");
            state.previous_failure = Some(now);
            return Err(e);
        }
    };

    if !statuses.is_empty() {
        println!("Slack notifications sent; HTTP statuses: {:?}", statuses);
    }

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

    Ok(())
}
