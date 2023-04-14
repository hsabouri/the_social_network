//! From/Into proto::Message;

use crate::messages::{Message, MessageId, MessageIdParsingError};
use crate::users::{UserId, UserIdParsingError};
use chrono::NaiveDateTime;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProtoDecodeMessageError {
    #[error("invalid MessageId")]
    MessageId(#[from] MessageIdParsingError),
    #[error("invalid UserId")]
    UserId(#[from] UserIdParsingError),
    #[error("invalid timestamp")]
    Timestamp(u64),
}

impl TryFrom<proto::Message> for Message {
    type Error = ProtoDecodeMessageError;

    fn try_from(value: proto::Message) -> Result<Self, Self::Error> {
        Ok(Message {
            id: MessageId::try_parse(value.message_id.as_str())?,
            user_id: UserId::try_parse(value.user_id.as_str())?,
            date: NaiveDateTime::from_timestamp_opt(value.timestamp as i64, 0)
                .ok_or_else(|| ProtoDecodeMessageError::Timestamp(value.timestamp))?,
            content: value.content,
        })
    }
}

#[cfg(feature = "proto")]
impl Into<proto::Message> for Message {
    fn into(self) -> proto::Message {
        proto::Message {
            message_id: self.id.to_string(),
            user_id: self.user_id.to_string(),
            timestamp: self.date.timestamp() as u64,
            content: self.content.clone(),
            read: false,
        }
    }
}
