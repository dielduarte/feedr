use std::path::PathBuf;
use std::process::{Output, Stdio};
use std::time::Duration;

use axum::Router;
use axum::http::StatusCode;
use axum::routing::get;
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use url::Url;

fn rss2() -> Vec<u8> {
    std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/rss2.xml"
    ))
    .unwrap()
}

struct Env {
    sites: Url,
    dir: TempDir,
}

async fn env() -> Env {
    let app = Router::new()
        .route("/a.xml", get(|| async { rss2() }))
        .route("/b.xml", get(|| async { rss2() }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let sites = Url::parse(&format!("http://{}/", listener.local_addr().unwrap())).unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    Env {
        sites,
        dir: tempfile::tempdir().unwrap(),
    }
}

impl Env {
    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_feedr"));
        command.arg("--db").arg(self.dir.path().join("feedr.db"));
        command
    }

    async fn run(&self, args: &[&str]) -> Output {
        self.command().args(args).output().await.unwrap()
    }

    /// Runs a command that must succeed and returns its stdout.
    async fn ok(&self, args: &[&str]) -> String {
        let output = self.run(args).await;
        assert!(
            output.status.success(),
            "feedr {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    }

    fn site(&self, path: &str) -> String {
        self.sites.join(path).unwrap().to_string()
    }

    fn write(&self, name: &str, contents: &str) -> PathBuf {
        let path = self.dir.path().join(name);
        std::fs::write(&path, contents).unwrap();
        path
    }
}

#[tokio::test]
async fn adds_feeds_into_folders_and_lists_them() {
    let env = env().await;

    let added = env
        .ok(&["add", &env.site("a.xml"), "--folder", "Tech"])
        .await;
    env.ok(&["add", &env.site("b.xml")]).await;
    let listing = env.ok(&["ls"]).await;

    assert!(added.contains("Example Blog"), "{added}");
    assert!(added.contains("4 new items"), "{added}");
    assert_eq!(
        listing,
        "Tech\n    4  Example Blog\nUncategorized\n    4  Example Blog\n8 unread\n"
    );
}

#[tokio::test]
async fn explains_why_a_feed_was_not_added() {
    let env = env().await;
    env.ok(&["add", &env.site("a.xml")]).await;

    let duplicate = env.run(&["add", &env.site("a.xml")]).await;
    let invalid = env.run(&["add", "not a url"]).await;

    assert!(!duplicate.status.success());
    assert!(String::from_utf8_lossy(&duplicate.stderr).contains("already subscribed"));
    assert!(!invalid.status.success());
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("not a web address"));
}

#[tokio::test]
async fn imports_then_refreshes_everything() {
    let env = env().await;
    let file = env.write(
        "subscriptions.opml",
        &format!(
            r#"<opml version="2.0"><body>
                 <outline text="Tech"><outline text="A" xmlUrl="{}"/></outline>
                 <outline text="B" xmlUrl="{}"/>
               </body></opml>"#,
            env.site("a.xml"),
            env.site("b.xml")
        ),
    );

    let imported = env.ok(&["import", file.to_str().unwrap()]).await;
    let refreshed = env.ok(&["refresh"]).await;

    assert!(imported.contains("Added 2 feeds"), "{imported}");
    assert!(refreshed.contains("8 new items"), "{refreshed}");
    assert!(env.ok(&["ls"]).await.ends_with("8 unread\n"));
}

#[tokio::test]
async fn exports_opml_to_stdout() {
    let env = env().await;
    env.ok(&["add", &env.site("a.xml"), "--folder", "Tech"])
        .await;

    let exported = env.ok(&["export"]).await;

    assert!(exported.starts_with("<?xml"));
    assert!(exported.contains(r#"text="Tech""#));
    assert!(exported.contains(&env.site("a.xml")));
}

#[tokio::test]
async fn serves_the_api_and_shuts_down_cleanly_with_clients_connected() {
    let env = env().await;
    let mut server = env
        .command()
        .args(["serve", "--port", "0"])
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut lines = BufReader::new(server.stdout.take().unwrap()).lines();
    let banner = lines.next_line().await.unwrap().unwrap();
    let base = Url::parse(banner.split_whitespace().last().unwrap()).unwrap();

    let sidebar = reqwest::get(base.join("api/sidebar").unwrap())
        .await
        .unwrap();
    assert_eq!(sidebar.status(), StatusCode::OK);
    let _events = reqwest::get(base.join("api/events").unwrap())
        .await
        .unwrap();

    let pid = server.id().unwrap().to_string();
    Command::new("kill")
        .args(["-INT", &pid])
        .status()
        .await
        .unwrap();

    let status = tokio::time::timeout(Duration::from_secs(5), server.wait())
        .await
        .expect("server did not shut down")
        .unwrap();
    assert!(status.success());
}

#[tokio::test]
async fn serves_on_the_host_and_port_from_the_environment() {
    let env = env().await;
    let port = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    };
    let mut server = env
        .command()
        .arg("serve")
        .env("FEEDR_HOST", "0.0.0.0")
        .env("FEEDR_PORT", port.to_string())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut lines = BufReader::new(server.stdout.take().unwrap()).lines();

    let banner = lines.next_line().await.unwrap().unwrap();

    assert!(
        banner.ends_with(&format!("http://0.0.0.0:{port}")),
        "{banner}"
    );
    let sidebar = reqwest::get(format!("http://127.0.0.1:{port}/api/sidebar"))
        .await
        .unwrap();
    assert_eq!(sidebar.status(), StatusCode::OK);
    server.kill().await.unwrap();
}
