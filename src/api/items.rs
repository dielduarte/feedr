use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::DateTime;
use serde::{Deserialize, Serialize};

use super::{ApiError, AppState};
use crate::db::{Cursor, Item, ItemQuery, ItemScope, ItemSummary};
use crate::model::{FeedId, FolderId, ItemId};

const DEFAULT_LIMIT: u32 = 50;
const MAX_LIMIT: u32 = 200;

fn scope(
    feed: Option<FeedId>,
    folder: Option<FolderId>,
    starred: Option<bool>,
) -> Result<ItemScope, ApiError> {
    match (feed, folder, starred.unwrap_or(false)) {
        (None, None, false) => Ok(ItemScope::All),
        (Some(feed), None, false) => Ok(ItemScope::Feed(feed)),
        (None, Some(folder), false) => Ok(ItemScope::Folder(folder)),
        (None, None, true) => Ok(ItemScope::Starred),
        _ => Err(ApiError::BadRequest(
            "choose one of feed, folder or starred".into(),
        )),
    }
}

/// Opaque to clients: `<published_at seconds>_<item id>`.
fn encode_cursor(cursor: Cursor) -> String {
    format!("{}_{}", cursor.published_at.timestamp(), cursor.id.0)
}

fn decode_cursor(raw: &str) -> Result<Cursor, ApiError> {
    let invalid = || ApiError::BadRequest(format!("invalid cursor: {raw}"));
    let (secs, id) = raw.split_once('_').ok_or_else(invalid)?;
    Ok(Cursor {
        published_at: DateTime::from_timestamp(secs.parse().map_err(|_| invalid())?, 0)
            .ok_or_else(invalid)?,
        id: ItemId(id.parse().map_err(|_| invalid())?),
    })
}

// Flat on purpose: serde's `flatten` breaks number parsing for query strings.
#[derive(Deserialize)]
pub struct ListQuery {
    feed: Option<FeedId>,
    folder: Option<FolderId>,
    starred: Option<bool>,
    unread: Option<bool>,
    cursor: Option<String>,
    limit: Option<u32>,
}

#[derive(Serialize)]
pub struct Page {
    items: Vec<ItemSummary>,
    next_cursor: Option<String>,
}

pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Page>, ApiError> {
    let page = state
        .db
        .list_items(ItemQuery {
            scope: scope(query.feed, query.folder, query.starred)?,
            unread_only: query.unread.unwrap_or(false),
            cursor: query.cursor.as_deref().map(decode_cursor).transpose()?,
            limit: query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT),
        })
        .await?;
    Ok(Json(Page {
        items: page.items,
        next_cursor: page.next.map(encode_cursor),
    }))
}

pub async fn open(
    State(state): State<AppState>,
    Path(id): Path<ItemId>,
) -> Result<Json<Item>, ApiError> {
    Ok(Json(state.db.get_item(id).await?))
}

#[derive(Deserialize)]
pub struct Update {
    read: Option<bool>,
    starred: Option<bool>,
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<ItemId>,
    Json(body): Json<Update>,
) -> Result<StatusCode, ApiError> {
    if let Some(read) = body.read {
        state.db.set_read(id, read).await?;
    }
    if let Some(starred) = body.starred {
        state.db.set_starred(id, starred).await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct MarkRead {
    feed: Option<FeedId>,
    folder: Option<FolderId>,
    starred: Option<bool>,
    /// The newest item the reader has seen; anything fetched after it stays unread.
    up_to: ItemId,
}

#[derive(Serialize)]
pub struct Marked {
    marked: u64,
}

pub async fn mark_read(
    State(state): State<AppState>,
    Json(body): Json<MarkRead>,
) -> Result<Json<Marked>, ApiError> {
    let scope = scope(body.feed, body.folder, body.starred)?;
    let marked = state.db.mark_read(scope, body.up_to).await?;
    Ok(Json(Marked { marked }))
}
