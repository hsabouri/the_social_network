use anyhow::Error;
use async_nats::Client;

use super::channels::*;
use super::codec::*;

use crate::messages::Messagelike;
use crate::{
    messages::{Message, MessageRef},
    users::{UserRef, Userlike},
};

pub struct PublishMessage {
    pub message: Message,
}

impl<'a> PublishMessage {
    pub fn new(message: Message) -> Self {
        Self { message }
    }

    pub async fn publish(self, client: Client) -> Result<(), Error> {
        Ok(client
            .publish(CHANNEL_MESSAGE.into(), encode_proto_message(self.message))
            .await?)
    }
}

pub struct PublishSeenMessage {
    pub user: UserRef,
    pub message: MessageRef,
}

impl PublishSeenMessage {
    pub fn new(message: impl Messagelike, user: impl Userlike) -> Self {
        Self {
            user: user.downgrade(),
            message: MessageRef::new(message.get_id()),
        }
    }

    pub async fn publish(self, client: Client) -> Result<(), Error> {
        Ok(client
            .publish(
                CHANNEL_MESSAGE_SEEN.into(),
                encode_proto_message_tag_request(self.user, self.message),
            )
            .await?)
    }
}

pub struct PublishFriendship {
    pub user: UserRef,
    pub friend: UserRef,
}

impl PublishFriendship {
    pub fn new(user: impl Userlike, friend: impl Userlike) -> Self {
        Self {
            user: UserRef::new(user.get_uuid()),
            friend: UserRef::new(friend.get_uuid()),
        }
    }

    pub async fn publish(self, client: Client) -> Result<(), Error> {
        Ok(client
            .publish(
                CHANNEL_NEW_FRIENDSHIP.into(),
                encode_proto_friendship(self.user, self.friend),
            )
            .await?)
    }
}

pub struct PublishRemoveFriendship {
    pub user: UserRef,
    pub friend: UserRef,
}

impl PublishRemoveFriendship {
    pub fn new(user: impl Userlike, friend: impl Userlike) -> Self {
        Self {
            user: UserRef::new(user.get_uuid()),
            friend: UserRef::new(friend.get_uuid()),
        }
    }

    pub async fn publish(self, client: Client) -> Result<(), Error> {
        Ok(client
            .publish(
                CHANNEL_REMOVED_FRIENDSHIP.into(),
                encode_proto_friendship(self.user, self.friend),
            )
            .await?)
    }
}
