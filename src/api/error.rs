use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

use crate::add_feed::AddFeedError;
use crate::db::DbError;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{0}")]
    BadRequest(String),
    #[error("not found")]
    NotFound,
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    Unprocessable(String),
    /// The upstream site failed, not us.
    #[error("{0}")]
    BadGateway(String),
    #[error("internal error")]
    Internal,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::Unprocessable(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::BadGateway(_) => StatusCode::BAD_GATEWAY,
            Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(json!({ "error": self.to_string() }))).into_response()
    }
}

impl From<DbError> for ApiError {
    fn from(error: DbError) -> Self {
        match error {
            DbError::NotFound => Self::NotFound,
            DbError::AlreadyExists => Self::Conflict(error.to_string()),
            other => {
                tracing::error!(error = %other, "database error");
                Self::Internal
            }
        }
    }
}

impl From<AddFeedError> for ApiError {
    fn from(error: AddFeedError) -> Self {
        match error {
            AddFeedError::Fetch(_) => Self::BadGateway(error.to_string()),
            AddFeedError::NoFeedFound => Self::Unprocessable(error.to_string()),
            AddFeedError::AlreadySubscribed => Self::Conflict(error.to_string()),
            AddFeedError::Db(db) => db.into(),
        }
    }
}
