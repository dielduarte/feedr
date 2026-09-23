use chrono::{TimeZone, Utc};
use feedr::db::Db;
use feedr::opml::{FeedEntry, Subscriptions, import, parse, render};
use tempfile::TempDir;

fn fixture() -> String {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/subscriptions.opml"
    ))
    .unwrap()
}

fn urls(feeds: &[FeedEntry]) -> Vec<&str> {
    feeds.iter().map(|f| f.url.as_str()).collect()
}

fn outline(subscriptions: &Subscriptions) -> Vec<(String, Vec<&str>)> {
    let mut all: Vec<_> = subscriptions
        .folders
        .iter()
        .map(|f| (f.name.clone(), urls(&f.feeds)))
        .collect();
    all.push((String::new(), urls(&subscriptions.uncategorized)));
    all
}

async fn open() -> (Db, TempDir) {
    let dir = tempfile::tempdir().unwrap();
    (Db::open(&dir.path().join("feedr.db")).await.unwrap(), dir)
}

mod parsing {
    use super::*;

    #[test]
    fn reads_folders_and_uncategorized_feeds() {
        let subscriptions = parse(&fixture()).unwrap();

        assert_eq!(
            outline(&subscriptions),
            [
                (
                    "Tech".to_string(),
                    vec![
                        "https://blog.rust-lang.org/feed.xml",
                        "https://nested.example.com/feed",
                        "https://jvns.ca/atom.xml"
                    ]
                ),
                ("News".to_string(), vec!["https://news.example.com/rss"]),
                (String::new(), vec!["https://loose.example.com/feed"]),
            ]
        );
    }

    #[test]
    fn prefers_the_title_attribute_and_falls_back_to_text() {
        let subscriptions = parse(&fixture()).unwrap();
        let rust = &subscriptions.folders[0].feeds[0];

        assert_eq!(rust.title.as_deref(), Some("The Rust Blog"));
        assert_eq!(
            rust.site_url.as_ref().unwrap().as_str(),
            "https://blog.rust-lang.org/"
        );
        assert_eq!(
            subscriptions.folders[1].feeds[0].title.as_deref(),
            Some("Text Only")
        );
    }

    #[test]
    fn reports_entries_with_unusable_feed_urls() {
        assert_eq!(parse(&fixture()).unwrap().invalid, ["not a url"]);
    }

    #[test]
    fn rejects_documents_that_are_not_opml() {
        assert!(parse("<html><body></body></html>").is_err());
        assert!(parse("").is_err());
    }
}

mod importing {
    use super::*;

    #[tokio::test]
    async fn creates_folders_and_feeds_due_for_fetching() {
        let (db, _dir) = open().await;
        let now = Utc.with_ymd_and_hms(2026, 9, 23, 12, 0, 0).unwrap();

        let report = import(&db, parse(&fixture()).unwrap(), now).await.unwrap();

        assert_eq!(report.added, 5);
        assert_eq!(report.skipped, 0);
        assert_eq!(report.invalid, ["not a url"]);
        let sidebar = db.sidebar().await.unwrap();
        assert_eq!(sidebar.folders[0].folder.name, "Tech");
        assert_eq!(sidebar.folders[0].feeds[0].title, "The Rust Blog");
        assert_eq!(
            sidebar.uncategorized[0].url.as_str(),
            "https://loose.example.com/feed"
        );
        assert_eq!(db.feeds_due(now).await.unwrap().len(), 5);
    }

    #[tokio::test]
    async fn skips_feeds_already_subscribed_and_reuses_folders() {
        let (db, _dir) = open().await;
        let now = Utc::now();
        import(&db, parse(&fixture()).unwrap(), now).await.unwrap();

        let again = import(&db, parse(&fixture()).unwrap(), now).await.unwrap();

        assert_eq!(again.added, 0);
        assert_eq!(again.skipped, 5);
        assert_eq!(db.sidebar().await.unwrap().folders.len(), 2);
    }

    #[tokio::test]
    async fn exports_what_it_imports() {
        let (db, _dir) = open().await;
        let original = parse(&fixture()).unwrap();
        import(&db, parse(&fixture()).unwrap(), Utc::now())
            .await
            .unwrap();

        let exported = parse(&render(&db.sidebar().await.unwrap())).unwrap();

        assert_eq!(outline(&exported), outline(&original));
        assert_eq!(
            exported.folders[0].feeds[0].title.as_deref(),
            Some("The Rust Blog")
        );
        assert_eq!(
            exported.folders[0].feeds[0]
                .site_url
                .as_ref()
                .unwrap()
                .as_str(),
            "https://blog.rust-lang.org/"
        );
    }
}
