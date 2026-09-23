use chrono::{DateTime, TimeZone, Utc};
use feedr::db::{Db, DbError, FetchRecord, ItemQuery, ItemScope, NewFeed};
use feedr::model::{FeedId, Validators};
use feedr::parse::{NewItem, ParsedFeed};
use feedr::sanitize::SanitizedHtml;
use tempfile::TempDir;
use url::Url;

struct TestDb {
    db: Db,
    _dir: TempDir,
}

async fn open() -> TestDb {
    let dir = tempfile::tempdir().unwrap();
    let db = Db::open(&dir.path().join("feedr.db")).await.unwrap();
    TestDb { db, _dir: dir }
}

fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 23, hour, 0, 0).unwrap()
}

fn new_feed(url: &str) -> NewFeed {
    NewFeed {
        url: Url::parse(url).unwrap(),
        title: format!("Title of {url}"),
        site_url: None,
        folder: None,
    }
}

fn item(guid: &str, published_at: DateTime<Utc>) -> NewItem {
    let base = Url::parse("https://example.com/").unwrap();
    NewItem {
        guid: guid.to_string(),
        url: Some(base.join(guid).unwrap()),
        title: Some(format!("Item {guid}")),
        author: None,
        content: Some(SanitizedHtml::clean(&format!("<p>{guid}</p>"), &base)),
        published_at,
    }
}

fn parsed(items: Vec<NewItem>) -> ParsedFeed {
    ParsedFeed {
        title: "Fetched title".to_string(),
        site_url: Some(Url::parse("https://example.com/").unwrap()),
        items,
    }
}

async fn feed_with_items(db: &Db, url: &str, items: Vec<NewItem>) -> FeedId {
    let feed = db.insert_feed(new_feed(url), at(0)).await.unwrap();
    let record = FetchRecord::Updated {
        feed: &parsed(items),
        validators: Validators::default(),
    };
    db.record_fetch(feed.id, record, at(0), at(1))
        .await
        .unwrap();
    feed.id
}

async fn page_guids(db: &Db, query: ItemQuery) -> Vec<String> {
    db.list_items(query)
        .await
        .unwrap()
        .items
        .into_iter()
        .map(|i| i.title.unwrap().trim_start_matches("Item ").to_string())
        .collect()
}

fn query(scope: ItemScope) -> ItemQuery {
    ItemQuery {
        scope,
        unread_only: false,
        cursor: None,
        limit: 50,
    }
}

mod folders {
    use super::*;

    #[tokio::test]
    async fn are_listed_in_creation_order() {
        let t = open().await;
        t.db.create_folder("Tech").await.unwrap();
        t.db.create_folder("News").await.unwrap();

        let names: Vec<_> =
            t.db.sidebar()
                .await
                .unwrap()
                .folders
                .into_iter()
                .map(|f| f.folder.name)
                .collect();

        assert_eq!(names, ["Tech", "News"]);
    }

    #[tokio::test]
    async fn reject_duplicate_names() {
        let t = open().await;
        t.db.create_folder("Tech").await.unwrap();

        assert!(matches!(
            t.db.create_folder("Tech").await,
            Err(DbError::AlreadyExists)
        ));
    }

    #[tokio::test]
    async fn can_be_renamed() {
        let t = open().await;
        let folder = t.db.create_folder("Tech").await.unwrap();

        t.db.rename_folder(folder.id, "Programming").await.unwrap();

        assert_eq!(
            t.db.sidebar().await.unwrap().folders[0].folder.name,
            "Programming"
        );
    }

    #[tokio::test]
    async fn can_be_reordered() {
        let t = open().await;
        t.db.create_folder("A").await.unwrap();
        t.db.create_folder("B").await.unwrap();
        let c = t.db.create_folder("C").await.unwrap();

        t.db.move_folder(c.id, 0).await.unwrap();

        let names: Vec<_> =
            t.db.sidebar()
                .await
                .unwrap()
                .folders
                .into_iter()
                .map(|f| f.folder.name)
                .collect();
        assert_eq!(names, ["C", "A", "B"]);
    }

    #[tokio::test]
    async fn deleting_keeps_feeds_as_uncategorized() {
        let t = open().await;
        let folder = t.db.create_folder("Tech").await.unwrap();
        let feed =
            t.db.insert_feed(
                NewFeed {
                    folder: Some(folder.id),
                    ..new_feed("https://a.com/feed")
                },
                at(0),
            )
            .await
            .unwrap();

        t.db.delete_folder(folder.id).await.unwrap();

        let sidebar = t.db.sidebar().await.unwrap();
        assert!(sidebar.folders.is_empty());
        assert_eq!(sidebar.uncategorized[0].id, feed.id);
    }

    #[tokio::test]
    async fn operations_on_missing_folders_are_not_found() {
        let t = open().await;
        let folder = t.db.create_folder("Tech").await.unwrap();
        t.db.delete_folder(folder.id).await.unwrap();

        assert!(matches!(
            t.db.rename_folder(folder.id, "X").await,
            Err(DbError::NotFound)
        ));
        assert!(matches!(
            t.db.delete_folder(folder.id).await,
            Err(DbError::NotFound)
        ));
        assert!(matches!(
            t.db.insert_feed(
                NewFeed {
                    folder: Some(folder.id),
                    ..new_feed("https://a.com/feed")
                },
                at(0)
            )
            .await,
            Err(DbError::NotFound)
        ));
    }
}

mod feeds {
    use super::*;

    #[tokio::test]
    async fn reject_duplicate_urls() {
        let t = open().await;
        t.db.insert_feed(new_feed("https://a.com/feed"), at(0))
            .await
            .unwrap();

        assert!(matches!(
            t.db.insert_feed(new_feed("https://a.com/feed"), at(0))
                .await,
            Err(DbError::AlreadyExists)
        ));
    }

    #[tokio::test]
    async fn are_due_until_next_fetch_time() {
        let t = open().await;
        let feed =
            t.db.insert_feed(new_feed("https://a.com/feed"), at(0))
                .await
                .unwrap();
        assert_eq!(t.db.feeds_due(at(0)).await.unwrap()[0].id, feed.id);

        t.db.record_fetch(feed.id, FetchRecord::NotModified, at(0), at(1))
            .await
            .unwrap();

        assert!(t.db.feeds_due(at(0)).await.unwrap().is_empty());
        assert_eq!(t.db.feeds_due(at(1)).await.unwrap()[0].id, feed.id);
    }

    #[tokio::test]
    async fn can_move_between_folders_at_a_position() {
        let t = open().await;
        let folder = t.db.create_folder("Tech").await.unwrap();
        let a =
            t.db.insert_feed(
                NewFeed {
                    folder: Some(folder.id),
                    ..new_feed("https://a.com/feed")
                },
                at(0),
            )
            .await
            .unwrap();
        let b =
            t.db.insert_feed(
                NewFeed {
                    folder: Some(folder.id),
                    ..new_feed("https://b.com/feed")
                },
                at(0),
            )
            .await
            .unwrap();
        let c =
            t.db.insert_feed(new_feed("https://c.com/feed"), at(0))
                .await
                .unwrap();

        t.db.move_feed(c.id, Some(folder.id), 1).await.unwrap();

        let sidebar = t.db.sidebar().await.unwrap();
        let ids: Vec<_> = sidebar.folders[0].feeds.iter().map(|f| f.id).collect();
        assert_eq!(ids, [a.id, c.id, b.id]);
        assert!(sidebar.uncategorized.is_empty());

        t.db.move_feed(a.id, None, 0).await.unwrap();

        let sidebar = t.db.sidebar().await.unwrap();
        assert_eq!(sidebar.uncategorized[0].id, a.id);
        let ids: Vec<_> = sidebar.folders[0].feeds.iter().map(|f| f.id).collect();
        assert_eq!(ids, [c.id, b.id]);
    }

    #[tokio::test]
    async fn custom_title_overrides_the_feed_title() {
        let t = open().await;
        let feed =
            t.db.insert_feed(new_feed("https://a.com/feed"), at(0))
                .await
                .unwrap();

        t.db.set_custom_title(feed.id, Some("Mine")).await.unwrap();
        assert_eq!(t.db.sidebar().await.unwrap().uncategorized[0].title, "Mine");

        t.db.set_custom_title(feed.id, None).await.unwrap();
        assert_eq!(
            t.db.sidebar().await.unwrap().uncategorized[0].title,
            "Title of https://a.com/feed"
        );
    }

    #[tokio::test]
    async fn deleting_removes_their_items() {
        let t = open().await;
        let feed = feed_with_items(&t.db, "https://a.com/feed", vec![item("1", at(1))]).await;

        t.db.delete_feed(feed).await.unwrap();

        assert!(
            t.db.list_items(query(ItemScope::All))
                .await
                .unwrap()
                .items
                .is_empty()
        );
    }
}

mod fetches {
    use super::*;

    #[tokio::test]
    async fn store_items_title_and_validators() {
        let t = open().await;
        let feed =
            t.db.insert_feed(new_feed("https://a.com/feed"), at(0))
                .await
                .unwrap();
        let validators = Validators {
            etag: Some("\"v1\"".to_string()),
            last_modified: Some("Wed, 23 Sep 2026 00:00:00 GMT".to_string()),
        };

        let new =
            t.db.record_fetch(
                feed.id,
                FetchRecord::Updated {
                    feed: &parsed(vec![item("1", at(1)), item("2", at(2))]),
                    validators: validators.clone(),
                },
                at(0),
                at(1),
            )
            .await
            .unwrap();

        assert_eq!(new, 2);
        let stored = &t.db.feeds_due(at(1)).await.unwrap()[0];
        assert_eq!(stored.title, "Fetched title");
        assert_eq!(stored.validators, validators);
    }

    #[tokio::test]
    async fn do_not_duplicate_or_reset_known_items() {
        let t = open().await;
        let feed = feed_with_items(&t.db, "https://a.com/feed", vec![item("1", at(1))]).await;
        let existing = t.db.list_items(query(ItemScope::All)).await.unwrap().items[0].id;
        t.db.set_read(existing, true).await.unwrap();

        let new =
            t.db.record_fetch(
                feed,
                FetchRecord::Updated {
                    feed: &parsed(vec![item("1", at(1)), item("2", at(2))]),
                    validators: Validators::default(),
                },
                at(1),
                at(2),
            )
            .await
            .unwrap();

        assert_eq!(new, 1);
        let items = t.db.list_items(query(ItemScope::All)).await.unwrap().items;
        assert_eq!(items.len(), 2);
        assert!(
            items
                .iter()
                .find(|i| i.id == existing)
                .unwrap()
                .read_at
                .is_some()
        );
    }

    #[tokio::test]
    async fn failures_count_up_and_success_resets_them() {
        let t = open().await;
        let feed =
            t.db.insert_feed(new_feed("https://a.com/feed"), at(0))
                .await
                .unwrap();

        for _ in 0..2 {
            t.db.record_fetch(
                feed.id,
                FetchRecord::Failed {
                    error: "timeout".to_string(),
                },
                at(0),
                at(0),
            )
            .await
            .unwrap();
        }

        let stored = &t.db.feeds_due(at(0)).await.unwrap()[0];
        assert_eq!(stored.error_count, 2);
        assert_eq!(stored.last_error.as_deref(), Some("timeout"));
        assert_eq!(
            t.db.sidebar().await.unwrap().uncategorized[0]
                .last_error
                .as_deref(),
            Some("timeout")
        );

        t.db.record_fetch(feed.id, FetchRecord::NotModified, at(0), at(0))
            .await
            .unwrap();

        let stored = &t.db.feeds_due(at(0)).await.unwrap()[0];
        assert_eq!(stored.error_count, 0);
        assert_eq!(stored.last_error, None);
    }

    #[tokio::test]
    async fn for_missing_feeds_are_not_found() {
        let t = open().await;

        assert!(matches!(
            t.db.record_fetch(FeedId(42), FetchRecord::NotModified, at(0), at(1))
                .await,
            Err(DbError::NotFound)
        ));
    }
}

mod items {
    use super::*;

    #[tokio::test]
    async fn unread_counts_roll_up_per_feed_folder_and_total() {
        let t = open().await;
        let folder = t.db.create_folder("Tech").await.unwrap();
        let a = feed_with_items(
            &t.db,
            "https://a.com/feed",
            vec![item("a1", at(1)), item("a2", at(2))],
        )
        .await;
        feed_with_items(&t.db, "https://b.com/feed", vec![item("b1", at(3))]).await;
        t.db.move_feed(a, Some(folder.id), 0).await.unwrap();
        let first =
            t.db.list_items(query(ItemScope::Feed(a)))
                .await
                .unwrap()
                .items[0]
                .id;
        t.db.set_read(first, true).await.unwrap();

        let sidebar = t.db.sidebar().await.unwrap();

        assert_eq!(sidebar.folders[0].feeds[0].unread, 1);
        assert_eq!(sidebar.folders[0].unread(), 1);
        assert_eq!(sidebar.uncategorized[0].unread, 1);
        assert_eq!(sidebar.total_unread(), 2);
    }

    #[tokio::test]
    async fn are_listed_newest_first_within_a_scope() {
        let t = open().await;
        let folder = t.db.create_folder("Tech").await.unwrap();
        let a = feed_with_items(
            &t.db,
            "https://a.com/feed",
            vec![item("a1", at(1)), item("a2", at(3))],
        )
        .await;
        let b = feed_with_items(&t.db, "https://b.com/feed", vec![item("b1", at(2))]).await;
        t.db.move_feed(b, Some(folder.id), 0).await.unwrap();

        assert_eq!(
            page_guids(&t.db, query(ItemScope::All)).await,
            ["a2", "b1", "a1"]
        );
        assert_eq!(
            page_guids(&t.db, query(ItemScope::Feed(a))).await,
            ["a2", "a1"]
        );
        assert_eq!(
            page_guids(&t.db, query(ItemScope::Folder(folder.id))).await,
            ["b1"]
        );
    }

    #[tokio::test]
    async fn can_be_filtered_to_unread_or_starred() {
        let t = open().await;
        feed_with_items(
            &t.db,
            "https://a.com/feed",
            vec![item("1", at(1)), item("2", at(2))],
        )
        .await;
        let items = t.db.list_items(query(ItemScope::All)).await.unwrap().items;
        t.db.set_read(items[0].id, true).await.unwrap();
        t.db.set_starred(items[1].id, true).await.unwrap();

        assert_eq!(
            page_guids(
                &t.db,
                ItemQuery {
                    unread_only: true,
                    ..query(ItemScope::All)
                }
            )
            .await,
            ["1"]
        );
        assert_eq!(page_guids(&t.db, query(ItemScope::Starred)).await, ["1"]);

        t.db.set_starred(items[1].id, false).await.unwrap();
        assert!(
            page_guids(&t.db, query(ItemScope::Starred))
                .await
                .is_empty()
        );
    }

    #[tokio::test]
    async fn pagination_is_stable_when_newer_items_arrive() {
        let t = open().await;
        let feed = feed_with_items(
            &t.db,
            "https://a.com/feed",
            (1..=5).map(|h| item(&h.to_string(), at(h))).collect(),
        )
        .await;

        let first =
            t.db.list_items(ItemQuery {
                limit: 2,
                ..query(ItemScope::All)
            })
            .await
            .unwrap();
        t.db.record_fetch(
            feed,
            FetchRecord::Updated {
                feed: &parsed(vec![item("6", at(6))]),
                validators: Validators::default(),
            },
            at(6),
            at(7),
        )
        .await
        .unwrap();
        let second = ItemQuery {
            limit: 2,
            cursor: first.next,
            ..query(ItemScope::All)
        };
        let second_page = t.db.list_items(second).await.unwrap();

        let titles = |items: &[feedr::db::ItemSummary]| {
            items
                .iter()
                .map(|i| i.title.clone().unwrap())
                .collect::<Vec<_>>()
        };
        assert_eq!(titles(&first.items), ["Item 5", "Item 4"]);
        assert_eq!(titles(&second_page.items), ["Item 3", "Item 2"]);
        let last =
            t.db.list_items(ItemQuery {
                limit: 2,
                cursor: second_page.next,
                ..query(ItemScope::All)
            })
            .await
            .unwrap();
        assert_eq!(titles(&last.items), ["Item 1"]);
        assert!(last.next.is_none());
    }

    #[tokio::test]
    async fn mark_read_spares_items_that_arrived_after_the_user_looked() {
        let t = open().await;
        let feed = feed_with_items(
            &t.db,
            "https://a.com/feed",
            vec![item("1", at(1)), item("2", at(2))],
        )
        .await;
        let seen = t.db.list_items(query(ItemScope::All)).await.unwrap().items[0].id;
        t.db.record_fetch(
            feed,
            FetchRecord::Updated {
                feed: &parsed(vec![item("0", at(0))]),
                validators: Validators::default(),
            },
            at(2),
            at(3),
        )
        .await
        .unwrap();

        let marked = t.db.mark_read(ItemScope::Feed(feed), seen).await.unwrap();

        assert_eq!(marked, 2);
        assert_eq!(
            page_guids(
                &t.db,
                ItemQuery {
                    unread_only: true,
                    ..query(ItemScope::All)
                }
            )
            .await,
            ["0"]
        );
    }

    #[tokio::test]
    async fn full_item_includes_sanitized_content() {
        let t = open().await;
        feed_with_items(&t.db, "https://a.com/feed", vec![item("1", at(1))]).await;
        let id = t.db.list_items(query(ItemScope::All)).await.unwrap().items[0].id;

        let full = t.db.get_item(id).await.unwrap();

        assert_eq!(full.summary.feed_title, "Fetched title");
        assert_eq!(full.content.unwrap().as_str(), "<p>1</p>");
    }

    #[tokio::test]
    async fn missing_items_are_not_found() {
        let t = open().await;

        assert!(matches!(
            t.db.get_item(feedr::model::ItemId(1)).await,
            Err(DbError::NotFound)
        ));
        assert!(matches!(
            t.db.set_read(feedr::model::ItemId(1), true).await,
            Err(DbError::NotFound)
        ));
    }
}
