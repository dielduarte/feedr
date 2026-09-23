use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

use super::{ApiError, AppState};
use crate::db::{Folder, SidebarFeed};
use crate::model::FolderId;

#[derive(Serialize)]
pub struct Sidebar {
    total_unread: u32,
    folders: Vec<SidebarFolder>,
    uncategorized: Vec<SidebarFeed>,
}

#[derive(Serialize)]
struct SidebarFolder {
    id: FolderId,
    name: String,
    unread: u32,
    feeds: Vec<SidebarFeed>,
}

pub async fn sidebar(State(state): State<AppState>) -> Result<Json<Sidebar>, ApiError> {
    let sidebar = state.db.sidebar().await?;
    Ok(Json(Sidebar {
        total_unread: sidebar.total_unread(),
        folders: sidebar
            .folders
            .into_iter()
            .map(|f| SidebarFolder {
                unread: f.unread(),
                id: f.folder.id,
                name: f.folder.name,
                feeds: f.feeds,
            })
            .collect(),
        uncategorized: sidebar.uncategorized,
    }))
}

#[derive(Deserialize)]
pub struct Name {
    name: String,
}

pub async fn create(
    State(state): State<AppState>,
    Json(body): Json<Name>,
) -> Result<(StatusCode, Json<Folder>), ApiError> {
    let folder = state.db.create_folder(&body.name).await?;
    Ok((StatusCode::CREATED, Json(folder)))
}

pub async fn rename(
    State(state): State<AppState>,
    Path(id): Path<FolderId>,
    Json(body): Json<Name>,
) -> Result<StatusCode, ApiError> {
    state.db.rename_folder(id, &body.name).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct Position {
    index: usize,
}

pub async fn move_to(
    State(state): State<AppState>,
    Path(id): Path<FolderId>,
    Json(body): Json<Position>,
) -> Result<StatusCode, ApiError> {
    state.db.move_folder(id, body.index).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<FolderId>,
) -> Result<StatusCode, ApiError> {
    state.db.delete_folder(id).await?;
    Ok(StatusCode::NO_CONTENT)
}
