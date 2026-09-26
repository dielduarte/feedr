use std::time::Duration;

use axum::Router;
use axum::response::Html;
use axum::routing::get;
use feedrsauros::api::{self, AppState};
use feedrsauros::db::Db;
use feedrsauros::fetch::Fetcher;
use feedrsauros::poller;
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
    let db = Db::open(&dir.path().join("feedrsauros.db")).await.unwrap();
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

    async fn subscribe(&self, site_path: &str, folder: Option<&str>) -> String {
        let url = self.sites.join(site_path).unwrap();
        let (status, body) = self
            .post("api/feeds", json!({ "url": url, "folder": folder }))
            .await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        body["slug"].as_str().unwrap().to_string()
    }

    async fn folder(&self, name: &str) -> String {
        let (status, body) = self.post("api/folders", json!({ "name": name })).await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        body["slug"].as_str().unwrap().to_string()
    }

    async fn sidebar(&self) -> Value {
        self.get("api/sidebar").await.1
    }

    async fn items(&self, query: &str) -> Vec<Value> {
        let (status, body) = self.get(&format!("api/items{query}")).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        body["items"].as_array().unwrap().clone()
    }

    async fn item_titles(&self, query: &str) -> Vec<String> {
        self.items(query)
            .await
            .iter()
            .map(|i| i["title"].as_str().unwrap().to_string())
            .collect()
    }

    /// The API path of an article, built from the slugs the list returns.
    fn item_path(item: &Value) -> String {
        format!(
            "api/feeds/{}/items/{}",
            item["feed_slug"].as_str().unwrap(),
            item["slug"].as_str().unwrap()
        )
    }
}

mod sidebar {
    use super::*;

    #[tokio::test]
    async fn counts_starred_articles() {
        let api = start().await;
        let feed = api.subscribe("a.xml", None).await;
        let items = api.items(&format!("?feed={feed}")).await;
        for item in &items[..2] {
            api.patch(&Api::item_path(item), json!({ "starred": true }))
                .await;
        }
        api.patch(&Api::item_path(&items[0]), json!({ "starred": false }))
            .await;

        assert_eq!(api.sidebar().await["total_starred"], 1);
    }

    #[tokio::test]
    async fn shows_folders_feeds_and_unread_counts_by_slug() {
        let api = start().await;
        let tech = api.folder("Tech").await;
        let a = api.subscribe("a.xml", Some(&tech)).await;
        let b = api.subscribe("b.xml", None).await;

        let sidebar = api.sidebar().await;

        assert_eq!(
            (tech.as_str(), a.as_str(), b.as_str()),
            ("tech", "example-blog", "example-blog-2")
        );
        assert_eq!(sidebar["total_unread"], 8);
        assert_eq!(sidebar["total_starred"], 0);
        assert_eq!(sidebar["folders"][0]["slug"], "tech");
        assert_eq!(sidebar["folders"][0]["name"], "Tech");
        assert_eq!(sidebar["folders"][0]["unread"], 4);
        assert_eq!(sidebar["folders"][0]["feeds"][0]["slug"], "example-blog");
        assert_eq!(sidebar["folders"][0]["feeds"][0]["title"], "Example Blog");
        assert_eq!(
            sidebar["folders"][0]["feeds"][0]["site_url"],
            "https://example.com/"
        );
        assert_eq!(sidebar["uncategorized"][0]["slug"], "example-blog-2");
        assert_eq!(sidebar["uncategorized"][0]["last_error"], Value::Null);
    }

    #[tokio::test]
    async fn never_exposes_internal_ids() {
        let api = start().await;
        let tech = api.folder("Tech").await;
        api.subscribe("a.xml", Some(&tech)).await;

        let sidebar = api.sidebar().await;
        let item = &api.items("").await[0];

        assert_eq!(sidebar["folders"][0].get("id"), None);
        assert_eq!(sidebar["folders"][0]["feeds"][0].get("id"), None);
        assert_eq!(item.get("id"), None);
        assert_eq!(item.get("feed_id"), None);
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
    async fn move_to_a_new_url_when_renamed() {
        let api = start().await;
        let tech = api.folder("Tech").await;

        let (status, body) = api
            .patch(&format!("api/folders/{tech}"), json!({ "name": "Code" }))
            .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["slug"], "code");
        assert_eq!(api.sidebar().await["folders"][0]["name"], "Code");
        assert_eq!(api.get("api/items?folder=code").await.0, StatusCode::OK);
        assert_eq!(
            api.get("api/items?folder=tech").await.0,
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            api.patch("api/folders/nope", json!({ "name": "X" }))
                .await
                .0,
            StatusCode::NOT_FOUND
        );
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
        let feed = api.subscribe("a.xml", Some(&tech)).await;

        let (status, _) = api.delete(&format!("api/folders/{tech}")).await;

        assert_eq!(status, StatusCode::NO_CONTENT);
        let sidebar = api.sidebar().await;
        assert_eq!(sidebar["folders"], json!([]));
        assert_eq!(sidebar["uncategorized"][0]["slug"], feed);
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
        assert_eq!(body["slug"], "example-blog");
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
                json!({ "url": api.sites.join("b.xml").unwrap(), "folder": "nope" }),
            )
            .await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    }

    #[tokio::test]
    async fn can_move_between_folders() {
        let api = start().await;
        let tech = api.folder("Tech").await;
        let a = api.subscribe("a.xml", Some(&tech)).await;
        let b = api.subscribe("b.xml", None).await;

        let (status, _) = api
            .put(
                &format!("api/feeds/{b}/position"),
                json!({ "folder": tech, "index": 0 }),
            )
            .await;

        assert_eq!(status, StatusCode::NO_CONTENT);
        let feeds = &api.sidebar().await["folders"][0]["feeds"];
        assert_eq!(feeds[0]["slug"], b);
        assert_eq!(feeds[1]["slug"], a);

        api.put(
            &format!("api/feeds/{a}/position"),
            json!({ "folder": null, "index": 0 }),
        )
        .await;

        assert_eq!(api.sidebar().await["uncategorized"][0]["slug"], a);
    }

    #[tokio::test]
    async fn move_to_a_new_url_when_renamed_and_back_when_reset() {
        let api = start().await;
        let feed = api.subscribe("a.xml", None).await;

        let (status, body) = api
            .put(
                &format!("api/feeds/{feed}/title"),
                json!({ "title": "Mine" }),
            )
            .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["slug"], "mine");
        assert_eq!(api.sidebar().await["uncategorized"][0]["title"], "Mine");
        assert_eq!(
            api.get(&format!("api/items?feed={feed}")).await.0,
            StatusCode::NOT_FOUND
        );

        let (_, body) = api
            .put("api/feeds/mine/title", json!({ "title": null }))
            .await;
        assert_eq!(body["slug"], "example-blog");
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
            .post("api/refresh", json!({ "feed": feed, "folder": "tech" }))
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
        assert_eq!(first["feed_slug"], "example-blog");
        assert!(first["slug"].is_string());
        assert!(first["summary"].is_string());
        assert!(first["published_at"].is_string());
        assert!(first["fetched_at"].is_string());
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
        let a = api.subscribe("a.xml", Some(&tech)).await;
        api.subscribe("b.xml", None).await;
        let first = api.items(&format!("?feed={a}")).await[0].clone();
        api.patch(
            &Api::item_path(&first),
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
        assert_eq!(
            api.get("api/items?feed=nope").await.0,
            StatusCode::NOT_FOUND
        );
    }

    #[tokio::test]
    async fn open_at_their_feed_and_article_slugs() {
        let api = start().await;
        api.subscribe("a.xml", None).await;

        let (status, item) = api.get("api/feeds/example-blog/items/first-post").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(item["title"], "First post");
        assert!(item["content_html"].as_str().unwrap().contains("Hello"));
        assert!(!item["content_html"].as_str().unwrap().contains("script"));
        assert_eq!(
            api.get("api/feeds/example-blog/items/nope").await.0,
            StatusCode::NOT_FOUND
        );
    }

    #[tokio::test]
    async fn can_be_marked_read_and_unread() {
        let api = start().await;
        api.subscribe("a.xml", None).await;
        let path = Api::item_path(&api.items("").await[0]);

        let (status, _) = api.patch(&path, json!({ "read": true })).await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        assert!(api.get(&path).await.1["read_at"].is_string());

        api.patch(&path, json!({ "read": false })).await;
        assert_eq!(api.get(&path).await.1["read_at"], Value::Null);
    }

    #[tokio::test]
    async fn can_be_marked_read_in_bulk_up_to_what_was_seen() {
        let api = start().await;
        let a = api.subscribe("a.xml", None).await;
        api.subscribe("b.xml", None).await;
        let seen_until = api.items(&format!("?feed={a}")).await[0]["fetched_at"].clone();

        let (status, body) = api
            .post(
                "api/items/mark-read",
                json!({ "feed": a, "seen_until": seen_until }),
            )
            .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["marked"], 4);
        assert_eq!(api.sidebar().await["total_unread"], 4);
    }
}

mod opml {
    use super::*;

    impl Api {
        async fn import(&self, body: String) -> (StatusCode, Value) {
            let response = self
                .client
                .post(self.base.join("api/opml").unwrap())
                .header("content-type", "text/x-opml")
                .body(body)
                .send()
                .await
                .unwrap();
            (response.status(), response.json().await.unwrap())
        }
    }

    #[tokio::test]
    async fn import_subscribes_and_fetches_in_the_background() {
        let api = start().await;
        let opml = format!(
            r#"<?xml version="1.0"?><opml version="2.0"><head/><body>
                 <outline text="Tech"><outline type="rss" text="A" xmlUrl="{}"/></outline>
                 <outline type="rss" text="B" xmlUrl="{}"/>
               </body></opml>"#,
            api.sites.join("a.xml").unwrap(),
            api.sites.join("b.xml").unwrap()
        );

        let (status, report) = api.import(opml).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(report, json!({ "added": 2, "skipped": 0, "invalid": [] }));
        assert_eq!(api.sidebar().await["folders"][0]["name"], "Tech");
        for _ in 0..100 {
            if api.sidebar().await["total_unread"] == 8 {
                return;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!("imported feeds were never fetched");
    }

    #[tokio::test]
    async fn import_rejects_documents_that_are_not_opml() {
        let api = start().await;

        let (status, body) = api.import("<html></html>".to_string()).await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body["error"].is_string());
    }

    #[tokio::test]
    async fn export_downloads_an_opml_file() {
        let api = start().await;
        let tech = api.folder("Tech").await;
        api.subscribe("a.xml", Some(&tech)).await;

        let response = api
            .client
            .get(api.base.join("api/opml").unwrap())
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            response.headers()["content-type"]
                .to_str()
                .unwrap()
                .starts_with("text/x-opml")
        );
        assert!(
            response.headers()["content-disposition"]
                .to_str()
                .unwrap()
                .contains("feedrsauros.opml")
        );
        let body = response.text().await.unwrap();
        assert!(body.contains(r#"text="Tech""#));
        assert!(body.contains(api.sites.join("a.xml").unwrap().as_str()));
    }
}

mod web_app {
    use super::*;

    #[tokio::test]
    async fn is_served_for_any_non_api_path_so_links_survive_a_reload() {
        let api = start().await;

        for path in ["", "unread", "feeds/example-blog/items/first-post"] {
            let response = api
                .client
                .get(api.base.join(path).unwrap())
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "/{path}");
            assert!(
                response.headers()["content-type"]
                    .to_str()
                    .unwrap()
                    .starts_with("text/html"),
                "/{path}"
            );
        }
    }

    #[tokio::test]
    async fn unknown_api_routes_stay_json_errors() {
        let api = start().await;

        let (status, body) = api.get("api/nope").await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(body["error"].is_string());
    }
}
