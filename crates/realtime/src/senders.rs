use async_nats::{Client, PublishError};
use thiserror::Error;

use super::channels::*;
use super::codec::*;

use models::{
    messages::{Message, MessageId, Messagelike},
    users::{UserId, Userlike},
};

#[derive(Error, Debug)]
pub enum SenderError {
    #[error("NATS publishing error")]
    Nats(#[from] PublishError),
}

pub struct PublishMessage {
    pub message: Message,
}

impl<'a> PublishMessage {
    pub fn new(message: Message) -> Self {
        Self { message }
    }

    pub async fn publish(self, client: Client) -> Result<(), SenderError> {
        Ok(client
            .publish(CHANNEL_MESSAGE.into(), encode_proto_message(self.message))
            .await?)
    }
}

pub struct PublishSeenMessage {
    pub user: UserId,
    pub message: MessageId,
}

impl PublishSeenMessage {
    pub fn new(message: impl Messagelike, user: impl Userlike) -> Self {
        Self {
            user: user.get_id(),
            message: message.get_id(),
        }
    }

    pub async fn publish(self, client: Client) -> Result<(), SenderError> {
        Ok(client
            .publish(
                CHANNEL_MESSAGE_SEEN.into(),
                encode_proto_message_tag_request(self.user, self.message),
            )
            .await?)
    }
}

pub struct PublishFriendship {
    pub user: UserId,
    pub friend: UserId,
}

impl PublishFriendship {
    pub fn new(user: impl Userlike, friend: impl Userlike) -> Self {
        Self {
            user: user.get_id(),
            friend: friend.get_id(),
        }
    }

    pub async fn publish(self, client: Client) -> Result<(), SenderError> {
        Ok(client
            .publish(
                CHANNEL_NEW_FRIENDSHIP.into(),
                encode_proto_friendship(self.user, self.friend),
            )
            .await?)
    }
}

pub struct PublishRemoveFriendship {
    pub user: UserId,
    pub friend: UserId,
}

impl PublishRemoveFriendship {
    pub fn new(user: impl Userlike, friend: impl Userlike) -> Self {
        Self {
            user: user.get_id(),
            friend: friend.get_id(),
        }
    }

    pub async fn publish(self, client: Client) -> Result<(), SenderError> {
        Ok(client
            .publish(
                CHANNEL_REMOVED_FRIENDSHIP.into(),
                encode_proto_friendship(self.user, self.friend),
            )
            .await?)
    }
}
