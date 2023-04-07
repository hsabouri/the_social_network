use std::{collections::HashSet, pin::Pin, task::Poll};

use anyhow::Error;
use async_nats::Client;
use futures::{stream::StreamExt, Stream};
use uuid::Uuid;

use super::channels::*;
use super::codec::*;

use crate::{
    messages::{Message, MessageRef},
    users::{UserRef, Userlike},
};

/// Stream of all new messages from all users.
pub struct NewMessages {
    inner: Pin<Box<dyn Stream<Item = Result<Message, Error>> + Send>>,
}

impl NewMessages {
    pub async fn new(client: &Client) -> Result<Self, Error> {
        let subscription = client.subscribe(CHANNEL_MESSAGE.into()).await?;

        let inner = subscription.map(|proto_message| decode_proto_message(proto_message.payload));

        Ok(Self {
            inner: Box::pin(inner),
        })
    }
}

impl Stream for NewMessages {
    type Item = Result<Message, Error>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        self.inner.poll_next_unpin(cx)
    }
}

/// Stream of all new friendships for all users.
pub struct NewFriendships {
    inner: Pin<Box<dyn Stream<Item = Result<(UserRef, UserRef), Error>> + Send>>,
}

impl NewFriendships {
    pub async fn new(client: &Client) -> Result<Self, Error> {
        let subscription = client.subscribe(CHANNEL_FRIENDSHIP.into()).await?;

        let inner =
            subscription.map(|proto_message| decode_proto_friendship(proto_message.payload));

        Ok(Self {
            inner: Box::pin(inner),
        })
    }
}

impl Stream for NewFriendships {
    type Item = Result<(UserRef, UserRef), Error>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        self.inner.poll_next_unpin(cx)
    }
}

/// Stream of all new friendships for all users.
pub struct SeenMessages {
    inner: Pin<Box<dyn Stream<Item = Result<(UserRef, MessageRef), Error>> + Send>>,
}

impl SeenMessages {
    pub async fn new(client: &Client) -> Result<Self, Error> {
        let subscription = client.subscribe(CHANNEL_MESSAGE_SEEN.into()).await?;

        let inner = subscription
            .map(|proto_message| decode_proto_message_tag_request(proto_message.payload));

        Ok(Self {
            inner: Box::pin(inner),
        })
    }
}

impl Stream for SeenMessages {
    type Item = Result<(UserRef, MessageRef), Error>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        self.inner.poll_next_unpin(cx)
    }
}

/// Stream of all new friendships for all users.
pub struct UnseenMessages {
    inner: Pin<Box<dyn Stream<Item = Result<(UserRef, MessageRef), Error>>>>,
}

impl UnseenMessages {
    pub async fn new(client: &Client) -> Result<Self, Error> {
        let subscription = client.subscribe(CHANNEL_MESSAGE_UNSEEN.into()).await?;

        let inner = subscription
            .map(|proto_message| decode_proto_message_tag_request(proto_message.payload));

        Ok(Self {
            inner: Box::pin(inner),
        })
    }
}

impl Stream for UnseenMessages {
    type Item = Result<(UserRef, MessageRef), Error>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        self.inner.poll_next_unpin(cx)
    }
}

/// Takes a stream `T` of `I::Userlike` and outputs a stream of the newly posted messages from these users.
///
/// If stream `T` is closed/finished, output stream will continue with newly posted message of all users returned by stream
/// `T` before it closed.
///
/// Demonstrates a dynamic filter stream.
pub struct UsersNewMessages<T, I>
where
    T: Stream<Item = I> + Unpin + Send,
    I: Userlike,
{
    users_stream: Option<T>,
    subscription: NewMessages,
    users: HashSet<Uuid>,
}

impl<T, I> UsersNewMessages<T, I>
where
    T: Stream<Item = I> + Unpin + Send,
    I: Userlike,
{
    pub async fn new(users: T, client: &Client) -> Result<Self, Error> {
        Ok(Self {
            users_stream: Some(users),
            subscription: NewMessages::new(client).await?,
            users: HashSet::new(),
        })
    }
}

impl<T, I> Stream for UsersNewMessages<T, I>
where
    T: Stream<Item = I> + Unpin + Send,
    I: Userlike,
{
    type Item = Result<Message, Error>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        // TODO: Put this part in a `stream-helper`
        let keep_stream = if let Some(users_stream) = self.users_stream.as_mut() {
            // Get potential new user from stream
            match users_stream.poll_next_unpin(cx) {
                Poll::Ready(output) => match output {
                    Some(new_user) => {
                        self.users.insert(new_user.get_uuid());
                        true
                    }
                    None => false,
                },
                Poll::Pending => true,
            }
        } else {
            false
        };

        // If user stream is finished, forget it.
        if !keep_stream {
            self.users_stream = None;
        }

        // Get potential new message from subscribtion
        match self.subscription.poll_next_unpin(cx) {
            Poll::Ready(output) => match output {
                Some(message) => {
                    match message {
                        Ok(message) => {
                            // Filtering with users in the list
                            if self.users.contains(&message.user_id) {
                                Poll::Ready(Some(Ok(message)))
                            } else {
                                Poll::Pending
                            }
                        }
                        err => Poll::Ready(Some(err)),
                    }
                }
                None => Poll::Ready(None),
            },
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Takes a user `Uuid` and outputs a stream of the new friendships for this user.
/// Demonstrates a static filter stream.
pub struct UserNewFriendships {
    pub user_id: Uuid,
    inner: Pin<Box<dyn Stream<Item = Result<UserRef, Error>> + Send>>,
}

impl UserNewFriendships {
    pub async fn new(user: impl Userlike, client: &Client) -> Result<Self, Error> {
        let friendships = NewFriendships::new(client).await?;
        let user_id = user.get_uuid();

        let inner = friendships.filter_map(move |friendship| async move {
            match friendship {
                Ok((user, friend)) if user.get_uuid() == user_id => Some(Ok(friend)),
                Ok(_) => None,
                Err(e) => Some(Err(e)),
            }
        });

        Ok(Self {
            user_id,
            inner: Box::pin(inner),
        })
    }
}

impl Stream for UserNewFriendships {
    type Item = Result<UserRef, Error>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        self.inner.poll_next_unpin(cx)
    }
}

/// Takes a stream `T` of `I::Userlike` and outputs a stream of the newly seen messages from these users.
///
/// If stream `T` is closed/finished, output stream will continue with newly seen message of all users returned by stream
/// `T` before it closed.
pub struct UsersSeenMessages<T, I>
where
    T: Stream<Item = I> + Unpin,
    I: Userlike,
{
    users_stream: Option<T>,
    subscription: SeenMessages,
    users: HashSet<Uuid>,
}

impl<T, I> UsersSeenMessages<T, I>
where
    T: Stream<Item = I> + Unpin,
    I: Userlike,
{
    pub async fn new(users: T, client: &Client) -> Result<Self, Error> {
        Ok(Self {
            users_stream: Some(users),
            subscription: SeenMessages::new(client).await?,
            users: HashSet::new(),
        })
    }
}

impl<T, I> Stream for UsersSeenMessages<T, I>
where
    T: Stream<Item = I> + Unpin,
    I: Userlike,
{
    type Item = Result<(UserRef, MessageRef), Error>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let keep_stream = if let Some(users_stream) = self.users_stream.as_mut() {
            // Get potential new user from stream
            match users_stream.poll_next_unpin(cx) {
                Poll::Ready(output) => match output {
                    Some(new_user) => {
                        self.users.insert(new_user.get_uuid());
                        true
                    }
                    None => false,
                },
                Poll::Pending => true,
            }
        } else {
            false
        };

        // If user stream is finished, forget it.
        if !keep_stream {
            self.users_stream = None;
        }

        // Get potential new message from subscribtion
        match self.subscription.poll_next_unpin(cx) {
            Poll::Ready(output) => match output {
                Some(message) => {
                    match message {
                        Ok((user, message)) => {
                            // Filtering with users in the list
                            if self.users.contains(&user.get_uuid()) {
                                Poll::Ready(Some(Ok((user, message))))
                            } else {
                                Poll::Pending
                            }
                        }
                        err => Poll::Ready(Some(err)),
                    }
                }
                None => Poll::Ready(None),
            },
            Poll::Pending => Poll::Pending,
        }
    }
}
