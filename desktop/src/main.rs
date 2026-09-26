// Release builds have no console window on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod links;

use std::net::Ipv4Addr;
use std::sync::Mutex;

use feedrsauros::cli::database_path;
use feedrsauros::db::Db;
use feedrsauros::server::{self, Server};
use tauri::webview::NewWindowResponse;
#[cfg(target_os = "macos")]
use tauri::{LogicalPosition, TitleBarStyle};
use tauri::{Manager, RunEvent, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent};

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
            let builder = WebviewWindowBuilder::new(app, "main", WebviewUrl::External(home))
                .title("feedrsauros")
                .inner_size(1280.0, 820.0)
                // Wide enough that the web app never switches to its phone layout.
                .min_inner_size(800.0, 480.0)
                .on_navigation(move |target| {
                    is_app_url(&in_window, target) || open_in_browser(target)
                })
                .on_new_window(move |target, _| {
                    if !is_app_url(&in_new_window, &target) {
                        open_in_browser(&target);
                    }
                    NewWindowResponse::Deny
                });
            // The page draws its own title bar, with the traffic lights over the sidebar.
            #[cfg(target_os = "macos")]
            let builder = builder
                .title_bar_style(TitleBarStyle::Overlay)
                .hidden_title(true)
                .traffic_light_position(LogicalPosition::new(20.0, 33.0))
                // Set before the page's own scripts, so its first render already leaves room.
                .initialization_script("window.__FEEDRSAUROS_TRAFFIC_LIGHTS__ = true")
                .on_page_load(|window, _| show_traffic_lights(&window));
            let window = builder.build()?;
            let events_window = window.clone();
            window.on_window_event(move |event| match event {
                WindowEvent::Focused(focused) => poller.set_active(*focused),
                // Entering or leaving fullscreen resizes the window.
                WindowEvent::Resized(_) => show_traffic_lights(&events_window),
                _ => {}
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

/// Fullscreen hides the traffic lights, so the page stops leaving room for them.
fn show_traffic_lights(window: &WebviewWindow) {
    let visible = cfg!(target_os = "macos") && !window.is_fullscreen().unwrap_or(false);
    let _ = window.eval(format!(
        "document.documentElement.toggleAttribute('data-traffic-lights', {visible})"
    ));
}

/// Always returns false, so the window itself never navigates away.
fn open_in_browser(target: &Url) -> bool {
    let _ = open::that_detached(target.as_str());
    false
}
