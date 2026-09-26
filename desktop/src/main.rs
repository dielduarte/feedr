// Release builds have no console window on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod links;

use std::net::Ipv4Addr;
use std::sync::Mutex;

use feedrsauros::cli::database_path;
use feedrsauros::db::Db;
use feedrsauros::server::{self, Server};
use tauri::webview::NewWindowResponse;
use tauri::{Manager, RunEvent, Url, WebviewUrl, WebviewWindowBuilder, WindowEvent};

use crate::links::is_app_url;

/// Taken on exit so the server can shut down cleanly.
struct Running(Mutex<Option<Server>>);

fn main() {
    let app = tauri::Builder::default()
        .setup(|app| {
            let server = tauri::async_runtime::block_on(start())?;
            let home: Url = format!("http://{}/", server.addr).parse()?;
            let poller = server.poller.clone();

            let (in_window, in_new_window) = (home.clone(), home.clone());
            let window = WebviewWindowBuilder::new(app, "main", WebviewUrl::External(home))
                .title("feedrsauros")
                .inner_size(1280.0, 820.0)
                .min_inner_size(720.0, 480.0)
                .on_navigation(move |target| is_app_url(&in_window, target) || open_in_browser(target))
                .on_new_window(move |target, _| {
                    if !is_app_url(&in_new_window, &target) {
                        open_in_browser(&target);
                    }
                    NewWindowResponse::Deny
                })
                .build()?;
            window.on_window_event(move |event| {
                if let WindowEvent::Focused(focused) = event {
                    poller.set_active(*focused);
                }
            });

            app.manage(Running(Mutex::new(Some(server))));
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("feedrsauros could not start");

    app.run(|app, event| {
        if let RunEvent::Exit = event {
            let server = app.state::<Running>().0.lock().expect("server lock").take();
            if let Some(server) = server {
                let _ = tauri::async_runtime::block_on(server.shutdown());
            }
        }
    });
}

/// The same database as the CLI, so feeds added from the terminal show up here too.
async fn start() -> anyhow::Result<Server> {
    let path = database_path(std::env::var_os("FEEDRSAUROS_DB").map(Into::into))?;
    let db = Db::open(&path).await?;
    let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await?;
    server::start(db, listener).await
}

/// Always returns false, so the window itself never navigates away.
fn open_in_browser(target: &Url) -> bool {
    let _ = open::that_detached(target.as_str());
    false
}
