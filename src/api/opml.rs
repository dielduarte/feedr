use axum::Json;
use axum::extract::State;
use axum::http::header;
use axum::response::IntoResponse;
use chrono::Utc;

use super::{ApiError, AppState};
use crate::opml::{ImportReport, import, parse, render};

pub async fn export(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let sidebar = state.db.sidebar().await?;
    Ok((
        [
            (header::CONTENT_TYPE, "text/x-opml; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"feedrsauros.opml\"",
            ),
        ],
        render(&sidebar),
    ))
}

pub async fn import_file(
    State(state): State<AppState>,
    body: String,
) -> Result<Json<ImportReport>, ApiError> {
    let subscriptions = parse(&body).map_err(|e| ApiError::BadRequest(e.to_string()))?;
    let report = import(&state.db, subscriptions, Utc::now()).await?;
    state.poller.wake();
    Ok(Json(report))
}
