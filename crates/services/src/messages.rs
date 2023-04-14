use std::ops::Deref;

use models::users::Userlike;
use models::messages::{Messagelike, MessageId, Message};
use realtime::senders::{PublishMessage, PublishSeenMessage};
use repository::messages::{AddSeenTagRequest, InsertMessageRequest, RemoveSeenTagRequest};

pub trait MessagelikeServices: Messagelike {
    fn seen_by(&self, user: impl Userlike) -> AddSeenTagRequest {
        AddSeenTagRequest::new(self.get_id(), user.get_id())
    }

    fn unseen_by(&self, user: impl Userlike) -> RemoveSeenTagRequest {
        RemoveSeenTagRequest::new(self.get_id(), user.get_id())
    }

    fn realtime_seen_by(self, user: impl Userlike) -> PublishSeenMessage {
        PublishSeenMessage::new(self, user)
    }
}

impl<T: Messagelike> MessagelikeServices for T {}

#[derive(Clone)]
pub struct MessageServices(Message);

impl Messagelike for MessageServices {
    fn get_id(&self) -> MessageId {
        self.id
    }
}

impl Deref for MessageServices {
    type Target = Message;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl MessageServices {
    pub fn new(message: Message) -> Self {
        Self (message)
    }

    pub fn insert(&self) -> InsertMessageRequest {
        InsertMessageRequest::new(self.user_id, self.content.clone())
            .with_datetime(self.date)
            .with_id(self.id)
    }

    pub fn realtime_publish(self) -> PublishMessage {
        PublishMessage::new(self.0)
    }
}