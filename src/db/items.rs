use chrono::{DateTime, Utc};
use sqlx::sqlite::SqliteRow;
use sqlx::{FromRow, QueryBuilder, Row, Sqlite};
use url::Url;

use super::feeds::parse_optional_url;
use super::{Db, DbError, found, from_ts, ts};
use crate::model::{FeedId, FolderId, ItemId};
use crate::sanitize::SanitizedHtml;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemScope {
    All,
    Folder(FolderId),
    Feed(FeedId),
    Starred,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    pub published_at: DateTime<Utc>,
    pub id: ItemId,
}

#[derive(Debug, Clone)]
pub struct ItemQuery {
    pub scope: ItemScope,
    pub unread_only: bool,
    pub cursor: Option<Cursor>,
    pub limit: u32,
}

#[derive(Debug, Clone)]
pub struct Page {
    pub items: Vec<ItemSummary>,
    pub next: Option<Cursor>,
}

#[derive(Debug, Clone)]
pub struct ItemSummary {
    pub id: ItemId,
    pub feed_id: FeedId,
    pub feed_title: String,
    pub url: Option<Url>,
    pub title: Option<String>,
    pub author: Option<String>,
    pub published_at: DateTime<Utc>,
    pub read_at: Option<DateTime<Utc>>,
    pub starred_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct Item {
    pub summary: ItemSummary,
    pub content: Option<SanitizedHtml>,
}

macro_rules! summary_select {
    () => {
        "SELECT items.id, items.feed_id, COALESCE(feeds.custom_title, feeds.title) AS feed_title,
                items.url, items.title, items.author, items.published_at, items.read_at, items.starred_at"
    };
}

impl FromRow<'_, SqliteRow> for ItemSummary {
    fn from_row(row: &SqliteRow) -> sqlx::Result<Self> {
        Ok(Self {
            id: row.try_get("id")?,
            feed_id: row.try_get("feed_id")?,
            feed_title: row.try_get("feed_title")?,
            url: parse_optional_url(row.try_get("url")?),
            title: row.try_get("title")?,
            author: row.try_get("author")?,
            published_at: from_ts(row.try_get("published_at")?),
            read_at: row.try_get::<Option<i64>, _>("read_at")?.map(from_ts),
            starred_at: row.try_get::<Option<i64>, _>("starred_at")?.map(from_ts),
        })
    }
}

impl FromRow<'_, SqliteRow> for Item {
    fn from_row(row: &SqliteRow) -> sqlx::Result<Self> {
        Ok(Self {
            summary: ItemSummary::from_row(row)?,
            content: row
                .try_get::<Option<String>, _>("content_html")?
                .map(SanitizedHtml::from_stored),
        })
    }
}

/// Written against the unaliased `items` table so it works in both SELECT and UPDATE.
fn push_scope(query: &mut QueryBuilder<Sqlite>, scope: ItemScope) {
    match scope {
        ItemScope::All => {}
        ItemScope::Feed(id) => {
            query.push(" AND items.feed_id = ").push_bind(id);
        }
        ItemScope::Folder(id) => {
            query
                .push(" AND items.feed_id IN (SELECT id FROM feeds WHERE folder_id = ")
                .push_bind(id)
                .push(")");
        }
        ItemScope::Starred => {
            query.push(" AND items.starred_at IS NOT NULL");
        }
    }
}

impl Db {
    pub async fn list_items(&self, query: ItemQuery) -> Result<Page, DbError> {
        let limit = query.limit as usize;
        let mut sql = QueryBuilder::new(concat!(
            summary_select!(),
            " FROM items JOIN feeds ON feeds.id = items.feed_id WHERE 1 = 1"
        ));
        push_scope(&mut sql, query.scope);
        if query.unread_only {
            sql.push(" AND items.read_at IS NULL");
        }
        if let Some(cursor) = query.cursor {
            sql.push(" AND (items.published_at, items.id) < (")
                .push_bind(ts(cursor.published_at))
                .push(", ")
                .push_bind(cursor.id)
                .push(")");
        }
        // One extra row tells us whether another page exists.
        sql.push(" ORDER BY items.published_at DESC, items.id DESC LIMIT ")
            .push_bind(limit as i64 + 1);

        let mut items: Vec<ItemSummary> = sql.build_query_as().fetch_all(&self.pool).await?;
        let next = if items.len() > limit {
            items.truncate(limit);
            items.last().map(|i| Cursor {
                published_at: i.published_at,
                id: i.id,
            })
        } else {
            None
        };
        Ok(Page { items, next })
    }

    pub async fn get_item(&self, id: ItemId) -> Result<Item, DbError> {
        Ok(sqlx::query_as(concat!(
            summary_select!(),
            ", items.content_html FROM items JOIN feeds ON feeds.id = items.feed_id WHERE items.id = ?"
        ))
        .bind(id)
        .fetch_one(&self.pool)
        .await?)
    }

    pub async fn set_read(&self, id: ItemId, read: bool) -> Result<(), DbError> {
        found(
            sqlx::query(
                "UPDATE items SET read_at = CASE WHEN ? THEN COALESCE(read_at, unixepoch()) END WHERE id = ?",
            )
            .bind(read)
            .bind(id)
            .execute(&self.pool)
            .await?,
        )
    }

    pub async fn set_starred(&self, id: ItemId, starred: bool) -> Result<(), DbError> {
        found(
            sqlx::query(
                "UPDATE items SET starred_at = CASE WHEN ? THEN COALESCE(starred_at, unixepoch()) END WHERE id = ?",
            )
            .bind(starred)
            .bind(id)
            .execute(&self.pool)
            .await?,
        )
    }

    /// Item ids grow with insertion, so `up_to` (the newest id the user has seen) spares
    /// items fetched after that, even when their publish date is older.
    pub async fn mark_read(&self, scope: ItemScope, up_to: ItemId) -> Result<u64, DbError> {
        let mut sql = QueryBuilder::new(
            "UPDATE items SET read_at = unixepoch() WHERE items.read_at IS NULL AND items.id <= ",
        );
        sql.push_bind(up_to);
        push_scope(&mut sql, scope);
        Ok(sql.build().execute(&self.pool).await?.rows_affected())
    }
}
