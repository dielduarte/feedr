use std::time::Duration;

use feedrsauros::db::Db;
use feedrsauros::server;

#[tokio::test]
async fn serves_the_app_and_shuts_down_with_an_event_stream_open() {
    let dir = tempfile::tempdir().unwrap();
    let db = Db::open(&dir.path().join("feedrsauros.db")).await.unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();

    let server = server::start(db, listener).await.unwrap();
    let base = format!("http://{}", server.addr);

    let sidebar = reqwest::get(format!("{base}/api/sidebar")).await.unwrap();
    assert_eq!(sidebar.status(), 200);
    let _events = reqwest::get(format!("{base}/api/events")).await.unwrap();

    tokio::time::timeout(Duration::from_secs(5), server.shutdown())
        .await
        .expect("shutdown hung")
        .unwrap();
}
