use anyhow::Error;
use chrono::{Duration, NaiveDate, NaiveDateTime};
use futures::{FutureExt, Stream, StreamExt};
use scylla::frame::value::{Time, Timestamp};
use scylla::Session;
use uuid::Uuid;

use crate::messages::{Message, MessageId};

use super::{timestamp_to_naive, TimeBucket};

/// FIXME: Timestamp and time_bucket are calculated by requester and not by DB.
/// It should be calculated in DB using a **User Defined Function** in Lua.
///
/// Message UUID is generated here and not by DB because it cannot be easily returned.
#[derive(Clone, Debug)]
pub struct InsertMessageRequest {
    pub message_id: Option<MessageId>,
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
            message_id: None,
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

    pub fn with_id(self, message_id: MessageId) -> Self {
        Self {
            message_id: Some(message_id),
            ..self
        }
    }

    pub async fn execute(self, session: &Session) -> Result<MessageId, Error> {
        let datetime = self
            .datetime
            .unwrap_or_else(|| chrono::offset::Local::now().naive_local());
        let message_id = self
            .message_id
            .unwrap_or_else(|| MessageId::new_now(self.user_id));
        let (timestamp, bucket_timestamp) = Self::get_timestamps(datetime);

        session
            .query("INSERT INTO messages (message_id, user_id, date_bucket, date, content) VALUES (?, ?, ?, ?, ?)", (
                message_id.as_tuple_i64(),
                self.user_id,
                bucket_timestamp,
                timestamp,
                self.content
            ))
            .await?;

        Ok(message_id)
    }
}

/// Scrolls through time buckets and returns the messages.
#[derive(Clone, Copy, Debug)]
pub struct GetLastMessagesOfUserRequest {
    pub user_id: Uuid,
    pub starting_from: Option<TimeBucket>,
    pub ends_at: Option<TimeBucket>,
}

impl GetLastMessagesOfUserRequest {
    pub fn new(user_id: Uuid) -> Self {
        Self {
            user_id,
            starting_from: None,
            ends_at: None,
        }
    }

    pub fn ends_from(self, time_bucket: TimeBucket) -> Self {
        Self {
            ends_at: Some(time_bucket),
            ..self
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
        let time_bucket_iter = self
            .starting_from
            .unwrap_or_else(|| TimeBucket::current())
            .iter_past_to(self.ends_at.unwrap_or_default());

        let time_bucket_stream = futures::stream::iter(time_bucket_iter);

        let bucketted_result = time_bucket_stream.map(move |bucket| {
            session.query(
                r#"SELECT message_id, date, content FROM messages
                        WHERE   user_id = ?
                            AND date_bucket = ?"#,
                (self.user_id, bucket.get_timestamp()),
            )
        });

        let stream = bucketted_result
            .map(move |res| {
                res.into_stream().map(move |res| match res {
                    Ok(res) => {
                        let messages: Vec<Result<Message, Error>> = res
                            .rows_or_empty()
                            .into_iter()
                            .map(|row| {
                                let (message_id, date, content): ((Uuid, i64), Timestamp, String) =
                                    row.into_typed()?;

                                Result::Ok(Message {
                                    id: MessageId::from_tuple_i64(message_id),
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

        stream
    }
}

#[derive(Clone, Copy, Debug)]
pub struct AddSeenTagRequest {
    pub user_id: Uuid,
    pub message_id: MessageId,
}

impl AddSeenTagRequest {
    pub fn new(message_id: MessageId, user_id: Uuid) -> Self {
        Self {
            user_id,
            message_id,
        }
    }

    pub async fn execute(self, session: &Session) -> Result<(), Error> {
        let _ = session
            .query(
                r#"INSERT INTO read_tags (user_id, message_id)
                VALUES (?, ?)"#,
                (self.user_id, self.message_id.as_tuple_i64()),
            )
            .await?;

        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RemoveSeenTagRequest {
    pub user_id: Uuid,
    pub message_id: MessageId,
}

impl RemoveSeenTagRequest {
    pub fn new(message_id: MessageId, user_id: Uuid) -> Self {
        Self {
            user_id,
            message_id,
        }
    }

    pub async fn execute(self, session: &Session) -> Result<(), Error> {
        let _ = session
            .query(
                r#"DELETE FROM read_tags WHERE user_id = ? AND message_id = ?"#,
                (self.user_id, self.message_id.as_tuple_i64()),
            )
            .await?;

        Ok(())
    }
}
