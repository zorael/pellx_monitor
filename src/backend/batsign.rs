use reqwest::blocking::Client;
use rppal::gpio::Level;
use std::sync::Arc;

/// Defines the Batsign backend for sending notifications via email using the Batsign service.
pub struct BatsignBackend {
    /// Unique identifier for the Batsign backend instance, used for logging and identification purposes.
    id: usize,

    /// HTTP client used to send requests to the Batsign service.
    client: Arc<Client>,

    /// Batsign URL to which the notification will be sent.
    url: String,
}

impl BatsignBackend {
    /// Creates a new instance of the BatsignBackend with the provided HTTP client and Batsign URL.
    pub fn new(id: usize, client: Arc<Client>, url: &str) -> Self {
        Self {
            id,
            client,
            url: url.to_owned(),
        }
    }
}

impl super::Backend for BatsignBackend {
    /// Returns the name of the backend, which is "batsign" in this case.
    fn name(&self) -> String {
        // This can be cached if it turns out to be a hotspot.
        format!(
            "batsign#{}:{}",
            self.id,
            get_email_from_batsign_url(&self.url).unwrap_or("(failed to parse email)")
        )
    }

    /// Builds the message to be sent via Batsign.
    fn build_message(&self, _level: Level, template: &str) -> String {
        template.to_owned()
    }

    /// Sends a notification via the Batsign backend by making a POST request to the specified URL with the message as the body.
    fn send_message(&mut self, message: &str) -> Result<(), String> {
        match self.client.post(&self.url).body(message.to_owned()).send() {
            Ok(resp) if resp.status().is_success() => Ok(()),
            Ok(resp) => Err(format!("HTTP {}", resp.status())),
            Err(e) => Err(e.to_string()),
        }
    }
}

/// Extracts an email address from a single Batsign URL, returning it as a `&str`.
fn get_email_from_batsign_url(url: &str) -> Option<&str> {
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

#[cfg(test)]
mod tests {
    #[test]
    fn test_get_email_from_batsign_url() {
        let url = "https://batsign.me/at/test@example.com/token";
        let email = super::get_email_from_batsign_url(url);
        assert_eq!(email, Some("test@example.com"));

        let url = "https://batsign.me/at/example@test.com/token";
        let email = super::get_email_from_batsign_url(url);
        assert_eq!(email, Some("example@test.com"));

        let url = "https://batsign.me/at/blork/token";
        let email = super::get_email_from_batsign_url(url);
        assert_eq!(email, None);

        let url = "https://batsign.me/";
        let email = super::get_email_from_batsign_url(url);
        assert_eq!(email, None);

        let url = "";
        let email = super::get_email_from_batsign_url(url);
        assert_eq!(email, None);
    }
}
