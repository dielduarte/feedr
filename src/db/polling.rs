use chrono::{DateTime, Utc};
use url::Url;

use super::{Db, DbError, found, from_ts, ts};
use crate::model::{FeedId, FeedScope};

impl Db {
    pub async fn recent_publish_times(
        &self,
        feed: FeedId,
        limit: u32,
    ) -> Result<Vec<DateTime<Utc>>, DbError> {
        let times = sqlx::query_scalar!(
            "SELECT published_at FROM items WHERE feed_id = ? ORDER BY published_at DESC LIMIT ?",
            feed,
            limit
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(times.into_iter().map(from_ts).collect())
    }

    /// Returns how many feeds were scheduled.
    pub async fn mark_due(&self, scope: FeedScope, now: DateTime<Utc>) -> Result<u64, DbError> {
        let now = ts(now);
        let result = match scope {
            FeedScope::All => {
                sqlx::query!("UPDATE feeds SET next_fetch_at = ?", now)
                    .execute(&self.pool)
                    .await?
            }
            FeedScope::Folder(folder) => {
                sqlx::query!(
                    "UPDATE feeds SET next_fetch_at = ? WHERE folder_id = ?",
                    now,
                    folder
                )
                .execute(&self.pool)
                .await?
            }
            FeedScope::Feed(feed) => {
                sqlx::query!("UPDATE feeds SET next_fetch_at = ? WHERE id = ?", now, feed)
                    .execute(&self.pool)
                    .await?
            }
        };
        Ok(result.rows_affected())
    }

    pub async fn update_feed_url(&self, feed: FeedId, url: &Url) -> Result<(), DbError> {
        let url = url.as_str();
        found(
            sqlx::query!("UPDATE feeds SET url = ? WHERE id = ?", url, feed)
                .execute(&self.pool)
                .await?,
        )
    }

    pub async fn next_due_at(&self) -> Result<Option<DateTime<Utc>>, DbError> {
        let next = sqlx::query_scalar!("SELECT MIN(next_fetch_at) FROM feeds")
            .fetch_one(&self.pool)
            .await?;
        Ok(next.map(from_ts))
    }
}
