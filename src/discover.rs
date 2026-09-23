use scraper::{Html, Selector};
use url::Url;

const FEED_TYPES: [&str; 4] = [
    "application/rss+xml",
    "application/atom+xml",
    "application/feed+json",
    "application/json",
];

/// Paths tried when a page doesn't advertise its feed, relative to the site root.
pub const COMMON_FEED_PATHS: [&str; 7] = [
    "/feed",
    "/rss",
    "/feed.xml",
    "/rss.xml",
    "/atom.xml",
    "/index.xml",
    "/feed.json",
];

/// Feeds advertised by `<link rel="alternate">` in an HTML page, in document order.
pub fn feed_links(html: &str, page_url: &Url) -> Vec<Url> {
    let selector = Selector::parse("link[rel][type][href]").expect("static selector is valid");
    Html::parse_document(html)
        .select(&selector)
        .filter(|link| {
            let attr = |name| link.value().attr(name).unwrap_or_default();
            let is_alternate = attr("rel")
                .split_ascii_whitespace()
                .any(|rel| rel.eq_ignore_ascii_case("alternate"));
            let is_feed = FEED_TYPES
                .iter()
                .any(|t| t.eq_ignore_ascii_case(attr("type").trim()));
            is_alternate && is_feed
        })
        .filter_map(|link| page_url.join(link.value().attr("href")?).ok())
        .collect()
}
