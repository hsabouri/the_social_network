use std::collections::HashSet;

use async_nats::{Client, Error as NatsError};
use futures::future::Either;
use futures::stream::select;
use futures::{FutureExt, Stream, TryFutureExt};
use futures::{StreamExt, TryStreamExt};
use models::friendships::FriendshipUpdate;
use thiserror::Error;

use super::channels::*;
use super::codec::*;

use models::{
    messages::{Message, MessageId},
    users::{UserId, Userlike},
};

#[derive(Error, Debug)]
pub enum ReceiverError {
    #[error("decoding failed")]
    Decoding(#[from] ProtoDecodingError),
    #[error("NATS receiver error")]
    Nats(#[from] NatsError),
}

#[derive(Error, Debug)]
#[error(transparent)]
pub struct InputError<E: std::error::Error + Send + Sync>(#[from] E);

#[derive(Error, Debug)]
pub enum ParReceiverError<E: std::error::Error + Send + Sync> {
    #[error("decoding failed")]
    Decoding(#[from] ProtoDecodingError),
    #[error("NATS connection error")]
    Nats(#[from] NatsError),
    #[error("Error in input value or stream")]
    Input(#[from] InputError<E>),
}

impl<E: std::error::Error + Send + Sync> From<ReceiverError> for ParReceiverError<E> {
    fn from(value: ReceiverError) -> Self {
        match value {
            ReceiverError::Decoding(e) => ParReceiverError::Decoding(e),
            ReceiverError::Nats(e) => ParReceiverError::Nats(e),
        }
    }
}

async fn inner_new_messages(
    client: Client,
) -> Result<impl Stream<Item = Result<Message, ProtoDecodingError>>, NatsError> {
    let subscription = client.subscribe(CHANNEL_MESSAGE.into()).await?;

    let stream = subscription.map(|proto_message| decode_proto_message(proto_message.payload));

    Ok(stream)
}

/// Stream of all new messages from all users. Connected to NATS.
pub fn new_messages<'a>(client: Client) -> impl Stream<Item = Result<Message, ReceiverError>> + 'a {
    inner_new_messages(client)
        .map_err(|e| ReceiverError::Nats(e))
        .map_ok(|stream| stream.map_err(|e| ReceiverError::Decoding(e)))
        .into_stream()
        .try_flatten()
}

async fn inner_new_friendships(
    client: Client,
) -> Result<impl Stream<Item = Result<(UserId, UserId), ProtoDecodingError>>, NatsError> {
    let subscription = client.subscribe(CHANNEL_NEW_FRIENDSHIP.into()).await?;

    let stream = subscription.map(|proto_message| decode_proto_friendship(proto_message.payload));

    Ok(stream)
}

/// Stream of all new friendships of all users. Connected to NATS.
pub fn new_friendships<'a>(
    client: Client,
) -> impl Stream<Item = Result<(UserId, UserId), ReceiverError>> + 'a {
    inner_new_friendships(client)
        .map_err(|e| ReceiverError::Nats(e))
        .map_ok(|stream| stream.map_err(|e| ReceiverError::Decoding(e)))
        .into_stream()
        .try_flatten()
}

async fn inner_removed_friendships(
    client: Client,
) -> Result<impl Stream<Item = Result<(UserId, UserId), ProtoDecodingError>>, NatsError> {
    let subscription = client.subscribe(CHANNEL_REMOVED_FRIENDSHIP.into()).await?;

    let stream = subscription.map(|proto_message| decode_proto_friendship(proto_message.payload));

    Ok(stream)
}

/// Stream of all new friendships of all users. Connected to NATS.
pub fn removed_friendships<'a>(
    client: Client,
) -> impl Stream<Item = Result<(UserId, UserId), ReceiverError>> + 'a {
    inner_removed_friendships(client)
        .map_err(|e| ReceiverError::Nats(e))
        .map_ok(|stream| stream.map_err(|e| ReceiverError::Decoding(e)))
        .into_stream()
        .try_flatten()
}

async fn inner_seen_messages(
    client: Client,
) -> Result<impl Stream<Item = Result<(UserId, MessageId), ProtoDecodingError>>, NatsError> {
    let subscription = client.subscribe(CHANNEL_MESSAGE_SEEN.into()).await?;

    let stream =
        subscription.map(|proto_message| decode_proto_message_tag_request(proto_message.payload));

    Ok(stream)
}

/// Stream of all seen notification for all messages from all users. Connected to NATS.
pub fn seen_messages<'a>(
    client: Client,
) -> impl Stream<Item = Result<(UserId, MessageId), ReceiverError>> + 'a {
    inner_seen_messages(client)
        .map_err(|e| ReceiverError::Nats(e))
        .map_ok(|stream| stream.map_err(|e| ReceiverError::Decoding(e)))
        .into_stream()
        .try_flatten()
}

async fn inner_unseen_messages(
    client: Client,
) -> Result<impl Stream<Item = Result<(UserId, MessageId), ProtoDecodingError>>, NatsError> {
    let subscription = client.subscribe(CHANNEL_MESSAGE_UNSEEN.into()).await?;

    let stream =
        subscription.map(|proto_message| decode_proto_message_tag_request(proto_message.payload));

    Ok(stream)
}

/// Stream of all unseen notification for all messages from all users. Connected to NATS.
pub fn unseen_messages<'a>(
    client: Client,
) -> impl Stream<Item = Result<(UserId, MessageId), ReceiverError>> + 'a {
    inner_unseen_messages(client)
        .map_err(|e| ReceiverError::Nats(e))
        .map_ok(|stream| stream.map_err(|e| ReceiverError::Decoding(e)))
        .into_stream()
        .try_flatten()
}

/// Stream of new messges from specific users. Those users are feeded by a Stream.
pub fn new_messages_from_users<'a, U: Userlike, E: std::error::Error + Send + Sync + 'a>(
    users: impl Stream<Item = Result<U, E>> + 'a,
    client: Client,
) -> impl Stream<Item = Result<Message, ParReceiverError<E>>> + 'a {
    let new_messages = new_messages(client);

    let left_right = select(
        users.map_ok(|u| u.get_id()).map(Either::Left),
        new_messages.map(Either::Right),
    );

    let stream = left_right
        .scan(HashSet::<UserId>::new(), |user_list, either| {
            let res = Some(match either {
                Either::Left(Ok(user)) => {
                    user_list.insert(user);
                    None
                }
                Either::Right(Ok(message)) if user_list.contains(&message.user_id) => {
                    Some(Ok(message))
                }
                Either::Right(Ok(_)) => None,
                Either::Left(Err(e)) => Some(Err(ParReceiverError::Input(InputError(e)))),
                Either::Right(Err(e)) => Some(Err(e.into())),
            });

            async { res } // https://users.rust-lang.org/t/lifetime-confusing-on-futures-scan/42204
        })
        .filter_map(|e| async { e });

    stream
}

/// Stream of new friendships of a specific user.
pub fn new_friends_of_user<'a, U: Userlike>(
    user: U,
    client: Client,
) -> impl Stream<Item = Result<UserId, ReceiverError>> + 'a {
    let new_friendships = new_friendships(client);
    let user_id = user.get_id();

    let stream = new_friendships.filter_map(move |friendship| {
        let res = match friendship {
            Ok((user, friend)) if user.get_id() == user_id => Some(Ok(friend)),
            Err(e) => Some(Err(e)),
            _other => None,
        };

        async { res }
    });

    stream
}

/// Stream of removed friendships of a specific user.
pub fn removed_friends_of_user<'a, U: Userlike>(
    user: U,
    client: Client,
) -> impl Stream<Item = Result<UserId, ReceiverError>> + 'a {
    let removed_friendships = removed_friendships(client);
    let user_id = user.get_id();

    let stream = removed_friendships.filter_map(move |friendship| {
        let res = match friendship {
            Ok((user, friend)) if user.get_id() == user_id => Some(Ok(friend)),
            Err(e) => Some(Err(e)),
            _other => None,
        };

        async { res }
    });

    stream
}

/// Stream of removed friendships of a specific user.
pub fn friendships_updates<'a>(
    client: Client,
) -> impl Stream<Item = Result<FriendshipUpdate, ReceiverError>> + 'a {
    let new_friendships = new_friendships(client.clone());
    let removed_friendships = removed_friendships(client);

    let stream = select(
        new_friendships.map_ok(|(a, b)| FriendshipUpdate::New(a, b)),
        removed_friendships.map_ok(|(a, b)| FriendshipUpdate::Removed(a, b)),
    );

    stream
}
