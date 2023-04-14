use std::{fmt::Display, str::FromStr};

use chrono::{NaiveDateTime, Utc};
use thiserror::Error;
use uuid::Uuid;

use crate::users::{UserId, UserIdParsingError, Userlike};

/// UUID and timestamp (milli-seconds precision).
/// Displayed it looks like this (53 bytes):
/// 11234567-1234-5678-1234-567812345678x0000000064371AB8
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct MessageId {
    user_id: UserId,
    timestamp: u64,
}

#[derive(Error, Debug)]
pub enum MessageIdParsingError {
    #[error("wrong size of str `{0}`, expected `53` bytes long str")]
    Size(usize),
    #[error("wrong format of str, got `{0}`, expected `x` at byte 36")]
    Format(char),
    #[error("wrong encoding of str")]
    Utf8Error(#[from] std::str::Utf8Error),
    #[error("wrong format or value for timestamp")]
    Timestamp(#[from] std::num::ParseIntError),
    #[error("error parsing user id")]
    UserId(#[from] UserIdParsingError),
}

impl FromStr for MessageId {
    type Err = MessageIdParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = s.as_bytes();
        let size = bytes.len();

        if size != 36 + 1 + 16 {
            return Err(MessageIdParsingError::Size(size));
        }

        let sep = bytes[36] as char;
        if sep != 'x' {
            return Err(MessageIdParsingError::Format(sep));
        }

        // Actually safe because is comes from a str.
        let user_id_str = std::str::from_utf8(&bytes[0..36])?;
        let timestamp_str = std::str::from_utf8(&bytes[37..63])?;
        let user_id = UserId::from_str(user_id_str)?;
        let timestamp = u64::from_str_radix(timestamp_str, 16)?;

        Ok(Self { user_id, timestamp })
    }
}

impl Display for MessageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}x{:#016x}", self.user_id, self.timestamp)
    }
}

impl MessageId {
    pub fn try_parse(s: impl AsRef<str>) -> Result<Self, MessageIdParsingError> {
        s.as_ref().parse()
    }

    pub fn new_now(user_id: UserId) -> Self {
        let datetime = Utc::now().naive_local();
        let timestamp = datetime.timestamp_millis() as u64;

        Self { user_id, timestamp }
    }

    pub fn from_tuple((user_id, timestamp): (Uuid, u64)) -> Self {
        Self {
            user_id: user_id.into(),
            timestamp,
        }
    }

    pub fn from_tuple_i64((user_id, timestamp): (Uuid, i64)) -> Self {
        Self {
            user_id: user_id.into(),
            timestamp: timestamp as u64,
        }
    }

    pub fn as_tuple(self) -> (Uuid, u64) {
        (self.user_id.into(), self.timestamp)
    }

    pub fn as_tuple_i64(self) -> (Uuid, i64) {
        (self.user_id.into(), self.timestamp as i64)
    }
}

pub trait Messagelike: Sized {
    fn get_id(&self) -> MessageId;
}

impl Messagelike for MessageId {
    fn get_id(&self) -> MessageId {
        *self
    }
}

#[derive(Clone, Debug)]
pub struct Message {
    pub id: MessageId,
    pub user_id: UserId,
    pub date: NaiveDateTime,
    pub content: String,
}

impl Message {
    pub fn new(user: impl Userlike, content: String) -> Self {
        Self {
            id: MessageId::new_now(user.get_id()),
            user_id: user.get_id(),
            date: chrono::offset::Local::now().naive_local(),
            content,
        }
    }
}

impl PartialEq for Message {
    fn eq(&self, other: &Self) -> bool {
        self.date == other.date
    }
}

impl Eq for Message {}

impl PartialOrd for Message {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.date.partial_cmp(&other.date)
    }
}

impl Ord for Message {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}
