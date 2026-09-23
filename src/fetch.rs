use std::error::Error as _;
use std::time::Duration;

use chrono::{DateTime, Utc};
use reqwest::StatusCode;
use reqwest::header::{self, HeaderMap};
use url::Url;

use crate::model::Validators;
use crate::parse::{ParsedFeed, parse};

/// Real feeds are well under this; the cap protects against endless or runaway responses.
const MAX_BODY_BYTES: usize = 10 * 1024 * 1024;
const MAX_REDIRECTS: usize = 10;
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(20);

const USER_AGENT: &str = concat!(
    "feedr/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/dielduarte/feedr)"
);

#[expect(
    clippy::large_enum_variant,
    reason = "short-lived and moved once; boxing would only add an allocation"
)]
pub enum Fetched {
    NotModified,
    Updated {
        feed: ParsedFeed,
        validators: Validators,
        /// Set when every redirect on the way was permanent, so the stored URL should change.
        moved_to: Option<Url>,
    },
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
    #[error("too many redirects")]
    TooManyRedirects,
    #[error("response is larger than {} MB", MAX_BODY_BYTES / 1024 / 1024)]
    TooLarge,
    #[error("{0}")]
    Parse(String),
}

impl FetchError {
    /// Whether a server answered at all, as opposed to the request never getting through.
    pub fn reached_server(&self) -> bool {
        !matches!(self, Self::Timeout | Self::Network(_))
    }
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

#[derive(Clone)]
pub struct Fetcher {
    client: reqwest::Client,
}

impl Fetcher {
    pub fn new(timeout: Duration) -> Self {
        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(timeout)
            // Followed by hand so we can tell permanent moves from temporary ones.
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("HTTP client configuration is static and valid");
        Self { client }
    }

    pub async fn fetch(
        &self,
        url: &Url,
        validators: &Validators,
        now: DateTime<Utc>,
    ) -> Result<Fetched, FetchError> {
        let Some(page) = self.get(url, validators, now).await? else {
            return Ok(Fetched::NotModified);
        };
        let Page {
            url,
            validators,
            body,
            moved_to,
        } = page;
        // Parsing and sanitizing is CPU-bound; keep it off the async workers.
        let parsed = tokio::task::spawn_blocking(move || parse(&body, &url, now)).await;
        match parsed {
            Ok(Ok(feed)) => Ok(Fetched::Updated {
                feed,
                validators,
                moved_to,
            }),
            Ok(Err(error)) => Err(FetchError::Parse(error.to_string())),
            Err(panic) => Err(FetchError::Parse(panic.to_string())),
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
        let mut current = url.clone();
        let mut all_permanent = true;

        for hops in 0..=MAX_REDIRECTS {
            let mut request = self.client.get(current.clone());
            if let Some(etag) = &validators.etag {
                request = request.header(header::IF_NONE_MATCH, etag);
            }
            if let Some(last_modified) = &validators.last_modified {
                request = request.header(header::IF_MODIFIED_SINCE, last_modified);
            }

            let mut response = request.send().await?;
            let status = response.status();
            match status {
                StatusCode::MOVED_PERMANENTLY
                | StatusCode::FOUND
                | StatusCode::SEE_OTHER
                | StatusCode::TEMPORARY_REDIRECT
                | StatusCode::PERMANENT_REDIRECT => {
                    let location = response
                        .headers()
                        .get(header::LOCATION)
                        .and_then(|l| l.to_str().ok())
                        .and_then(|l| current.join(l).ok())
                        .ok_or(FetchError::Http(status.as_u16()))?;
                    all_permanent &= matches!(
                        status,
                        StatusCode::MOVED_PERMANENTLY | StatusCode::PERMANENT_REDIRECT
                    );
                    current = location;
                    continue;
                }
                StatusCode::NOT_MODIFIED => return Ok(None),
                StatusCode::TOO_MANY_REQUESTS | StatusCode::SERVICE_UNAVAILABLE => {
                    return Err(FetchError::RetryLater {
                        retry_after: retry_after(response.headers(), now),
                    });
                }
                _ if !status.is_success() => return Err(FetchError::Http(status.as_u16())),
                _ => {}
            }

            if response
                .content_length()
                .is_some_and(|len| len > MAX_BODY_BYTES as u64)
            {
                return Err(FetchError::TooLarge);
            }
            let validators = Validators {
                etag: header_string(response.headers(), header::ETAG),
                last_modified: header_string(response.headers(), header::LAST_MODIFIED),
            };
            let mut body = Vec::new();
            while let Some(chunk) = response.chunk().await? {
                if body.len() + chunk.len() > MAX_BODY_BYTES {
                    return Err(FetchError::TooLarge);
                }
                body.extend_from_slice(&chunk);
            }

            return Ok(Some(Page {
                moved_to: (hops > 0 && all_permanent).then(|| current.clone()),
                url: current,
                validators,
                body,
            }));
        }
        Err(FetchError::TooManyRedirects)
    }
}

pub struct Page {
    /// Final URL after redirects.
    pub url: Url,
    pub validators: Validators,
    pub body: Vec<u8>,
    pub moved_to: Option<Url>,
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
