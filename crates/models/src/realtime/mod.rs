//! Realtime features for users, all in form of streams.

use std::{collections::HashSet, pin::Pin, task::Poll};

use anyhow::Error;
use async_nats::{Client, Subscriber};
use chrono::NaiveDateTime;
use futures::{stream::StreamExt, Stream};
use prost::Message as ProstMessage;
use uuid::Uuid;

mod channels;

use channels::*;

use crate::{messages::Message, users::Userlike};

/// Takes a stream `T` of `I::Userlike` and outputs a stream of the newly posted messages from these users.
///
/// If stream `T` is closed/finished, output stream will continue with newly posted message of all users returned by stream
/// `T` before it closed.
pub struct UsersNewMessages<T, I>
where
    T: Stream<Item = I>,
    I: Userlike,
{
    users_stream: Option<T>,
    subscription: Subscriber,
    users: HashSet<Uuid>,
}

impl<T, I> UsersNewMessages<T, I>
where
    T: Stream<Item = I>,
    I: Userlike,
{
    pub async fn new(users: T, client: Client) -> Result<Self, Error> {
        Ok(Self {
            users_stream: Some(users),
            subscription: client.subscribe(CHANNEL_USER_MESSAGE.into()).await?,
            users: HashSet::new(),
        })
    }
}

fn parse_proto_message(payload: prost::bytes::Bytes) -> Result<Message, Error> {
    let m = proto::Message::decode_length_delimited(payload)?;

    let message = Message {
        id: Uuid::try_parse(m.message_id.as_str())?,
        user_id: Uuid::try_parse(m.user_id.as_str())?,
        date: NaiveDateTime::from_timestamp_millis(m.timestamp as i64).unwrap(),
        content: m.content,
    };

    Ok(message)
}

impl<T, I> Stream for UsersNewMessages<T, I>
where
    T: Stream<Item = I> + Unpin,
    I: Userlike,
{
    type Item = Result<Message, Error>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let keep_stream = if let Some(users_stream) = self.users_stream.as_mut() {
            // Get potential new user in list
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
                Some(nats_message) => {
                    match parse_proto_message(nats_message.payload) {
                        Ok(message) => {
                            // Filtering with users in the list
                            if self.users.contains(&message.user_id) {
                                Poll::Ready(Some(Ok(message)))
                            } else {
                                Poll::Pending
                            }
                        },
                        Err(e) => Poll::Ready(Some(Err(Error::from(e)))),
                    }
                }
                None => Poll::Ready(None),
            },
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Builder for a stream of new friendships with a specific user.
pub struct UserNewFriendships {
    pub user_id: Uuid,
}

/// Builder for a stream of newly seen messages from a list of a user.
pub struct UsersSeenMessages {
    pub users: HashSet<Uuid>,
}

#[cfg(test)]
#[tokio::test]
async fn message_broadcast_test() -> Result<(), Box<dyn std::error::Error>> {
    use async_nats::ConnectOptions;
    use futures::StreamExt;

    println!("Testing a simple NATS case: send/recv");

    let sender = tokio::spawn(async move {
        let sender = ConnectOptions::new()
            .connect("nats://ruser:password@127.0.0.1:4222")
            .await
            .unwrap();
        println!("SENDER: Connected to NATS: {}", sender.connection_state());

        for _ in 0..10 {
            let _ = tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let _ = sender.publish("notification".into(), "ouai".into()).await;
            println!("SENDER: Sent message");
        }
    });

    let receiver_1 = tokio::spawn(async move {
        let receiver = ConnectOptions::new()
            .connect("nats://ruser:password@127.0.0.1:4222")
            .await
            .unwrap();
        println!(
            "RECEIVER_1: Connected to NATS: {}",
            receiver.connection_state()
        );

        let mut sub = receiver.subscribe("notification".into()).await.unwrap();

        println!("RECEIVER_1: message: \"{:#?}\"", sub.next().await);
    });

    let _ = tokio::join!(receiver_1, sender);

    Ok(())
}
