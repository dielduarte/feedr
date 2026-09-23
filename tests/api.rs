use std::time::Duration;

use axum::Router;
use axum::response::Html;
use axum::routing::get;
use feedr::api::{self, AppState};
use feedr::db::Db;
use feedr::fetch::Fetcher;
use feedr::poller;
use reqwest::StatusCode;
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio_util::sync::CancellationToken;
use url::Url;

fn rss2() -> Vec<u8> {
    std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/rss2.xml"
    ))
    .unwrap()
}

async fn listen(app: Router) -> Url {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = Url::parse(&format!("http://{}/", listener.local_addr().unwrap())).unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    base
}

struct Api {
    base: Url,
    sites: Url,
    client: reqwest::Client,
    cancel: CancellationToken,
    _dir: TempDir,
}

impl Drop for Api {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

async fn start() -> Api {
    let sites = listen(
        Router::new()
            .route("/a.xml", get(|| async { rss2() }))
            .route("/b.xml", get(|| async { rss2() }))
            .route("/no-feed", get(|| async { Html("<html></html>") })),
    )
    .await;
    let dir = tempfile::tempdir().unwrap();
    let db = Db::open(&dir.path().join("feedr.db")).await.unwrap();
    let fetcher = Fetcher::new(Duration::from_secs(5));
    let cancel = CancellationToken::new();
    let (poller, _) = poller::spawn(db.clone(), fetcher.clone(), cancel.clone());
    let base = listen(api::router(AppState {
        db,
        fetcher,
        poller,
    }))
    .await;
    Api {
        base,
        sites,
        client: reqwest::Client::new(),
        cancel,
        _dir: dir,
    }
}

impl Api {
    async fn send(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut request = self.client.request(method, self.base.join(path).unwrap());
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request.send().await.unwrap();
        let status = response.status();
        let text = response.text().await.unwrap();
        let json = if text.is_empty() {
            Value::Null
        } else {
            serde_json::from_str(&text).unwrap()
        };
        (status, json)
    }

    async fn get(&self, path: &str) -> (StatusCode, Value) {
        self.send(reqwest::Method::GET, path, None).await
    }

    async fn post(&self, path: &str, body: Value) -> (StatusCode, Value) {
        self.send(reqwest::Method::POST, path, Some(body)).await
    }

    async fn put(&self, path: &str, body: Value) -> (StatusCode, Value) {
        self.send(reqwest::Method::PUT, path, Some(body)).await
    }

    async fn patch(&self, path: &str, body: Value) -> (StatusCode, Value) {
        self.send(reqwest::Method::PATCH, path, Some(body)).await
    }

    async fn delete(&self, path: &str) -> (StatusCode, Value) {
        self.send(reqwest::Method::DELETE, path, None).await
    }

    async fn subscribe(&self, site_path: &str, folder: Option<i64>) -> i64 {
        let url = self.sites.join(site_path).unwrap();
        let (status, body) = self
            .post("api/feeds", json!({ "url": url, "folder_id": folder }))
            .await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        body["id"].as_i64().unwrap()
    }

    async fn folder(&self, name: &str) -> i64 {
        let (status, body) = self.post("api/folders", json!({ "name": name })).await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        body["id"].as_i64().unwrap()
    }

    async fn sidebar(&self) -> Value {
        self.get("api/sidebar").await.1
    }

    async fn item_titles(&self, query: &str) -> Vec<String> {
        let (status, body) = self.get(&format!("api/items{query}")).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        body["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| i["title"].as_str().unwrap().to_string())
            .collect()
    }

    async fn first_item_id(&self, query: &str) -> i64 {
        self.get(&format!("api/items{query}")).await.1["items"][0]["id"]
            .as_i64()
            .unwrap()
    }
}

mod sidebar {
    use super::*;

    #[tokio::test]
    async fn shows_folders_feeds_and_unread_counts() {
        let api = start().await;
        let tech = api.folder("Tech").await;
        let a = api.subscribe("a.xml", Some(tech)).await;
        let b = api.subscribe("b.xml", None).await;

        let sidebar = api.sidebar().await;

        assert_eq!(sidebar["total_unread"], 8);
        assert_eq!(sidebar["folders"][0]["id"], tech);
        assert_eq!(sidebar["folders"][0]["name"], "Tech");
        assert_eq!(sidebar["folders"][0]["unread"], 4);
        assert_eq!(sidebar["folders"][0]["feeds"][0]["id"], a);
        assert_eq!(sidebar["folders"][0]["feeds"][0]["title"], "Example Blog");
        assert_eq!(
            sidebar["folders"][0]["feeds"][0]["site_url"],
            "https://example.com/"
        );
        assert_eq!(sidebar["uncategorized"][0]["id"], b);
        assert_eq!(sidebar["uncategorized"][0]["unread"], 4);
        assert_eq!(sidebar["uncategorized"][0]["last_error"], Value::Null);
    }
}

mod folders {
    use super::*;

    #[tokio::test]
    async fn reject_duplicate_names() {
        let api = start().await;
        api.folder("Tech").await;

        let (status, body) = api.post("api/folders", json!({ "name": "Tech" })).await;

        assert_eq!(status, StatusCode::CONFLICT);
        assert!(body["error"].is_string());
    }

    #[tokio::test]
    async fn can_be_renamed() {
        let api = start().await;
        let tech = api.folder("Tech").await;

        let (status, _) = api
            .patch(&format!("api/folders/{tech}"), json!({ "name": "Code" }))
            .await;

        assert_eq!(status, StatusCode::NO_CONTENT);
        assert_eq!(api.sidebar().await["folders"][0]["name"], "Code");
        let (status, _) = api.patch("api/folders/999", json!({ "name": "X" })).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn can_be_reordered() {
        let api = start().await;
        api.folder("A").await;
        let b = api.folder("B").await;

        let (status, _) = api
            .put(&format!("api/folders/{b}/position"), json!({ "index": 0 }))
            .await;

        assert_eq!(status, StatusCode::NO_CONTENT);
        assert_eq!(api.sidebar().await["folders"][0]["name"], "B");
    }

    #[tokio::test]
    async fn deleting_keeps_their_feeds() {
        let api = start().await;
        let tech = api.folder("Tech").await;
        let feed = api.subscribe("a.xml", Some(tech)).await;

        let (status, _) = api.delete(&format!("api/folders/{tech}")).await;

        assert_eq!(status, StatusCode::NO_CONTENT);
        let sidebar = api.sidebar().await;
        assert_eq!(sidebar["folders"], json!([]));
        assert_eq!(sidebar["uncategorized"][0]["id"], feed);
    }
}

mod feeds {
    use super::*;

    #[tokio::test]
    async fn subscribing_reports_the_feed_and_its_items() {
        let api = start().await;
        let url = api.sites.join("a.xml").unwrap();

        let (status, body) = api.post("api/feeds", json!({ "url": url })).await;

        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(body["title"], "Example Blog");
        assert_eq!(body["new_items"], 4);
    }

    #[tokio::test]
    async fn subscribing_explains_failures() {
        let api = start().await;
        api.subscribe("a.xml", None).await;
        let post = |url: String| api.post("api/feeds", json!({ "url": url }));

        assert_eq!(
            post(api.sites.join("a.xml").unwrap().into()).await.0,
            StatusCode::CONFLICT
        );
        assert_eq!(
            post(api.sites.join("no-feed").unwrap().into()).await.0,
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            post(api.sites.join("gone").unwrap().into()).await.0,
            StatusCode::BAD_GATEWAY
        );
        assert_eq!(
            post("not a url".to_string()).await.0,
            StatusCode::BAD_REQUEST
        );
        let (status, body) = api
            .post(
                "api/feeds",
                json!({ "url": api.sites.join("b.xml").unwrap(), "folder_id": 999 }),
            )
            .await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    }

    #[tokio::test]
    async fn can_move_between_folders() {
        let api = start().await;
        let tech = api.folder("Tech").await;
        let a = api.subscribe("a.xml", Some(tech)).await;
        let b = api.subscribe("b.xml", None).await;

        let (status, _) = api
            .put(
                &format!("api/feeds/{b}/position"),
                json!({ "folder_id": tech, "index": 0 }),
            )
            .await;

        assert_eq!(status, StatusCode::NO_CONTENT);
        let feeds = &api.sidebar().await["folders"][0]["feeds"];
        assert_eq!(feeds[0]["id"], b);
        assert_eq!(feeds[1]["id"], a);

        api.put(
            &format!("api/feeds/{a}/position"),
            json!({ "folder_id": null, "index": 0 }),
        )
        .await;

        assert_eq!(api.sidebar().await["uncategorized"][0]["id"], a);
    }

    #[tokio::test]
    async fn can_be_renamed_and_reset() {
        let api = start().await;
        let feed = api.subscribe("a.xml", None).await;

        api.put(
            &format!("api/feeds/{feed}/title"),
            json!({ "title": "Mine" }),
        )
        .await;
        assert_eq!(api.sidebar().await["uncategorized"][0]["title"], "Mine");

        let (status, _) = api
            .put(&format!("api/feeds/{feed}/title"), json!({ "title": null }))
            .await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        assert_eq!(
            api.sidebar().await["uncategorized"][0]["title"],
            "Example Blog"
        );
    }

    #[tokio::test]
    async fn unsubscribing_removes_their_items() {
        let api = start().await;
        let feed = api.subscribe("a.xml", None).await;

        let (status, _) = api.delete(&format!("api/feeds/{feed}")).await;

        assert_eq!(status, StatusCode::NO_CONTENT);
        assert!(api.item_titles("").await.is_empty());
        assert_eq!(
            api.delete(&format!("api/feeds/{feed}")).await.0,
            StatusCode::NOT_FOUND
        );
    }

    #[tokio::test]
    async fn can_be_refreshed_on_demand() {
        let api = start().await;
        let feed = api.subscribe("a.xml", None).await;

        let (status, body) = api.post("api/refresh", json!({ "feed": feed })).await;

        assert_eq!(status, StatusCode::ACCEPTED);
        assert_eq!(body["scheduled"], 1);
        let (status, _) = api
            .post("api/refresh", json!({ "feed": feed, "folder": 1 }))
            .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
}

mod items {
    use super::*;

    #[tokio::test]
    async fn are_listed_newest_first_with_their_feed() {
        let api = start().await;
        api.subscribe("a.xml", None).await;

        let (_, body) = api.get("api/items").await;
        let first = &body["items"][0];

        assert_eq!(body["items"].as_array().unwrap().len(), 4);
        assert_eq!(first["feed_title"], "Example Blog");
        assert!(first["published_at"].is_string());
        assert_eq!(first["read_at"], Value::Null);
        assert_eq!(first.get("content_html"), None);
        assert_eq!(body["next_cursor"], Value::Null);
    }

    #[tokio::test]
    async fn are_paginated_with_an_opaque_cursor() {
        let api = start().await;
        api.subscribe("a.xml", None).await;
        let all = api.item_titles("").await;

        let (_, page) = api.get("api/items?limit=3").await;
        let cursor = page["next_cursor"].as_str().unwrap();
        let rest = api.item_titles(&format!("?limit=3&cursor={cursor}")).await;

        assert_eq!(page["items"].as_array().unwrap().len(), 3);
        assert_eq!(rest, all[3..]);
        assert_eq!(
            api.get("api/items?cursor=garbage").await.0,
            StatusCode::BAD_REQUEST
        );
    }

    #[tokio::test]
    async fn can_be_filtered_by_scope_and_state() {
        let api = start().await;
        let tech = api.folder("Tech").await;
        let a = api.subscribe("a.xml", Some(tech)).await;
        api.subscribe("b.xml", None).await;
        let starred = api.first_item_id(&format!("?feed={a}")).await;
        api.patch(
            &format!("api/items/{starred}"),
            json!({ "starred": true, "read": true }),
        )
        .await;

        assert_eq!(api.item_titles(&format!("?feed={a}")).await.len(), 4);
        assert_eq!(api.item_titles(&format!("?folder={tech}")).await.len(), 4);
        assert_eq!(api.item_titles("?starred=true").await.len(), 1);
        assert_eq!(api.item_titles("?unread=true").await.len(), 7);
        assert_eq!(
            api.get(&format!("api/items?feed={a}&starred=true")).await.0,
            StatusCode::BAD_REQUEST
        );
    }

    #[tokio::test]
    async fn open_with_their_sanitized_content() {
        let api = start().await;
        api.subscribe("a.xml", None).await;
        let (_, list) = api.get("api/items").await;
        let post = list["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["title"] == "First post")
            .unwrap()["id"]
            .clone();

        let (status, item) = api.get(&format!("api/items/{post}")).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(item["title"], "First post");
        assert!(item["content_html"].as_str().unwrap().contains("Hello"));
        assert!(!item["content_html"].as_str().unwrap().contains("script"));
        assert_eq!(api.get("api/items/999").await.0, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn can_be_marked_read_and_unread() {
        let api = start().await;
        api.subscribe("a.xml", None).await;
        let id = api.first_item_id("").await;

        let (status, _) = api
            .patch(&format!("api/items/{id}"), json!({ "read": true }))
            .await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        assert!(api.get(&format!("api/items/{id}")).await.1["read_at"].is_string());

        api.patch(&format!("api/items/{id}"), json!({ "read": false }))
            .await;
        assert_eq!(
            api.get(&format!("api/items/{id}")).await.1["read_at"],
            Value::Null
        );
    }

    #[tokio::test]
    async fn can_be_marked_read_in_bulk_up_to_what_was_seen() {
        let api = start().await;
        let a = api.subscribe("a.xml", None).await;
        api.subscribe("b.xml", None).await;
        let newest_seen = api.first_item_id(&format!("?feed={a}")).await;

        let (status, body) = api
            .post(
                "api/items/mark-read",
                json!({ "feed": a, "up_to": newest_seen }),
            )
            .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["marked"], 4);
        assert_eq!(api.sidebar().await["total_unread"], 4);
    }
}
