use chrono::NaiveDateTime;
use scylla::Session;
use scylla::frame::value::{Timestamp, Time};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct NewMessageRequest {
    pub user_id: Uuid,
    pub content: String,
    pub timestamp: Option<Timestamp>,
}

impl NewMessageRequest {
    /// Fixes the timestamp bucket
    pub fn new(user_id: Uuid, content: String) -> Self {
        Self {
            user_id,
            content,
            timestamp: None
        }
    }

    pub fn with_datetime(self, datetime: NaiveDateTime) -> Self {
        let timestamp = Timestamp (chrono::Duration::seconds(datetime.timestamp()));
        Self {
            timestamp: Some(timestamp),
            ..self
        }
    }
}