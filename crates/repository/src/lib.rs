use std::{iter::from_fn, ops::Deref};

use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime};
use scylla::frame::value::Timestamp;

pub mod messages;
pub mod users;

// Re-exports
pub use scylla::Session;
pub use sqlx::PgPool;

/// A bucket is used to group rows in ScyllaDB. We group them by week starting on monday 00:00.
/// This is not generic yet.
#[derive(Clone, Copy, Debug)]
pub struct TimeBucket(NaiveDate);

impl Deref for TimeBucket {
    type Target = NaiveDate;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for TimeBucket {
    fn default() -> Self {
        Self(NaiveDate::from_ymd_opt(2023, 01, 02).unwrap())
    }
}

impl TimeBucket {
    pub fn current() -> Self {
        Self::from_date(chrono::offset::Local::now().date_naive())
    }

    pub fn from_datetime(datetime: NaiveDateTime) -> Self {
        Self::from_date(datetime.date())
    }

    pub fn from_date(datetime: NaiveDate) -> Self {
        let bucket = datetime - Duration::days(datetime.weekday() as i64);

        Self(bucket)
    }

    pub fn get_timestamp(self) -> Timestamp {
        Timestamp(Duration::seconds(
            self.0.and_time(NaiveTime::default()).timestamp(),
        ))
    }

    pub fn previous(self) -> Self {
        Self(self.0 - Duration::days(7))
    }

    pub fn date(self) -> NaiveDate {
        self.0
    }

    pub fn datetime(self) -> NaiveDateTime {
        self.0.and_time(NaiveTime::default())
    }

    pub fn iter_past_to(mut self, end: TimeBucket) -> impl Iterator<Item = TimeBucket> {
        from_fn(move || {
            if self.0 > end.0 {
                let ret = self;
                self = self.previous();

                Some(ret)
            } else {
                None
            }
        })
    }

    pub fn iter_forward_to(mut self, end: TimeBucket) -> impl Iterator<Item = TimeBucket> {
        from_fn(move || {
            if self.0 < end.0 {
                let ret = self;
                self = self.previous();

                Some(ret)
            } else {
                None
            }
        })
    }
}

fn timestamp_to_naive(ts: Timestamp) -> NaiveDateTime {
    NaiveDateTime::from_timestamp_millis(ts.0.num_seconds()).unwrap()
}
