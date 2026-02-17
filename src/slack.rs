use reqwest::blocking::Client;
use std::time::Instant;

use crate::defaults;
use crate::notifications::NotificationState;
use crate::settings::Settings;

pub const SLACK_ERROR_EMOJI: &str = ":x:";
pub const SLACK_SUCCESS_EMOJI: &str = ":white_tick:";

/// Sends a Slack notification.
fn send_slack_notification_impl(
    client: &Client,
    slack_webhook_url: &str,
    message: &str,
    emoji: &str,
    dry_run: bool,
) -> Result<(), reqwest::Error> {
    let payload = serde_json::json!({
        "text": format!("{} {}", emoji, message)
    });

    if dry_run {
        println!("Dry run: would otherwise have sent Slack notification");
        println!("\n{}\n", payload);
        return Ok(());
    }

    client
        .post(slack_webhook_url)
        .body(payload.to_string())
        .send()?;
    Ok(())
}

pub fn should_send_slack_notification(
    now: Instant,
    settings: &Settings,
    state: &NotificationState,
) -> bool {
    if settings.slack.webhook_url.is_empty()
        || settings.slack.webhook_url == defaults::slack::DUMMY_WEBHOOK_URL
    {
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
        println!("...should send restored notification!");
    }

    true
}

/// Sends a Slack notification if it should. Returns the updated notification state.
pub fn send_slack_notification(
    client: &Client,
    now: Instant,
    settings: &Settings,
    emoji: &str,
    message: &str,
    state: &NotificationState,
) -> Result<NotificationState, reqwest::Error> {
    let mut state = state.clone();
    state.reset();

    match &settings.slack.webhook_url {
        url if url.is_empty() => {}
        url if url == defaults::slack::DUMMY_WEBHOOK_URL => {}
        url => match send_slack_notification_impl(client, url, message, emoji, settings.dry_run) {
            Ok(()) => {
                println!("Sent Slack notification");
                state.previous = Some(now);
            }
            Err(e) => {
                state.previous_failure = Some(now);
                return Err(e);
            }
        },
    }

    Ok(state)
}
