use std::collections::HashSet;

use chrono::{DateTime, Utc};
use sqlx::SqliteConnection;
use url::Url;

use super::{Db, DbError, found, from_ts, place, ts};
use crate::model::{FeedId, FolderId, Validators};
use crate::parse::ParsedFeed;
use crate::slugs::{slugify, unique_slug};

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
    pub slug: String,
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
    /// We couldn't tell whether the feed is healthy (e.g. the machine is offline): try again
    /// later without touching its error state.
    Postponed,
}

struct FeedRow {
    id: FeedId,
    slug: String,
    folder_id: Option<FolderId>,
    url: String,
    site_url: Option<String>,
    title: String,
    custom_title: Option<String>,
    etag: Option<String>,
    last_modified: Option<String>,
    next_fetch_at: i64,
    error_count: u32,
    last_error: Option<String>,
}

impl TryFrom<FeedRow> for Feed {
    type Error = DbError;

    fn try_from(row: FeedRow) -> Result<Self, DbError> {
        Ok(Self {
            id: row.id,
            slug: row.slug,
            folder: row.folder_id,
            url: parse_stored_url(&row.url)?,
            site_url: parse_optional_url(row.site_url),
            title: row.title,
            custom_title: row.custom_title,
            validators: Validators {
                etag: row.etag,
                last_modified: row.last_modified,
            },
            next_fetch_at: from_ts(row.next_fetch_at),
            error_count: row.error_count,
            last_error: row.last_error,
        })
    }
}

/// Feed URLs are validated before they are stored, so a failure here means a corrupt row.
pub(super) fn parse_stored_url(url: &str) -> Result<Url, DbError> {
    Url::parse(url).map_err(|e| DbError::Sqlx(sqlx::Error::Decode(Box::new(e))))
}

pub(super) fn parse_optional_url(url: Option<String>) -> Option<Url> {
    url.and_then(|u| Url::parse(&u).ok())
}

impl Db {
    /// The new feed is due immediately so the poller picks it up on its next pass.
    pub async fn insert_feed(&self, new: NewFeed, now: DateTime<Utc>) -> Result<Feed, DbError> {
        insert_feed(&mut *self.pool.acquire().await?, &new, now).await
    }

    /// Stores a feed together with its first fetch, so a subscription never exists without items.
    /// Returns the new feed and how many items were stored.
    pub async fn subscribe(
        &self,
        new: NewFeed,
        feed: &ParsedFeed,
        validators: Validators,
        now: DateTime<Utc>,
        next_fetch_at: DateTime<Utc>,
    ) -> Result<(Feed, u64), DbError> {
        let mut tx = self.pool.begin().await?;
        let stored = insert_feed(&mut tx, &new, now).await?;
        let record = FetchRecord::Updated { feed, validators };
        let inserted = apply_fetch(&mut tx, stored.id, record, now, next_fetch_at).await?;
        tx.commit().await?;
        Ok((stored, inserted))
    }

    pub async fn feeds_due(&self, now: DateTime<Utc>) -> Result<Vec<Feed>, DbError> {
        let now = ts(now);
        sqlx::query_as!(
            FeedRow,
            r#"SELECT id AS "id: FeedId", slug, folder_id AS "folder_id: FolderId", url, site_url, title,
                      custom_title, etag, last_modified, next_fetch_at, error_count AS "error_count: u32", last_error
               FROM feeds
               WHERE next_fetch_at <= ?
               ORDER BY next_fetch_at, id"#,
            now
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(Feed::try_from)
        .collect()
    }

    pub async fn move_feed(
        &self,
        id: FeedId,
        folder: Option<FolderId>,
        index: usize,
    ) -> Result<(), DbError> {
        let mut tx = self.pool.begin().await?;
        sqlx::query_scalar!("SELECT 1 FROM feeds WHERE id = ?", id)
            .fetch_one(&mut *tx)
            .await?;
        let siblings = sqlx::query_scalar!(
            r#"SELECT id AS "id: FeedId" FROM feeds WHERE folder_id IS ? ORDER BY position, id"#,
            folder
        )
        .fetch_all(&mut *tx)
        .await?;
        for (position, feed) in place(siblings, id, index).into_iter().enumerate() {
            let position = position as i64;
            sqlx::query!(
                "UPDATE feeds SET folder_id = ?, position = ? WHERE id = ?",
                folder,
                position,
                feed
            )
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// Returns the new slug: a feed's URL follows the name it's shown with.
    pub async fn set_custom_title(
        &self,
        id: FeedId,
        title: Option<&str>,
    ) -> Result<String, DbError> {
        let mut tx = self.pool.begin().await?;
        let feed_title = sqlx::query_scalar!("SELECT title FROM feeds WHERE id = ?", id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(DbError::NotFound)?;
        let slug = free_feed_slug(&mut tx, title.unwrap_or(&feed_title), Some(id)).await?;
        sqlx::query!(
            "UPDATE feeds SET custom_title = ?, slug = ? WHERE id = ?",
            title,
            slug,
            id
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(slug)
    }

    pub async fn feed_id(&self, slug: &str) -> Result<FeedId, DbError> {
        sqlx::query_scalar!(
            r#"SELECT id AS "id: FeedId" FROM feeds WHERE slug = ?"#,
            slug
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or(DbError::NotFound)
    }

    pub async fn delete_feed(&self, id: FeedId) -> Result<(), DbError> {
        found(
            sqlx::query!("DELETE FROM feeds WHERE id = ?", id)
                .execute(&self.pool)
                .await?,
        )
    }

    /// Feeds whose last fetch succeeded, most recently checked first.
    pub async fn healthy_feeds(&self, limit: u32) -> Result<Vec<Feed>, DbError> {
        sqlx::query_as!(
            FeedRow,
            r#"SELECT id AS "id: FeedId", slug, folder_id AS "folder_id: FolderId", url, site_url, title,
                      custom_title, etag, last_modified, next_fetch_at, error_count AS "error_count: u32", last_error
               FROM feeds
               WHERE error_count = 0
               ORDER BY next_fetch_at DESC
               LIMIT ?"#,
            limit
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(Feed::try_from)
        .collect()
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
        let inserted = apply_fetch(&mut tx, id, record, now, next_fetch_at).await?;
        tx.commit().await?;
        Ok(inserted)
    }
}

async fn insert_feed(
    conn: &mut SqliteConnection,
    new: &NewFeed,
    now: DateTime<Utc>,
) -> Result<Feed, DbError> {
    let slug = free_feed_slug(conn, &new.title, None).await?;
    let url = new.url.as_str();
    let site_url = new.site_url.as_ref().map(Url::as_str);
    let now = ts(now);
    sqlx::query_as!(
        FeedRow,
        r#"INSERT INTO feeds (folder_id, position, url, site_url, title, next_fetch_at, slug)
           VALUES (?1, (SELECT COALESCE(MAX(position), -1) + 1 FROM feeds WHERE folder_id IS ?1), ?2, ?3, ?4, ?5, ?6)
           RETURNING id AS "id: FeedId", slug, folder_id AS "folder_id: FolderId", url, site_url, title,
                     custom_title, etag, last_modified, next_fetch_at, error_count AS "error_count: u32", last_error"#,
        new.folder,
        url,
        site_url,
        new.title,
        now,
        slug
    )
    .fetch_one(conn)
    .await?
    .try_into()
}

async fn apply_fetch(
    conn: &mut SqliteConnection,
    id: FeedId,
    record: FetchRecord<'_>,
    now: DateTime<Utc>,
    next_fetch_at: DateTime<Utc>,
) -> Result<u64, DbError> {
    let now = ts(now);
    let next_fetch_at = ts(next_fetch_at);
    let mut inserted = 0;

    let updated = match record {
            FetchRecord::NotModified => {
                sqlx::query!(
                    "UPDATE feeds SET error_count = 0, last_error = NULL, next_fetch_at = ? WHERE id = ?",
                    next_fetch_at,
                    id
                )
                .execute(&mut *conn)
                .await?
            }
            FetchRecord::Postponed => {
            sqlx::query!(
                "UPDATE feeds SET next_fetch_at = ? WHERE id = ?",
                next_fetch_at,
                id
            )
            .execute(&mut *conn)
            .await?
        }
        FetchRecord::Failed { error } => {
                sqlx::query!(
                    "UPDATE feeds SET error_count = error_count + 1, last_error = ?, next_fetch_at = ? WHERE id = ?",
                    error,
                    next_fetch_at,
                    id
                )
                .execute(&mut *conn)
                .await?
            }
            FetchRecord::Updated { feed, validators } => {
                let site_url = feed.site_url.as_ref().map(Url::as_str);
                let updated = sqlx::query!(
                    "UPDATE feeds
                     SET title = ?, site_url = COALESCE(?, site_url), etag = ?, last_modified = ?,
                         error_count = 0, last_error = NULL, next_fetch_at = ?
                     WHERE id = ?",
                    feed.title,
                    site_url,
                    validators.etag,
                    validators.last_modified,
                    next_fetch_at,
                    id
                )
                .execute(&mut *conn)
                .await?;

                // Slugs only need to be unique within the feed; articles keep theirs forever.
                let mut taken: HashSet<String> =
                    sqlx::query_scalar!("SELECT slug FROM items WHERE feed_id = ?", id)
                        .fetch_all(&mut *conn)
                        .await?
                        .into_iter()
                        .collect();

                for item in &feed.items {
                    let slug = unique_slug(&slugify(item.title.as_deref().unwrap_or(""), "article"), |s| {
                        taken.contains(s)
                    });
                    let url = item.url.as_ref().map(Url::as_str);
                    let content = item.content.as_ref().map(|c| c.as_str());
                    let summary = item.summary.as_deref();
                    let published_at = ts(item.published_at);
                    let stored = sqlx::query!(
                        "INSERT INTO items (feed_id, slug, guid, url, title, author, content_html, summary, published_at, fetched_at)
                         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                         ON CONFLICT (feed_id, guid) DO NOTHING",
                        id,
                        slug,
                        item.guid,
                        url,
                        item.title,
                        item.author,
                        content,
                        summary,
                        published_at,
                        now
                    )
                    .execute(&mut *conn)
                    .await?
                    .rows_affected();
                    if stored > 0 {
                        taken.insert(slug);
                    }
                    inserted += stored;
                }
                updated
            }
        };

    found(updated)?;
    Ok(inserted)
}

/// A slug for `title` that no other feed uses (`except` is the feed being renamed).
async fn free_feed_slug(
    conn: &mut SqliteConnection,
    title: &str,
    except: Option<FeedId>,
) -> Result<String, DbError> {
    let base = slugify(title, "feed");
    let taken: HashSet<String> = sqlx::query_scalar!(
        "SELECT slug FROM feeds WHERE (slug = ?1 OR slug LIKE ?1 || '-%') AND id IS NOT ?2",
        base,
        except
    )
    .fetch_all(conn)
    .await?
    .into_iter()
    .collect();
    Ok(unique_slug(&base, |slug| taken.contains(slug)))
}
