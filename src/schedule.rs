use std::time::Duration;

use chrono::{DateTime, Utc};

use crate::model::FeedId;

/// Used when a feed has too little history to infer how often it publishes.
pub const POLL_INTERVAL: Duration = Duration::from_secs(30 * 60);
pub const MIN_INTERVAL: Duration = Duration::from_secs(15 * 60);
pub const MAX_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// How many recent posts inform the interval.
pub const HISTORY: u32 = 20;
const CHECKS_PER_GAP: u32 = 4;

pub enum Attempt {
    Succeeded,
    Failed {
        consecutive_failures: u32,
        retry_after: Option<Duration>,
    },
}

/// Checks a few times per typical gap between recent posts. A feed that has been quiet for
/// longer than its usual gap is treated as slowing down, so dormant feeds drift to daily.
pub fn adaptive_interval(published: &[DateTime<Utc>], now: DateTime<Utc>) -> Duration {
    let mut dates = published.to_vec();
    dates.sort_unstable_by(|a, b| b.cmp(a));
    dates.dedup();
    dates.truncate(HISTORY as usize);

    let mut gaps: Vec<Duration> = dates
        .windows(2)
        .filter_map(|pair| (pair[0] - pair[1]).to_std().ok())
        .collect();
    if gaps.is_empty() {
        return POLL_INTERVAL;
    }
    gaps.sort_unstable();
    let median = gaps[gaps.len() / 2];
    let since_newest = (now - dates[0]).to_std().unwrap_or_default();

    (median.max(since_newest) / CHECKS_PER_GAP).clamp(MIN_INTERVAL, MAX_INTERVAL)
}

pub fn next_fetch_at(
    attempt: Attempt,
    interval: Duration,
    feed: FeedId,
    now: DateTime<Utc>,
) -> DateTime<Utc> {
    let delay = match attempt {
        Attempt::Succeeded => interval,
        Attempt::Failed {
            retry_after: Some(retry_after),
            ..
        } => retry_after,
        Attempt::Failed {
            consecutive_failures,
            ..
        } => interval.saturating_mul(2u32.saturating_pow(consecutive_failures)),
    }
    .min(MAX_INTERVAL);

    now + delay + jitter(delay, feed)
}

/// Up to 5% extra delay, stable per feed, so feeds added together don't stay in lockstep.
fn jitter(delay: Duration, feed: FeedId) -> Duration {
    let spread = (feed.0 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 54;
    delay.mul_f64((spread % 1000) as f64 / 20_000.0)
}
