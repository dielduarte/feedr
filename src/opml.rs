use chrono::{DateTime, Utc};
use opml::{Body, Head, OPML, Outline};
use serde::Serialize;
use url::Url;

use crate::add_feed::parse_input;
use crate::db::{Db, DbError, NewFeed, Sidebar, SidebarFeed};

/// A subscription list, with folders flattened to one level like feedrsauros's own.
#[derive(Debug, Default)]
pub struct Subscriptions {
    pub folders: Vec<FolderEntry>,
    pub uncategorized: Vec<FeedEntry>,
    /// Feed URLs that could not be used, as written in the file.
    pub invalid: Vec<String>,
}

#[derive(Debug)]
pub struct FolderEntry {
    pub name: String,
    pub feeds: Vec<FeedEntry>,
}

#[derive(Debug)]
pub struct FeedEntry {
    pub url: Url,
    pub title: Option<String>,
    pub site_url: Option<Url>,
}

#[derive(Debug, thiserror::Error)]
#[error("not a valid OPML file: {0}")]
pub struct OpmlError(#[from] opml::Error);

#[derive(Debug, Serialize)]
pub struct ImportReport {
    pub added: u32,
    pub skipped: u32,
    pub invalid: Vec<String>,
}

pub fn parse(xml: &str) -> Result<Subscriptions, OpmlError> {
    let document = OPML::from_str(xml)?;
    let mut subscriptions = Subscriptions::default();

    for outline in document.body.outlines {
        if outline.xml_url.is_some() {
            collect_feeds(
                outline,
                &mut subscriptions.uncategorized,
                &mut subscriptions.invalid,
            );
            continue;
        }
        let name = label(&outline).unwrap_or_default();
        let mut feeds = Vec::new();
        collect_feeds(outline, &mut feeds, &mut subscriptions.invalid);
        match subscriptions.folders.iter_mut().find(|f| f.name == name) {
            Some(folder) => folder.feeds.extend(feeds),
            None if !feeds.is_empty() => subscriptions.folders.push(FolderEntry { name, feeds }),
            None => {}
        }
    }
    Ok(subscriptions)
}

/// Gathers every feed under `outline`, however deeply nested, in document order.
fn collect_feeds(outline: Outline, feeds: &mut Vec<FeedEntry>, invalid: &mut Vec<String>) {
    if let Some(xml_url) = &outline.xml_url {
        match parse_input(xml_url) {
            Some(url) => feeds.push(FeedEntry {
                url,
                title: label(&outline),
                site_url: outline.html_url.as_deref().and_then(|u| Url::parse(u).ok()),
            }),
            None => invalid.push(xml_url.clone()),
        }
        return;
    }
    for child in outline.outlines {
        collect_feeds(child, feeds, invalid);
    }
}

fn label(outline: &Outline) -> Option<String> {
    [outline.title.as_deref(), Some(outline.text.as_str())]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|s| !s.is_empty())
        .map(str::to_string)
}

/// Adds the feeds as due immediately, leaving the fetching to the poller so a large import
/// returns at once and is fetched with the usual concurrency limits.
pub async fn import(
    db: &Db,
    subscriptions: Subscriptions,
    now: DateTime<Utc>,
) -> Result<ImportReport, DbError> {
    let mut report = ImportReport {
        added: 0,
        skipped: 0,
        invalid: subscriptions.invalid,
    };
    let uncategorized = subscriptions.uncategorized.into_iter().map(|f| (None, f));
    let mut feeds: Vec<_> = uncategorized.collect();
    for folder in subscriptions.folders {
        let id = db.ensure_folder(&folder.name).await?.id;
        feeds.extend(folder.feeds.into_iter().map(|f| (Some(id), f)));
    }

    for (folder, feed) in feeds {
        let new = NewFeed {
            title: feed
                .title
                .unwrap_or_else(|| feed.url.host_str().unwrap_or(feed.url.as_str()).to_string()),
            url: feed.url,
            site_url: feed.site_url,
            folder,
        };
        match db.insert_feed(new, now).await {
            Ok(_) => report.added += 1,
            Err(DbError::AlreadyExists) => report.skipped += 1,
            Err(other) => return Err(other),
        }
    }
    Ok(report)
}

pub fn render(sidebar: &Sidebar) -> String {
    let folders = sidebar.folders.iter().map(|folder| Outline {
        text: folder.folder.name.clone(),
        title: Some(folder.folder.name.clone()),
        outlines: folder.feeds.iter().map(feed_outline).collect(),
        ..Outline::default()
    });
    let document = OPML {
        head: Some(Head {
            title: Some("feedrsauros subscriptions".to_string()),
            ..Head::default()
        }),
        body: Body {
            outlines: folders
                .chain(sidebar.uncategorized.iter().map(feed_outline))
                .collect(),
        },
        ..OPML::default()
    };
    let xml = document
        .to_string()
        .expect("OPML built from plain strings always serializes");
    format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{xml}\n")
}

fn feed_outline(feed: &SidebarFeed) -> Outline {
    Outline {
        text: feed.title.clone(),
        title: Some(feed.title.clone()),
        r#type: Some("rss".to_string()),
        xml_url: Some(feed.url.to_string()),
        html_url: feed.site_url.as_ref().map(Url::to_string),
        ..Outline::default()
    }
}
