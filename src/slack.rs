use reqwest::blocking::Client;
use std::time::Instant;

use crate::notifications::NotificationState;
use crate::settings::Settings;
use crate::defaults;

pub const SLACK_ERROR_EMOJI: &str = ":x:";
pub const SLACK_SUCCESS_EMOJI: &str = ":white_tick:";

fn send_slack_notification(
    client: &Client,
    slack_webhook_url: &str,
    message: &str,
    emoji: &str,
) -> Result<(), reqwest::Error> {
    let payload = serde_json::json!({
        "text": format!("{} {}", emoji, message)
    });

    client
        .post(slack_webhook_url)
        .body(payload.to_string())
        .send()?;
    Ok(())
}

pub fn maybe_send_slack_notification(
    client: &Client,
    now: Instant,
    settings: &Settings,
    emoji: &str,
    body: &str,
    state: &NotificationState,
) -> Result<NotificationState, reqwest::Error> {
    if settings.slack_webhook_url.is_empty() || settings.slack_webhook_url == defaults::SLACK_WEBHOOK_URL_PLACEHOLDER {
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
        println!("...should send restored notification!");
    }

    let mut state = state.clone();
    state.reset();

    if settings.dry_run {
        println!("Dry run: would otherwise have sent alarm Slack notification");
        state.previous = Some(now);
        return Ok(state.clone());
    }

    match &settings.slack_webhook_url {
        url if url.is_empty() => {}
        url if url == defaults::SLACK_WEBHOOK_URL_PLACEHOLDER => {}
        url => {
            match send_slack_notification(
                client,
                url,
                body,
                emoji,
            ) {
                Ok(()) => {
                    println!("Sent Slack notification");
                    state.previous = Some(now);
                },
                Err(e) => {
                    state.previous_failure = Some(now);
                    return Err(e);
                }
            }
        }
    }

    Ok(state.clone())
}
