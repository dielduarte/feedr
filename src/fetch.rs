use std::error::Error as _;
use std::time::Duration;

use chrono::{DateTime, Utc};
use reqwest::StatusCode;
use reqwest::header::{self, HeaderMap};
use url::Url;

use crate::model::Validators;
use crate::parse::{ParsedFeed, parse};

const USER_AGENT: &str = concat!(
    "feedr/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/dielduarte/feedr)"
);

pub enum FetchOutcome {
    NotModified,
    Updated {
        feed: ParsedFeed,
        validators: Validators,
    },
    Failed(FetchError),
}

#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("request timed out")]
    Timeout,
    #[error("network error: {0}")]
    Network(String),
    #[error("server asked to retry later")]
    RetryLater { retry_after: Option<Duration> },
    #[error("server responded with {0}")]
    Http(u16),
    #[error("{0}")]
    Parse(String),
}

impl From<reqwest::Error> for FetchError {
    fn from(error: reqwest::Error) -> Self {
        if error.is_timeout() {
            return Self::Timeout;
        }
        // reqwest's own message is generic ("error sending request"); the cause is in the chain.
        let mut message = error.to_string();
        let mut source = error.source();
        while let Some(cause) = source {
            message.push_str(": ");
            message.push_str(&cause.to_string());
            source = cause.source();
        }
        Self::Network(message)
    }
}

pub struct Fetcher {
    client: reqwest::Client,
}

impl Fetcher {
    pub fn new(timeout: Duration) -> Self {
        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(timeout)
            .build()
            .expect("HTTP client configuration is static and valid");
        Self { client }
    }

    pub async fn fetch(
        &self,
        url: &Url,
        validators: &Validators,
        now: DateTime<Utc>,
    ) -> FetchOutcome {
        let page = match self.get(url, validators, now).await {
            Ok(Some(page)) => page,
            Ok(None) => return FetchOutcome::NotModified,
            Err(error) => return FetchOutcome::Failed(error),
        };
        match parse(&page.body, &page.url, now) {
            Ok(feed) => FetchOutcome::Updated {
                feed,
                validators: page.validators,
            },
            Err(error) => FetchOutcome::Failed(FetchError::Parse(error.to_string())),
        }
    }

    /// Fetches any document, feed or not, so callers can inspect HTML pages.
    pub async fn fetch_page(&self, url: &Url, now: DateTime<Utc>) -> Result<Page, FetchError> {
        // Without validators a server has no reason to answer 304; treat it as a broken response.
        self.get(url, &Validators::default(), now)
            .await?
            .ok_or(FetchError::Http(StatusCode::NOT_MODIFIED.as_u16()))
    }

    /// `None` means the server answered 304 Not Modified.
    async fn get(
        &self,
        url: &Url,
        validators: &Validators,
        now: DateTime<Utc>,
    ) -> Result<Option<Page>, FetchError> {
        let mut request = self.client.get(url.clone());
        if let Some(etag) = &validators.etag {
            request = request.header(header::IF_NONE_MATCH, etag);
        }
        if let Some(last_modified) = &validators.last_modified {
            request = request.header(header::IF_MODIFIED_SINCE, last_modified);
        }

        let response = request.send().await?;
        let status = response.status();
        match status {
            StatusCode::NOT_MODIFIED => return Ok(None),
            StatusCode::TOO_MANY_REQUESTS | StatusCode::SERVICE_UNAVAILABLE => {
                return Err(FetchError::RetryLater {
                    retry_after: retry_after(response.headers(), now),
                });
            }
            _ if !status.is_success() => return Err(FetchError::Http(status.as_u16())),
            _ => {}
        }

        Ok(Some(Page {
            validators: Validators {
                etag: header_string(response.headers(), header::ETAG),
                last_modified: header_string(response.headers(), header::LAST_MODIFIED),
            },
            url: response.url().clone(),
            body: response.bytes().await?.to_vec(),
        }))
    }
}

pub struct Page {
    /// Final URL after redirects.
    pub url: Url,
    pub validators: Validators,
    pub body: Vec<u8>,
}

fn header_string(headers: &HeaderMap, name: header::HeaderName) -> Option<String> {
    headers.get(name)?.to_str().ok().map(str::to_string)
}

/// Retry-After is either a number of seconds or an HTTP date.
fn retry_after(headers: &HeaderMap, now: DateTime<Utc>) -> Option<Duration> {
    let value = headers.get(header::RETRY_AFTER)?.to_str().ok()?.trim();
    if let Ok(seconds) = value.parse() {
        return Some(Duration::from_secs(seconds));
    }
    let at = DateTime::parse_from_rfc2822(value).ok()?;
    Some(
        (at.with_timezone(&Utc) - now)
            .to_std()
            .unwrap_or(Duration::ZERO),
    )
}
