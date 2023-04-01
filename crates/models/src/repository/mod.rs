use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime};
use futures::Stream;
use scylla::frame::value::{Time, Timestamp};

pub mod messages;
pub mod users;

/// A bucket is used to group rows in ScyllaDB. We group them by week starting on monday 00:00.
/// This is not generic yet.
#[derive(Clone, Copy, Debug)]
pub struct TimeBucket(NaiveDate);

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

    pub fn iter_past(self) -> TimebucketIterator {
        TimebucketIterator::starting_from(self)
    }
}

pub struct TimebucketIterator {
    current_timebucket: TimeBucket,
}

impl TimebucketIterator {
    pub fn starting_from(bucket: TimeBucket) -> Self {
        Self {
            current_timebucket: bucket,
        }
    }

    pub fn into_stream(self) -> impl Stream<Item = TimeBucket> {
        futures::stream::iter(self)
    }
}

impl Iterator for TimebucketIterator {
    type Item = TimeBucket;

    fn next(&mut self) -> Option<Self::Item> {
        let tb = self.current_timebucket;

        self.current_timebucket = tb.previous();

        Some(tb)
    }
}

fn timestamp_to_naive(ts: Timestamp) -> NaiveDateTime {
    NaiveDateTime::from_timestamp_millis(ts.0.num_seconds()).unwrap()
}
