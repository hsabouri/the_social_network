use std::collections::HashSet;

use anyhow::Error;
use async_nats::Client;
use futures::future::Either;
use futures::stream::select;
use futures::{FutureExt, Stream};
use futures::{StreamExt, TryStreamExt};
use uuid::Uuid;

use super::channels::*;
use super::codec::*;

use crate::{
    messages::{Message, MessageRef},
    users::{UserRef, Userlike},
};

async fn inner_new_messages(
    client: &Client,
) -> Result<impl Stream<Item = Result<Message, Error>>, Error> {
    let subscription = client.subscribe(CHANNEL_MESSAGE.into()).await?;

    let stream = subscription.map(|proto_message| decode_proto_message(proto_message.payload));

    Ok(stream)
}

/// Stream of all new messages from all users. Connected to NATS.
pub fn new_messages<'a>(
    client: &'a Client,
) -> impl Stream<Item = Result<Message, Error>> + 'a {
    inner_new_messages(client).into_stream().try_flatten()
}

async fn inner_new_friendships(
    client: &Client,
) -> Result<impl Stream<Item = Result<(UserRef, UserRef), Error>>, Error> {
    let subscription = client.subscribe(CHANNEL_FRIENDSHIP.into()).await?;

    let stream = subscription.map(|proto_message| decode_proto_friendship(proto_message.payload));

    Ok(stream)
}

/// Stream of all new friendships of all users. Connected to NATS.
pub fn new_friendships<'a>(
    client: &'a Client,
) -> impl Stream<Item = Result<(UserRef, UserRef), Error>> + 'a {
    inner_new_friendships(client).into_stream().try_flatten()
}

async fn inner_seen_messages(
    client: &Client,
) -> Result<impl Stream<Item = Result<(UserRef, MessageRef), Error>>, Error> {
    let subscription = client.subscribe(CHANNEL_MESSAGE_SEEN.into()).await?;

    let stream =
        subscription.map(|proto_message| decode_proto_message_tag_request(proto_message.payload));

    Ok(stream)
}

/// Stream of all seen notification for all messages from all users. Connected to NATS.
pub fn seen_messages<'a>(
    client: &'a Client,
) -> impl Stream<Item = Result<(UserRef, MessageRef), Error>> + 'a {
    inner_seen_messages(client).into_stream().try_flatten()
}

async fn inner_unseen_messages(
    client: &Client,
) -> Result<impl Stream<Item = Result<(UserRef, MessageRef), Error>>, Error> {
    let subscription = client
        .subscribe(CHANNEL_MESSAGE_UNSEEN.into()).await?;

    let stream =
        subscription.map(|proto_message| decode_proto_message_tag_request(proto_message.payload));

    Ok(stream)
}

/// Stream of all unseen notification for all messages from all users. Connected to NATS.
pub fn unseen_messages<'a>(
    client: &'a Client,
) -> impl Stream<Item = Result<(UserRef, MessageRef), Error>> + 'a {
    inner_unseen_messages(client).into_stream().try_flatten()
}

/// Stream of new messges from specific users. Those users are feeded by a Stream.
pub fn new_messages_from_users<'a, U: Userlike>(
    users: impl Stream<Item = Result<U, Error>> + 'a,
    client: &'a Client,
) -> impl Stream<Item = Result<Message, Error>> + 'a {
    let new_messages = new_messages(client);

    let left_right = select(
        users.map_ok(|u| u.get_uuid()).map(Either::Left),
        new_messages.map(Either::Right),
    );

    let stream = left_right
        .scan(HashSet::<Uuid>::new(), |user_list, either| {
            let res = Some(match either {
                Either::Left(Ok(user)) => {
                    user_list.insert(user);
                    None
                }
                Either::Right(Ok(message)) if user_list.contains(&message.user_id) => {
                    Some(Ok(message))
                }
                Either::Right(Ok(_)) => None,
                Either::Left(Err(e)) => Some(Err(e)),
                Either::Right(Err(e)) => Some(Err(e)),
            });

            async { res } // https://users.rust-lang.org/t/lifetime-confusing-on-futures-scan/42204
        })
        .filter_map(|e| async { e });

    stream
}

/// Stream of new friendships of a specific user.
pub fn new_friends_of_user<'a, U: Userlike>(
    user: U,
    client: &'a Client,
) -> impl Stream<Item = Result<UserRef, Error>> + 'a {
    let new_friendships = new_friendships(client);
    let user_id = user.get_uuid();

    let stream = new_friendships.filter_map(move |friendship| {
        let res = match friendship {
            Ok((user, friend)) if user.get_uuid() == user_id => Some(Ok(friend)),
            Err(e) => Some(Err(e)),
            _other => None,
        };

        async { res }
    });

    stream
}
