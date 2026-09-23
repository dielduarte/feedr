mod error;
mod events;
mod feeds;
mod folders;
mod items;
mod opml;
mod web;

use axum::Router;
use axum::routing::{get, patch, post, put};

use crate::db::Db;
use crate::fetch::Fetcher;
use crate::poller::PollerHandle;

pub use error::ApiError;

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub fetcher: Fetcher,
    pub poller: PollerHandle,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/events", get(events::stream))
        .route("/api/sidebar", get(folders::sidebar))
        .route("/api/folders", post(folders::create))
        .route(
            "/api/folders/{id}",
            patch(folders::rename).delete(folders::delete),
        )
        .route("/api/folders/{id}/position", put(folders::move_to))
        .route("/api/feeds", post(feeds::subscribe))
        .route("/api/feeds/{id}", axum::routing::delete(feeds::unsubscribe))
        .route("/api/feeds/{id}/position", put(feeds::move_to))
        .route("/api/feeds/{id}/title", put(feeds::rename))
        .route("/api/refresh", post(feeds::refresh))
        .route("/api/items", get(items::list))
        .route("/api/items/mark-read", post(items::mark_read))
        .route("/api/items/{id}", get(items::open).patch(items::update))
        .route("/api/opml", get(opml::export).post(opml::import_file))
        .fallback(web::serve)
        .with_state(state)
}
