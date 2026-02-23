use reqwest::blocking::Client;
use rppal::gpio::Level;

pub struct BatsignBackend;

impl super::Backend for BatsignBackend {
    fn name(&self) -> &'static str {
        "batsign"
    }

    fn build_message(&self, _level: Level, template: &str) -> String {
        template.to_owned()
    }

    fn send_via_backend(
        &self,
        client: &Client,
        url: &str,
        message: String,
    ) -> Result<reqwest::StatusCode, reqwest::Error> {
        let res = client.post(url).body(message).send()?;
        Ok(res.status())
    }
}

/// Extracts email addresses from a list of Batsign URLs, returning them as a comma-separated string.
#[cfg(false)]
fn get_emails_from_batsign_urls(urls: &[String]) -> Option<String> {
    let emails: Vec<&str> = urls
        .iter()
        .filter_map(|u| get_email_from_single_batsign_url(u))
        .collect();

    (!emails.is_empty()).then_some(emails.join(", "))
}

/// Extracts an email address from a single Batsign URL, returning it as a `&str`.
#[cfg(false)]
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

#[cfg(false)]
#[cfg(test)]
mod tests {
    #[test]
    fn test_get_emails_from_batsign_urls() {
        let vec = vec![
            "https://batsign.me/at/test@example.com/token".to_string(),
            "https://batsign.me/at/example@test.com/token".to_string(),
            "https://batsign.me/at/blork/token".to_string(),
            "https://batsign.me/".to_string(),
            "".to_string(),
        ];

        let expected = Some("test@example.com, example@test.com".to_string());
        let emails = super::get_emails_from_batsign_urls(&vec);
        assert_eq!(emails, expected);
    }

    #[test]
    fn test_get_email_from_single_batsign_url() {
        let url = "https://batsign.me/at/test@example.com/token";
        let email = super::get_email_from_single_batsign_url(url);
        assert_eq!(email, Some("test@example.com"));

        let url = "https://batsign.me/at/example@test.com/token";
        let email = super::get_email_from_single_batsign_url(url);
        assert_eq!(email, Some("example@test.com"));

        let url = "https://batsign.me/at/blork/token";
        let email = super::get_email_from_single_batsign_url(url);
        assert_eq!(email, None);

        let url = "https://batsign.me/";
        let email = super::get_email_from_single_batsign_url(url);
        assert_eq!(email, None);

        let url = "";
        let email = super::get_email_from_single_batsign_url(url);
        assert_eq!(email, None);
    }
}
