use chrono::{DateTime, Utc};
use sqlx::sqlite::SqliteRow;
use sqlx::{FromRow, Row};
use url::Url;

use super::{Db, DbError, found, from_ts, place, ts};
use crate::model::{FeedId, FolderId, Validators};
use crate::parse::ParsedFeed;

#[derive(Debug, Clone)]
pub struct NewFeed {
    pub url: Url,
    pub title: String,
    pub site_url: Option<Url>,
    pub folder: Option<FolderId>,
}

#[derive(Debug, Clone)]
pub struct Feed {
    pub id: FeedId,
    pub folder: Option<FolderId>,
    pub url: Url,
    pub site_url: Option<Url>,
    pub title: String,
    pub custom_title: Option<String>,
    pub validators: Validators,
    pub next_fetch_at: DateTime<Utc>,
    pub error_count: u32,
    pub last_error: Option<String>,
}

pub enum FetchRecord<'a> {
    NotModified,
    Updated {
        feed: &'a ParsedFeed,
        validators: Validators,
    },
    Failed {
        error: String,
    },
}

macro_rules! feed_columns {
    () => {
        "id, folder_id, url, site_url, title, custom_title, etag, last_modified, next_fetch_at, error_count, last_error"
    };
}

impl FromRow<'_, SqliteRow> for Feed {
    fn from_row(row: &SqliteRow) -> sqlx::Result<Self> {
        let url: String = row.try_get("url")?;
        Ok(Self {
            id: row.try_get("id")?,
            folder: row.try_get("folder_id")?,
            url: Url::parse(&url).map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
            site_url: parse_optional_url(row.try_get("site_url")?),
            title: row.try_get("title")?,
            custom_title: row.try_get("custom_title")?,
            validators: Validators {
                etag: row.try_get("etag")?,
                last_modified: row.try_get("last_modified")?,
            },
            next_fetch_at: from_ts(row.try_get("next_fetch_at")?),
            error_count: row.try_get("error_count")?,
            last_error: row.try_get("last_error")?,
        })
    }
}

pub(super) fn parse_optional_url(url: Option<String>) -> Option<Url> {
    url.and_then(|u| Url::parse(&u).ok())
}

impl Db {
    /// The new feed is due immediately so the poller picks it up on its next pass.
    pub async fn insert_feed(&self, new: NewFeed, now: DateTime<Utc>) -> Result<Feed, DbError> {
        let feed = sqlx::query_as(concat!(
            "INSERT INTO feeds (folder_id, position, url, site_url, title, next_fetch_at)
             VALUES (?1, (SELECT COALESCE(MAX(position), -1) + 1 FROM feeds WHERE folder_id IS ?1), ?2, ?3, ?4, ?5)
             RETURNING ",
            feed_columns!()
        ))
        .bind(new.folder)
        .bind(new.url.as_str())
        .bind(new.site_url.as_ref().map(Url::as_str))
        .bind(&new.title)
        .bind(ts(now))
        .fetch_one(&self.pool)
        .await?;
        Ok(feed)
    }

    pub async fn feeds_due(&self, now: DateTime<Utc>) -> Result<Vec<Feed>, DbError> {
        Ok(sqlx::query_as(concat!(
            "SELECT ",
            feed_columns!(),
            " FROM feeds WHERE next_fetch_at <= ? ORDER BY next_fetch_at, id"
        ))
        .bind(ts(now))
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn move_feed(
        &self,
        id: FeedId,
        folder: Option<FolderId>,
        index: usize,
    ) -> Result<(), DbError> {
        let mut tx = self.pool.begin().await?;
        let exists: Option<i64> = sqlx::query_scalar("SELECT 1 FROM feeds WHERE id = ?")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
        if exists.is_none() {
            return Err(DbError::NotFound);
        }
        let siblings: Vec<FeedId> =
            sqlx::query_scalar("SELECT id FROM feeds WHERE folder_id IS ? ORDER BY position, id")
                .bind(folder)
                .fetch_all(&mut *tx)
                .await?;
        for (position, feed) in place(siblings, id, index).into_iter().enumerate() {
            sqlx::query("UPDATE feeds SET folder_id = ?, position = ? WHERE id = ?")
                .bind(folder)
                .bind(position as i64)
                .bind(feed)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn set_custom_title(&self, id: FeedId, title: Option<&str>) -> Result<(), DbError> {
        found(
            sqlx::query("UPDATE feeds SET custom_title = ? WHERE id = ?")
                .bind(title)
                .bind(id)
                .execute(&self.pool)
                .await?,
        )
    }

    pub async fn delete_feed(&self, id: FeedId) -> Result<(), DbError> {
        found(
            sqlx::query("DELETE FROM feeds WHERE id = ?")
                .bind(id)
                .execute(&self.pool)
                .await?,
        )
    }

    /// Returns how many previously unseen items were stored.
    pub async fn record_fetch(
        &self,
        id: FeedId,
        record: FetchRecord<'_>,
        now: DateTime<Utc>,
        next_fetch_at: DateTime<Utc>,
    ) -> Result<u64, DbError> {
        let mut tx = self.pool.begin().await?;
        let mut inserted = 0;

        let updated = match record {
            FetchRecord::NotModified => {
                sqlx::query("UPDATE feeds SET error_count = 0, last_error = NULL, next_fetch_at = ? WHERE id = ?")
                    .bind(ts(next_fetch_at))
                    .bind(id)
                    .execute(&mut *tx)
                    .await?
            }
            FetchRecord::Failed { error } => {
                sqlx::query(
                    "UPDATE feeds SET error_count = error_count + 1, last_error = ?, next_fetch_at = ? WHERE id = ?",
                )
                .bind(error)
                .bind(ts(next_fetch_at))
                .bind(id)
                .execute(&mut *tx)
                .await?
            }
            FetchRecord::Updated { feed, validators } => {
                let updated = sqlx::query(
                    "UPDATE feeds
                     SET title = ?, site_url = COALESCE(?, site_url), etag = ?, last_modified = ?,
                         error_count = 0, last_error = NULL, next_fetch_at = ?
                     WHERE id = ?",
                )
                .bind(&feed.title)
                .bind(feed.site_url.as_ref().map(Url::as_str))
                .bind(validators.etag)
                .bind(validators.last_modified)
                .bind(ts(next_fetch_at))
                .bind(id)
                .execute(&mut *tx)
                .await?;

                for item in &feed.items {
                    inserted += sqlx::query(
                        "INSERT INTO items (feed_id, guid, url, title, author, content_html, published_at, fetched_at)
                         VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                         ON CONFLICT (feed_id, guid) DO NOTHING",
                    )
                    .bind(id)
                    .bind(&item.guid)
                    .bind(item.url.as_ref().map(Url::as_str))
                    .bind(&item.title)
                    .bind(&item.author)
                    .bind(item.content.as_ref().map(|c| c.as_str()))
                    .bind(ts(item.published_at))
                    .bind(ts(now))
                    .execute(&mut *tx)
                    .await?
                    .rows_affected();
                }
                updated
            }
        };

        found(updated)?;
        tx.commit().await?;
        Ok(inserted)
    }
}
