use chrono::{DateTime, TimeZone, Utc};
use feedr::parse::{ParsedFeed, parse};
use url::Url;

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

fn fetched_at() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 23, 12, 0, 0).unwrap()
}

fn parse_fixture(name: &str, feed_url: &str) -> ParsedFeed {
    parse(&fixture(name), &Url::parse(feed_url).unwrap(), fetched_at()).unwrap()
}

fn rss2() -> ParsedFeed {
    parse_fixture("rss2.xml", "https://example.com/feed.xml")
}

#[test]
fn reads_feed_metadata() {
    let feed = rss2();

    assert_eq!(feed.title, "Example Blog");
    assert_eq!(feed.site_url.unwrap().as_str(), "https://example.com/");
    assert_eq!(feed.items.len(), 4);
}

#[test]
fn uses_the_feed_guid_when_present() {
    let item = &rss2().items[0];

    assert_eq!(item.guid, "post-1");
    assert_eq!(item.url.as_ref().unwrap().as_str(), "https://example.com/posts/1");
    assert_eq!(item.title.as_deref(), Some("First post"));
    assert_eq!(item.published_at, Utc.with_ymd_and_hms(2026, 9, 22, 10, 0, 0).unwrap());
}

#[test]
fn prefers_full_content_over_summary() {
    let content = rss2().items[0].content.clone().unwrap();

    assert!(content.as_str().contains("Hello"));
    assert!(!content.as_str().contains("Short summary"));
}

#[test]
fn strips_scripts_and_event_handlers() {
    let content = rss2().items[0].content.clone().unwrap();

    assert!(!content.as_str().contains("script"));
    assert!(!content.as_str().contains("alert"));
    assert!(!content.as_str().contains("onclick"));
}

#[test]
fn resolves_relative_urls_against_the_item_link() {
    let content = rss2().items[0].content.clone().unwrap();

    assert!(content.as_str().contains(r#"href="https://example.com/about""#));
    assert!(content.as_str().contains(r#"src="https://example.com/posts/img.png""#));
}

#[test]
fn falls_back_to_the_link_when_guid_is_missing() {
    assert_eq!(rss2().items[1].guid, "https://example.com/posts/2");
}

#[test]
fn derives_a_stable_distinct_guid_when_guid_and_link_are_missing() {
    let first = rss2();
    let second = rss2();

    assert!(!first.items[2].guid.is_empty());
    assert_eq!(first.items[2].guid, second.items[2].guid);
    assert_ne!(first.items[2].guid, first.items[3].guid);
}

#[test]
fn uses_fetch_time_when_item_has_no_date() {
    assert_eq!(rss2().items[1].published_at, fetched_at());
}

#[test]
fn parses_atom_using_updated_when_published_is_missing() {
    let feed = parse_fixture("atom.xml", "https://atom.example.org/feed.xml");
    let item = &feed.items[0];

    assert_eq!(feed.title, "Atom Example");
    assert_eq!(feed.site_url.unwrap().as_str(), "https://atom.example.org/");
    assert_eq!(item.guid, "urn:uuid:1225c695-cfb8-4ebb-aaaa-80da344efa6a");
    assert_eq!(item.published_at, Utc.with_ymd_and_hms(2026, 9, 20, 18, 30, 2).unwrap());
    assert_eq!(item.content.as_ref().unwrap().as_str(), "<p>Atom body</p>");
}

#[test]
fn parses_json_feed() {
    let feed = parse_fixture("feed.json", "https://json.example.net/feed.json");
    let item = &feed.items[0];

    assert_eq!(feed.title, "JSON Example");
    assert_eq!(feed.site_url.unwrap().as_str(), "https://json.example.net/");
    assert_eq!(item.guid, "json-1");
    assert_eq!(item.content.as_ref().unwrap().as_str(), "<p>JSON body</p>");
}

#[test]
fn rejects_content_that_is_not_a_feed() {
    let result = parse(
        &fixture("not-a-feed.html"),
        &Url::parse("https://example.com/").unwrap(),
        fetched_at(),
    );

    assert!(result.is_err());
}
