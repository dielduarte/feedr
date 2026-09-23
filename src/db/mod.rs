mod feeds;
mod folders;
mod items;
mod sidebar;

use std::path::Path;
use std::time::Duration;

use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteQueryResult};

pub use feeds::{Feed, FetchRecord, NewFeed};
pub use folders::Folder;
pub use items::{Cursor, Item, ItemQuery, ItemScope, ItemSummary, Page};
pub use sidebar::{Sidebar, SidebarFeed, SidebarFolder};

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("not found")]
    NotFound,
    #[error("already exists")]
    AlreadyExists,
    #[error(transparent)]
    Sqlx(sqlx::Error),
    #[error(transparent)]
    Migrate(#[from] sqlx::migrate::MigrateError),
}

impl From<sqlx::Error> for DbError {
    fn from(error: sqlx::Error) -> Self {
        match &error {
            sqlx::Error::RowNotFound => Self::NotFound,
            sqlx::Error::Database(e) if e.is_unique_violation() => Self::AlreadyExists,
            sqlx::Error::Database(e) if e.is_foreign_key_violation() => Self::NotFound,
            _ => Self::Sqlx(error),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Db {
    pool: SqlitePool,
}

impl Db {
    pub async fn open(path: &Path) -> Result<Self, DbError> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .foreign_keys(true)
            .busy_timeout(Duration::from_secs(5));
        let pool = SqlitePoolOptions::new().connect_with(options).await?;
        sqlx::migrate!().run(&pool).await?;
        Ok(Self { pool })
    }
}

fn ts(time: DateTime<Utc>) -> i64 {
    time.timestamp()
}

fn from_ts(secs: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(secs, 0).unwrap_or_default()
}

fn found(result: SqliteQueryResult) -> Result<(), DbError> {
    match result.rows_affected() {
        0 => Err(DbError::NotFound),
        _ => Ok(()),
    }
}

/// Moves `id` to `index` within `siblings` (clamped to the end), returning the new order.
fn place<T: PartialEq>(mut siblings: Vec<T>, id: T, index: usize) -> Vec<T> {
    siblings.retain(|s| *s != id);
    siblings.insert(index.min(siblings.len()), id);
    siblings
}
