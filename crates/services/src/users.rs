use std::{collections::HashSet, ops::Deref};

use anyhow::Error;
use futures::{
    future::Either,
    stream::{select, select_all, StreamExt, TryStreamExt},
    Stream,
};

use realtime::{self, Client};
use repository::{
    messages::{GetLastMessagesOfUserRequest, InsertMessageRequest},
    users::GetUserByNameRequest,
    PgPool, Session,
};

use models::{
    friendships::{FriendUpdate, FriendshipUpdate},
    messages::Message,
    users::{User, UserId, Userlike},
};
use repository::users::{
    DeleteUserRequest, GetFriendsOfUserRequest, InsertFriendshipRequest, InsertUserRequest,
    RemoveFriendshipRequest,
};

pub trait UserlikeServices: Userlike {
    fn delete(&self) -> DeleteUserRequest {
        DeleteUserRequest::new(self.get_id())
    }

    fn friend_with(&self, other: impl Userlike) -> InsertFriendshipRequest {
        InsertFriendshipRequest::new(self.get_id(), other.get_id())
    }

    fn remove_friend(&self, other: impl Userlike) -> RemoveFriendshipRequest {
        RemoveFriendshipRequest::new(self.get_id(), other.get_id())
    }

    fn insert_message(&self, content: String) -> InsertMessageRequest {
        InsertMessageRequest::new(self.get_id(), content)
    }

    fn get_messages(&self) -> GetLastMessagesOfUserRequest {
        GetLastMessagesOfUserRequest::new(self.get_id())
    }

    fn get_friends(&self) -> GetFriendsOfUserRequest {
        GetFriendsOfUserRequest::new(self.get_id())
    }

    fn insert(name: String) -> InsertUserRequest {
        InsertUserRequest::new(name)
    }

    fn realtime_friend_with(self, other: impl Userlike) -> realtime::senders::PublishFriendship {
        realtime::senders::PublishFriendship::new(self, other)
    }

    fn realtime_remove_friend(
        self,
        other: impl Userlike,
    ) -> realtime::senders::PublishRemoveFriendship {
        realtime::senders::PublishRemoveFriendship::new(self, other)
    }
}

impl<T: Userlike> UserlikeServices for T {}

#[derive(Clone, Copy)]
pub struct UserIdServices(UserId);

impl Deref for UserIdServices {
    type Target = UserId;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Userlike for UserIdServices {
    fn get_id(&self) -> UserId {
        self.0.get_id()
    }
}

impl UserIdServices {
    pub fn new(user: impl Userlike) -> Self {
        Self (user.get_id())
    }

    pub async fn get_timeline<'a>(
        self,
        conn: &'a PgPool,
        session: &'a Session,
    ) -> impl Stream<Item = Result<Message, Error>> + 'a {
        get_timeline(self, conn, session).await
    }

    pub fn real_time_timeline<'a>(
        self,
        pg: &'a PgPool,
        nats: Client,
    ) -> impl Stream<Item = Result<Message, Error>> + 'a {
        let self_id = self.get_id();
        let initial_friends = self
            .get_friends()
            .stream(pg)
            .map_ok(|f| FriendUpdate::New(f));

        let updates = realtime::receivers::friendships_updates(nats.clone()).filter_map(
            move |f| async move {
                match f {
                    Ok(
                        FriendshipUpdate::New(user, friend) | FriendshipUpdate::New(friend, user),
                    ) if user == self_id => Some(Ok(FriendUpdate::New(friend))),
                    Ok(
                        FriendshipUpdate::Removed(user, friend)
                        | FriendshipUpdate::Removed(friend, user),
                    ) if user == self_id => Some(Ok(FriendUpdate::Removed(friend))),
                    Ok(_other) => None,
                    Err(e) => Some(Err(e)),
                }
            },
        );

        let friends = initial_friends.chain(updates);
        let messages = realtime::receivers::new_messages(nats.clone());

        let stream = select(friends.map(Either::Left), messages.map(Either::Right));

        let stream = stream
            .scan(HashSet::<UserId>::new(), |user_list, either| {
                let res = Some(match either {
                    Either::Left(Ok(friend)) => {
                        match friend {
                            FriendUpdate::New(friend) => {
                                user_list.insert(friend);
                            }
                            FriendUpdate::Removed(friend) => {
                                user_list.remove(&friend);
                            }
                        }
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
}

#[derive(Clone)]
pub struct UserServices(User);

impl Userlike for UserServices {
    fn get_id(&self) -> UserId {
        self.id
    }
}

impl Userlike for &UserServices {
    fn get_id(&self) -> UserId {
        self.id
    }
}

impl Deref for UserServices {
    type Target = User;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl UserServices {
    pub fn get_by_name(name: String) -> GetUserByNameRequest {
        GetUserByNameRequest::new(name)
    }

    pub async fn get_timeline<'a>(
        &'a self,
        conn: &'a PgPool,
        session: &'a Session,
    ) -> impl Stream<Item = Result<Message, Error>> + 'a {
        get_timeline(self, conn, session).await
    }
}

async fn get_timeline<'a>(
    user: impl Userlike + 'a,
    conn: &'a PgPool,
    session: &'a Session,
) -> impl Stream<Item = Result<Message, Error>> + 'a {
    let friends = user
        .get_friends()
        .stream(conn)
        .collect::<Vec<Result<UserId, Error>>>()
        .await;

    let friends_streams: Vec<_> = friends
        .into_iter()
        .filter_map(|f| f.ok())
        .map(|f| Box::pin(f.get_messages().stream(session)))
        .collect();

    let stream = select_all(friends_streams);

    stream
}
