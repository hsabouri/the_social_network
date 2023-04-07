use anyhow::Error;
use async_trait::async_trait;
use chrono::NaiveDateTime;
use uuid::Uuid;

use crate::{
    realtime::{PublishMessage, PublishSeenMessage},
    repository::messages::{AddSeenTagRequest, InsertMessageRequest, RemoveSeenTagRequest},
    users::Userlike,
};

#[async_trait]
pub trait Messagelike: Sized {
    fn get_uuid(&self) -> Uuid;

    fn seen_by(&self, user: impl Userlike) -> AddSeenTagRequest {
        AddSeenTagRequest::new(self.get_uuid(), user.get_uuid())
    }

    fn unseen_by(&self, user: impl Userlike) -> RemoveSeenTagRequest {
        RemoveSeenTagRequest::new(self.get_uuid(), user.get_uuid())
    }

    fn realtime_seen_by(self, user: impl Userlike) -> PublishSeenMessage {
        PublishSeenMessage::new(self, user)
    }
}

impl Messagelike for Uuid {
    fn get_uuid(&self) -> Uuid {
        *self
    }
}

/// Stores only the Uuid of the message.
/// Provides methods to easily get the full message infos at the expense of a request to DB.
#[derive(Clone, Copy)]
pub struct MessageRef(pub Uuid);

impl Messagelike for MessageRef {
    fn get_uuid(&self) -> Uuid {
        self.0
    }
}

impl MessageRef {
    pub fn from_str_uuid(message_id: impl AsRef<str>) -> Result<Self, Error> {
        let uuid = Uuid::try_parse(message_id.as_ref())
            .map_err(|e| Error::from(e).context("malformed UUID"))?;

        Ok(Self::new(uuid))
    }

    pub fn new(message_id: Uuid) -> Self {
        Self(message_id)
    }
}

#[derive(Clone, Debug)]
pub struct Message {
    pub id: Uuid,
    pub user_id: Uuid,
    pub date: NaiveDateTime,
    pub content: String,
}

impl Message {
    /// This will generate an UUID, to be broadcasted and inserted in DB at the same time.
    pub fn new(user: impl Userlike, content: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id: user.get_uuid(),
            date: chrono::offset::Local::now().naive_local(),
            content,
        }
    }

    pub fn insert(&self) -> InsertMessageRequest {
        InsertMessageRequest::new(self.user_id, self.content.clone())
            .with_datetime(self.date)
            .with_uuid(self.id)
    }

    pub fn realtime_publish(self) -> PublishMessage {
        PublishMessage::new(self)
    }
}

impl TryFrom<proto::Message> for Message {
    type Error = anyhow::Error;

    fn try_from(value: proto::Message) -> Result<Self, Self::Error> {
        Ok(Message {
            id: Uuid::try_parse(value.message_id.as_str())?,
            user_id: Uuid::try_parse(value.user_id.as_str())?,
            date: NaiveDateTime::from_timestamp_opt(value.timestamp as i64, 0).unwrap(),
            content: value.content,
        })
    }
}

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

impl Messagelike for Message {
    fn get_uuid(&self) -> Uuid {
        self.id
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
