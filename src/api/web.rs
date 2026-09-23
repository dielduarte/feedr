use axum::http::{StatusCode, Uri, header};
use axum::response::{Html, IntoResponse, Response};

use super::ApiError;

/// The built web app. Release builds embed it; debug builds read it from disk, so a frontend
/// rebuild shows up without recompiling.
#[derive(rust_embed::Embed)]
#[folder = "web/dist"]
#[allow_missing = true]
struct Assets;

const NOT_BUILT: &str = "<!doctype html><title>feedr</title>\
    <p>The web app hasn't been built yet. Run <code>pnpm --dir web install && pnpm --dir web build</code>, \
    then reload.</p>";

/// Serves static files, and `index.html` for every other non-API path so client-side routes
/// survive a reload.
pub async fn serve(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    if path == "api" || path.starts_with("api/") {
        return ApiError::NotFound.into_response();
    }

    if let Some(file) = Assets::get(path).filter(|_| !path.is_empty()) {
        // Vite fingerprints everything under assets/, so those files never change in place.
        let cache = if path.starts_with("assets/") {
            "public, max-age=31536000, immutable"
        } else {
            "no-cache"
        };
        return (
            [
                (header::CONTENT_TYPE, file.metadata.mimetype().to_string()),
                (header::CACHE_CONTROL, cache.to_string()),
            ],
            file.data,
        )
            .into_response();
    }

    match Assets::get("index.html") {
        Some(index) => ([(header::CACHE_CONTROL, "no-cache")], Html(index.data)).into_response(),
        None => (StatusCode::OK, Html(NOT_BUILT)).into_response(),
    }
}
