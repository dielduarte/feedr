use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::{ApiError, AppState};
use crate::add_feed::{Added, add_feed, parse_input};
use crate::model::{FeedId, FeedScope, FolderId};

#[derive(Deserialize)]
pub struct Subscribe {
    url: String,
    folder_id: Option<FolderId>,
}

pub async fn subscribe(
    State(state): State<AppState>,
    Json(body): Json<Subscribe>,
) -> Result<(StatusCode, Json<Added>), ApiError> {
    let url = parse_input(&body.url)
        .ok_or_else(|| ApiError::BadRequest(format!("not a web address: {}", body.url)))?;
    let added = add_feed(&state.db, &state.fetcher, &url, body.folder_id, Utc::now()).await?;
    Ok((StatusCode::CREATED, Json(added)))
}

pub async fn unsubscribe(
    State(state): State<AppState>,
    Path(id): Path<FeedId>,
) -> Result<StatusCode, ApiError> {
    state.db.delete_feed(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct Position {
    /// `null` moves the feed out of any folder.
    folder_id: Option<FolderId>,
    index: usize,
}

pub async fn move_to(
    State(state): State<AppState>,
    Path(id): Path<FeedId>,
    Json(body): Json<Position>,
) -> Result<StatusCode, ApiError> {
    state.db.move_feed(id, body.folder_id, body.index).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct Title {
    /// `null` goes back to the title the feed publishes.
    title: Option<String>,
}

pub async fn rename(
    State(state): State<AppState>,
    Path(id): Path<FeedId>,
    Json(body): Json<Title>,
) -> Result<StatusCode, ApiError> {
    state.db.set_custom_title(id, body.title.as_deref()).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct Refresh {
    feed: Option<FeedId>,
    folder: Option<FolderId>,
}

#[derive(Serialize)]
pub struct Scheduled {
    scheduled: u64,
}

pub async fn refresh(
    State(state): State<AppState>,
    Json(body): Json<Refresh>,
) -> Result<(StatusCode, Json<Scheduled>), ApiError> {
    let scope = match (body.feed, body.folder) {
        (None, None) => FeedScope::All,
        (Some(feed), None) => FeedScope::Feed(feed),
        (None, Some(folder)) => FeedScope::Folder(folder),
        (Some(_), Some(_)) => {
            return Err(ApiError::BadRequest(
                "refresh either a feed or a folder".into(),
            ));
        }
    };
    let scheduled = state.poller.refresh(scope).await?;
    Ok((StatusCode::ACCEPTED, Json(Scheduled { scheduled })))
}
