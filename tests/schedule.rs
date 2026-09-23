use std::time::Duration;

use chrono::{DateTime, TimeZone, Utc};
use feedr::model::FeedId;
use feedr::schedule::{
    Attempt, MAX_INTERVAL, MIN_INTERVAL, POLL_INTERVAL, adaptive_interval, next_fetch_at,
};

const HOUR: Duration = Duration::from_secs(3600);

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 23, 12, 0, 0).unwrap()
}

fn every(gap: Duration, count: u32) -> Vec<DateTime<Utc>> {
    (0..count).map(|i| now() - gap * i).collect()
}

/// Scheduled delays include up to 5% of per-feed jitter.
fn assert_delay(at: DateTime<Utc>, expected: Duration) {
    let delay = (at - now()).to_std().unwrap();
    assert!(
        delay >= expected && delay <= expected + expected / 20,
        "delay {delay:?} not within 5% above {expected:?}"
    );
}

mod adaptive_interval {
    use super::*;

    #[test]
    fn uses_the_default_without_enough_history() {
        assert_eq!(adaptive_interval(&[], now()), POLL_INTERVAL);
        assert_eq!(adaptive_interval(&[now()], now()), POLL_INTERVAL);
    }

    #[test]
    fn ignores_duplicate_timestamps_from_undated_items() {
        assert_eq!(
            adaptive_interval(&[now(), now(), now()], now()),
            POLL_INTERVAL
        );
    }

    #[test]
    fn polls_four_times_per_typical_gap_between_posts() {
        assert_eq!(adaptive_interval(&every(HOUR * 24, 10), now()), HOUR * 6);
    }

    #[test]
    fn uses_the_median_so_one_burst_does_not_dominate() {
        let mut dates = every(HOUR * 24, 10);
        dates.extend([now() - HOUR * 24 * 3 - HOUR]);

        assert_eq!(adaptive_interval(&dates, now()), HOUR * 6);
    }

    #[test]
    fn accepts_dates_in_any_order() {
        let mut dates = every(HOUR * 24, 10);
        dates.reverse();

        assert_eq!(adaptive_interval(&dates, now()), HOUR * 6);
    }

    #[test]
    fn never_polls_more_often_than_the_minimum() {
        assert_eq!(
            adaptive_interval(&every(Duration::from_secs(300), 10), now()),
            MIN_INTERVAL
        );
    }

    #[test]
    fn slows_down_for_dormant_feeds() {
        let dormant: Vec<_> = every(HOUR, 10)
            .into_iter()
            .map(|d| d - HOUR * 24 * 60)
            .collect();

        assert_eq!(adaptive_interval(&dormant, now()), MAX_INTERVAL);
    }
}

mod next_fetch_at {
    use super::*;

    #[test]
    fn waits_one_interval_after_success() {
        assert_delay(
            next_fetch_at(Attempt::Succeeded, HOUR, FeedId(1), now()),
            HOUR,
        );
    }

    #[test]
    fn backs_off_exponentially_on_consecutive_failures() {
        let failed = |n| Attempt::Failed {
            consecutive_failures: n,
            retry_after: None,
        };

        assert_delay(next_fetch_at(failed(1), HOUR, FeedId(1), now()), HOUR * 2);
        assert_delay(next_fetch_at(failed(3), HOUR, FeedId(1), now()), HOUR * 8);
        assert_delay(
            next_fetch_at(failed(30), HOUR, FeedId(1), now()),
            MAX_INTERVAL,
        );
    }

    #[test]
    fn honors_retry_after_from_the_server() {
        let attempt = Attempt::Failed {
            consecutive_failures: 1,
            retry_after: Some(HOUR * 5),
        };

        assert_delay(next_fetch_at(attempt, HOUR, FeedId(1), now()), HOUR * 5);
    }

    #[test]
    fn spreads_feeds_apart_deterministically() {
        let a = next_fetch_at(Attempt::Succeeded, HOUR, FeedId(1), now());
        let b = next_fetch_at(Attempt::Succeeded, HOUR, FeedId(2), now());

        assert_ne!(a, b);
        assert_eq!(a, next_fetch_at(Attempt::Succeeded, HOUR, FeedId(1), now()));
    }
}
