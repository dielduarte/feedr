use std::time::Duration;

use axum::Router;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::get;
use chrono::{DateTime, TimeZone, Utc};
use feedrsauros::fetch::{FetchError, Fetched, Fetcher};
use feedrsauros::model::Validators;
use url::Url;

const ETAG: &str = "\"v1\"";
const LAST_MODIFIED: &str = "Tue, 22 Sep 2026 10:00:00 GMT";

fn rss2() -> Vec<u8> {
    std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/rss2.xml"
    ))
    .unwrap()
}

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 23, 12, 0, 0).unwrap()
}

fn header_is(headers: &HeaderMap, name: header::HeaderName, value: &str) -> bool {
    headers.get(name).is_some_and(|v| v == value)
}

async fn with_etag(headers: HeaderMap) -> Response {
    if header_is(&headers, header::IF_NONE_MATCH, ETAG) {
        return StatusCode::NOT_MODIFIED.into_response();
    }
    (
        [(header::ETAG, ETAG), (header::LAST_MODIFIED, LAST_MODIFIED)],
        rss2(),
    )
        .into_response()
}

async fn with_last_modified_only(headers: HeaderMap) -> Response {
    if header_is(&headers, header::IF_MODIFIED_SINCE, LAST_MODIFIED) {
        return StatusCode::NOT_MODIFIED.into_response();
    }
    ([(header::LAST_MODIFIED, LAST_MODIFIED)], rss2()).into_response()
}

async fn requires_user_agent(headers: HeaderMap) -> Response {
    let identified = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|ua| ua.starts_with("feedrsauros/"));
    if identified {
        rss2().into_response()
    } else {
        StatusCode::FORBIDDEN.into_response()
    }
}

async fn slow() -> Vec<u8> {
    tokio::time::sleep(Duration::from_secs(5)).await;
    rss2()
}

async fn serve() -> Url {
    let app = Router::new()
        .route("/feed.xml", get(with_etag))
        .route("/last-modified.xml", get(with_last_modified_only))
        .route("/moved", get(|| async { Redirect::permanent("/feed.xml") }))
        .route("/identified.xml", get(requires_user_agent))
        .route(
            "/rate-limited-seconds",
            get(|| async {
                (
                    StatusCode::TOO_MANY_REQUESTS,
                    [(header::RETRY_AFTER, "120")],
                )
            }),
        )
        .route(
            "/rate-limited-date",
            get(|| async {
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    [(header::RETRY_AFTER, "Wed, 23 Sep 2026 12:01:30 GMT")],
                )
            }),
        )
        .route(
            "/rate-limited-bare",
            get(|| async { StatusCode::TOO_MANY_REQUESTS }),
        )
        .route("/missing", get(|| async { StatusCode::NOT_FOUND }))
        .route(
            "/page.html",
            get(|| async { "<html><body>not a feed</body></html>" }),
        )
        .route("/slow", get(slow))
        .route(
            "/temporary",
            get(|| async { Redirect::temporary("/feed.xml") }),
        )
        .route(
            "/moved-twice",
            get(|| async { Redirect::permanent("/moved") }),
        )
        .route(
            "/moved-then-temporary",
            get(|| async { Redirect::permanent("/temporary") }),
        )
        .route("/loop", get(|| async { Redirect::temporary("/loop") }))
        .route("/huge", get(|| async { vec![b' '; 11 * 1024 * 1024] }));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    Url::parse(&format!("http://{addr}/")).unwrap()
}

async fn fetch(url: Url, validators: Validators) -> Result<Fetched, FetchError> {
    Fetcher::new(Duration::from_secs(5))
        .fetch(&url, &validators, now())
        .await
}

#[tokio::test]
async fn returns_the_parsed_feed_and_its_validators() {
    let base = serve().await;

    let Ok(Fetched::Updated {
        feed, validators, ..
    }) = fetch(base.join("feed.xml").unwrap(), Validators::default()).await
    else {
        panic!("expected an updated feed");
    };

    assert_eq!(feed.title, "Example Blog");
    assert_eq!(feed.items.len(), 4);
    assert_eq!(validators.etag.as_deref(), Some(ETAG));
    assert_eq!(validators.last_modified.as_deref(), Some(LAST_MODIFIED));
}

#[tokio::test]
async fn is_not_modified_when_the_etag_matches() {
    let base = serve().await;
    let validators = Validators {
        etag: Some(ETAG.to_string()),
        last_modified: None,
    };

    let outcome = fetch(base.join("feed.xml").unwrap(), validators).await;

    assert!(matches!(outcome, Ok(Fetched::NotModified)));
}

#[tokio::test]
async fn is_not_modified_when_last_modified_matches() {
    let base = serve().await;
    let validators = Validators {
        etag: None,
        last_modified: Some(LAST_MODIFIED.to_string()),
    };

    let outcome = fetch(base.join("last-modified.xml").unwrap(), validators).await;

    assert!(matches!(outcome, Ok(Fetched::NotModified)));
}

async fn moved_to(path: &str) -> Option<Url> {
    let base = serve().await;
    match fetch(base.join(path).unwrap(), Validators::default()).await {
        Ok(Fetched::Updated { moved_to, .. }) => moved_to.inspect(|url| {
            assert_eq!(url.path(), "/feed.xml");
        }),
        _ => panic!("expected {path} to resolve to a feed"),
    }
}

#[tokio::test]
async fn reports_where_a_permanently_moved_feed_lives() {
    assert!(moved_to("moved").await.is_some());
    assert!(moved_to("moved-twice").await.is_some());
}

#[tokio::test]
async fn follows_temporary_redirects_without_reporting_a_move() {
    assert!(moved_to("temporary").await.is_none());
    assert!(moved_to("moved-then-temporary").await.is_none());
}

#[tokio::test]
async fn is_not_moved_without_redirects() {
    assert!(moved_to("feed.xml").await.is_none());
}

#[tokio::test]
async fn gives_up_on_redirect_loops() {
    let base = serve().await;

    let outcome = fetch(base.join("loop").unwrap(), Validators::default()).await;

    assert!(matches!(outcome, Err(FetchError::TooManyRedirects)));
}

#[tokio::test]
async fn refuses_oversized_responses() {
    let base = serve().await;

    let outcome = fetch(base.join("huge").unwrap(), Validators::default()).await;

    assert!(matches!(outcome, Err(FetchError::TooLarge)));
}

#[tokio::test]
async fn identifies_itself_with_a_user_agent() {
    let base = serve().await;

    let outcome = fetch(base.join("identified.xml").unwrap(), Validators::default()).await;

    assert!(matches!(outcome, Ok(Fetched::Updated { .. })));
}

#[tokio::test]
async fn reports_rate_limits_with_retry_after_in_seconds() {
    let base = serve().await;

    let outcome = fetch(
        base.join("rate-limited-seconds").unwrap(),
        Validators::default(),
    )
    .await;

    assert!(matches!(
        outcome,
        Err(FetchError::RetryLater { retry_after: Some(d) }) if d == Duration::from_secs(120)
    ));
}

#[tokio::test]
async fn reports_unavailability_with_retry_after_as_a_date() {
    let base = serve().await;

    let outcome = fetch(
        base.join("rate-limited-date").unwrap(),
        Validators::default(),
    )
    .await;

    assert!(matches!(
        outcome,
        Err(FetchError::RetryLater { retry_after: Some(d) }) if d == Duration::from_secs(90)
    ));
}

#[tokio::test]
async fn reports_rate_limits_without_retry_after() {
    let base = serve().await;

    let outcome = fetch(
        base.join("rate-limited-bare").unwrap(),
        Validators::default(),
    )
    .await;

    assert!(matches!(
        outcome,
        Err(FetchError::RetryLater { retry_after: None })
    ));
}

#[tokio::test]
async fn reports_http_errors() {
    let base = serve().await;

    let outcome = fetch(base.join("missing").unwrap(), Validators::default()).await;

    assert!(matches!(
        outcome,
        Err(FetchError::Http(status)) if status == 404
    ));
}

#[tokio::test]
async fn reports_content_that_is_not_a_feed() {
    let base = serve().await;

    let outcome = fetch(base.join("page.html").unwrap(), Validators::default()).await;

    assert!(matches!(outcome, Err(FetchError::Parse(_))));
}

#[tokio::test]
async fn gives_up_on_slow_servers() {
    let base = serve().await;

    let outcome = Fetcher::new(Duration::from_millis(200))
        .fetch(&base.join("slow").unwrap(), &Validators::default(), now())
        .await;

    assert!(matches!(outcome, Err(FetchError::Timeout)));
}

#[tokio::test]
async fn reports_unreachable_hosts() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);

    let outcome = fetch(
        Url::parse(&format!("http://{addr}/feed.xml")).unwrap(),
        Validators::default(),
    )
    .await;

    assert!(matches!(outcome, Err(FetchError::Network(_))));
}
