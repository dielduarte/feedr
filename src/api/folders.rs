use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

use super::feeds::Renamed;
use super::{ApiError, AppState};
use crate::db::{Folder, SidebarFeed};

#[derive(Serialize)]
pub struct Sidebar {
    total_unread: u32,
    folders: Vec<SidebarFolder>,
    uncategorized: Vec<SidebarFeed>,
}

#[derive(Serialize)]
struct SidebarFolder {
    slug: String,
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
                slug: f.folder.slug,
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
    Path(slug): Path<String>,
    Json(body): Json<Name>,
) -> Result<Json<Renamed>, ApiError> {
    let id = state.db.folder_id(&slug).await?;
    let slug = state.db.rename_folder(id, &body.name).await?;
    Ok(Json(Renamed { slug }))
}

#[derive(Deserialize)]
pub struct Position {
    index: usize,
}

pub async fn move_to(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Json(body): Json<Position>,
) -> Result<StatusCode, ApiError> {
    let id = state.db.folder_id(&slug).await?;
    state.db.move_folder(id, body.index).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<StatusCode, ApiError> {
    let id = state.db.folder_id(&slug).await?;
    state.db.delete_folder(id).await?;
    Ok(StatusCode::NO_CONTENT)
}
