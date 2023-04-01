use anyhow::Error;
use chrono::{Duration, NaiveDateTime};
use futures::{FutureExt, Stream, StreamExt};
use scylla::frame::value::Timestamp;
use scylla::{QueryResult, Session};
use uuid::Uuid;

use crate::messages::Message;

use super::{timestamp_to_naive, TimeBucket};

/// FIXME: Timestamp and time_bucket are calculated by requester and not by DB.
/// It should be calculated in DB using a **User Defined Function** in Lua.
///
/// Message UUID is generated here and not by DB because it cannot be easily returned.
#[derive(Clone, Debug)]
pub struct InsertMessageRequest {
    pub message_id: Uuid,
    pub user_id: Uuid,
    pub content: String,
    pub datetime: Option<NaiveDateTime>,
}

impl InsertMessageRequest {
    fn get_timestamps(datetime: NaiveDateTime) -> (Timestamp, Timestamp) {
        let timestamp = datetime.timestamp();

        (
            Timestamp(Duration::seconds(timestamp)),
            TimeBucket::from_datetime(datetime).get_timestamp(),
        )
    }

    /// Fixes the timestamp bucket
    pub fn new(user_id: Uuid, content: String) -> Self {
        Self {
            message_id: Uuid::new_v4(),
            user_id,
            content,
            datetime: None,
        }
    }

    pub fn with_datetime(self, datetime: NaiveDateTime) -> Self {
        Self {
            datetime: Some(datetime),
            ..self
        }
    }

    pub async fn execute(self, session: &Session) -> Result<Uuid, Error> {
        let datetime = self
            .datetime
            .unwrap_or_else(|| chrono::offset::Local::now().naive_local());
        let (timestamp, bucket_timestamp) = Self::get_timestamps(datetime);

        session
            .query("INSERT INTO messages (message_id, user_id, date_bucket, date, content) VALUES (?, ?, ?, ?, ?)", (
                self.message_id,
                self.user_id,
                bucket_timestamp,
                timestamp,
                self.content
            ))
            .await?;

        Ok(self.message_id)
    }
}

/// Scrolls through time buckets and returns the messages.
#[derive(Clone, Copy, Debug)]
pub struct GetLastMessagesOfUserRequest {
    pub user_id: Uuid,
    pub starting_from: Option<TimeBucket>,
}

impl GetLastMessagesOfUserRequest {
    pub fn new(user_id: Uuid) -> Self {
        Self {
            user_id,
            starting_from: None,
        }
    }

    pub fn starting_from(self, time_bucket: TimeBucket) -> Self {
        Self {
            starting_from: Some(time_bucket),
            ..self
        }
    }

    pub fn stream<'a>(
        self,
        session: &'a Session,
    ) -> impl Stream<Item = Result<Message, Error>> + 'a {
        let user_id = self.user_id;
        let time_bucket_stream = self
            .starting_from
            .unwrap_or_else(|| TimeBucket::current())
            .iter_past()
            .into_stream();

        let bucketted_result = time_bucket_stream.map(move |bucket| {
            session.query(
                r#"SELECT message_id, date, content FROM messages
                        WHERE   user_id = ?
                            AND date_bucket = ?"#,
                (self.user_id, bucket.get_timestamp()),
            )
        });

        let s = bucketted_result
            .map(move |res| {
                res.into_stream().map(move |res| match res {
                    Ok(res) => {
                        let messages: Vec<Result<Message, Error>> = res
                            .rows_or_empty()
                            .into_iter()
                            .map(|row| {
                                let (message_id, date, content): (Uuid, Timestamp, String) =
                                    row.into_typed()?;

                                Result::Ok(Message {
                                    id: message_id,
                                    date: timestamp_to_naive(date),
                                    content,
                                    user_id,
                                })
                            })
                            .collect();

                        futures::stream::iter(messages)
                    }
                    Err(e) => futures::stream::iter(vec![Err(Error::from(e))]),
                })
            })
            .flatten()
            .flatten();

        s
    }
}