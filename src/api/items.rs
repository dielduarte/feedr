use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{ApiError, AppState};
use crate::db::{Cursor, Db, Item, ItemQuery, ItemScope, ItemSummary};
use crate::model::ItemId;

const DEFAULT_LIMIT: u32 = 50;
const MAX_LIMIT: u32 = 200;

async fn scope(
    db: &Db,
    feed: Option<String>,
    folder: Option<String>,
    starred: Option<bool>,
) -> Result<ItemScope, ApiError> {
    match (feed, folder, starred.unwrap_or(false)) {
        (None, None, false) => Ok(ItemScope::All),
        (Some(feed), None, false) => Ok(ItemScope::Feed(db.feed_id(&feed).await?)),
        (None, Some(folder), false) => Ok(ItemScope::Folder(db.folder_id(&folder).await?)),
        (None, None, true) => Ok(ItemScope::Starred),
        _ => Err(ApiError::BadRequest(
            "choose one of feed, folder or starred".into(),
        )),
    }
}

/// Opaque to clients: `<published_at seconds>_<internal id>`.
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
    feed: Option<String>,
    folder: Option<String>,
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
            scope: scope(&state.db, query.feed, query.folder, query.starred).await?,
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
    Path((feed, item)): Path<(String, String)>,
) -> Result<Json<Item>, ApiError> {
    let id = state.db.item_id(&feed, &item).await?;
    Ok(Json(state.db.get_item(id).await?))
}

#[derive(Deserialize)]
pub struct Update {
    read: Option<bool>,
    starred: Option<bool>,
}

pub async fn update(
    State(state): State<AppState>,
    Path((feed, item)): Path<(String, String)>,
    Json(body): Json<Update>,
) -> Result<StatusCode, ApiError> {
    let id = state.db.item_id(&feed, &item).await?;
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
    feed: Option<String>,
    folder: Option<String>,
    starred: Option<bool>,
    /// The newest `fetched_at` the reader has seen; anything stored after it stays unread.
    seen_until: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct Marked {
    marked: u64,
}

pub async fn mark_read(
    State(state): State<AppState>,
    Json(body): Json<MarkRead>,
) -> Result<Json<Marked>, ApiError> {
    let scope = scope(&state.db, body.feed, body.folder, body.starred).await?;
    let marked = state.db.mark_read(scope, body.seen_until).await?;
    Ok(Json(Marked { marked }))
}
