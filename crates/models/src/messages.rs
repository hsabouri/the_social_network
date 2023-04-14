use std::{fmt::Display, str::FromStr};

use anyhow::Error;
use chrono::{NaiveDateTime, Utc};
use uuid::Uuid;

use crate::users::{UserId, Userlike};

/// UUID and timestamp (milli-seconds precision).
/// Displayed it looks like this :
/// 11234567-1234-5678-1234-567812345678x0000000064371AB8
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct MessageId {
    user_id: UserId,
    timestamp: u64,
}

impl FromStr for MessageId {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = s.as_bytes();

        if bytes.len() == 36 + 1 + 16 && s.is_ascii() && bytes[36] == 'x' as u8 {
            let user_id = UserId::from_str(std::str::from_utf8(&bytes[0..36])?)?;
            let timestamp = u64::from_str_radix(std::str::from_utf8(&bytes[37..63])?, 16)?;

            Ok(Self { user_id, timestamp })
        } else {
            Err(Error::msg("Message ID size is incorrect or is not ASCII"))
        }
    }
}

impl Display for MessageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}x{:#016x}", self.user_id, self.timestamp)
    }
}

impl MessageId {
    pub fn try_parse(s: impl AsRef<str>) -> Result<Self, Error> {
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

#[cfg(feature = "proto")]
impl TryFrom<proto::Message> for Message {
    type Error = anyhow::Error;

    fn try_from(value: proto::Message) -> Result<Self, Self::Error> {
        Ok(Message {
            id: MessageId::try_parse(value.message_id.as_str())?,
            user_id: UserId::try_parse(value.user_id.as_str())?,
            date: NaiveDateTime::from_timestamp_opt(value.timestamp as i64, 0).unwrap(),
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
