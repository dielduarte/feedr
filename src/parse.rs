use chrono::{DateTime, Utc};
use feed_rs::model::{Entry, Link, Text};
use sha2::{Digest, Sha256};
use url::Url;

use crate::sanitize::SanitizedHtml;

#[derive(Debug)]
pub struct ParsedFeed {
    pub title: String,
    pub site_url: Option<Url>,
    pub items: Vec<NewItem>,
}

#[derive(Debug, Clone)]
pub struct NewItem {
    pub guid: String,
    pub url: Option<Url>,
    pub title: Option<String>,
    pub author: Option<String>,
    pub content: Option<SanitizedHtml>,
    pub published_at: DateTime<Utc>,
}

#[derive(Debug, thiserror::Error)]
#[error("invalid feed: {0}")]
pub struct ParseError(#[from] feed_rs::parser::ParseFeedError);

pub fn parse(bytes: &[u8], feed_url: &Url, fetched_at: DateTime<Utc>) -> Result<ParsedFeed, ParseError> {
    let feed = feed_rs::parser::Builder::new()
        .base_uri(Some(feed_url.as_str()))
        .id_generator(stable_id)
        .build()
        .parse(bytes)?;

    Ok(ParsedFeed {
        title: feed
            .title
            .map(|t| t.content)
            .unwrap_or_else(|| feed_url.host_str().unwrap_or(feed_url.as_str()).to_string()),
        site_url: primary_link(&feed.links),
        items: feed
            .entries
            .into_iter()
            .map(|entry| to_item(entry, feed_url, fetched_at))
            .collect(),
    })
}

fn to_item(entry: Entry, feed_url: &Url, fetched_at: DateTime<Utc>) -> NewItem {
    let url = primary_link(&entry.links);
    let base = url.as_ref().unwrap_or(feed_url);
    let raw_content = entry
        .content
        .and_then(|c| c.body)
        .or(entry.summary.map(|s| s.content));

    NewItem {
        guid: entry.id,
        content: raw_content.map(|raw| SanitizedHtml::clean(&raw, base)),
        url,
        title: entry.title.map(|t| t.content),
        author: entry.authors.into_iter().next().map(|p| p.name),
        published_at: entry.published.or(entry.updated).unwrap_or(fetched_at),
    }
}

fn primary_link(links: &[Link]) -> Option<Url> {
    links
        .iter()
        .find(|l| matches!(l.rel.as_deref(), None | Some("alternate")))
        .and_then(|l| Url::parse(&l.href).ok())
}

// feed-rs falls back to a random UUID, which would re-insert the same item on every poll.
fn stable_id(links: &[Link], title: &Option<Text>, feed_url: Option<&str>) -> String {
    match links.first() {
        Some(link) => link.href.clone(),
        None => {
            let title = title.as_ref().map_or("", |t| t.content.as_str());
            let digest = Sha256::digest(format!("{}\n{title}", feed_url.unwrap_or_default()));
            format!("sha256:{}", hex::encode(digest))
        }
    }
}
