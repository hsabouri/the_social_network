use std::collections::HashSet;

use anyhow::Error;
use async_nats::Client;
use async_trait::async_trait;
use futures::{
    future::Either,
    stream::{select, select_all, StreamExt, TryStreamExt},
    Stream,
};
use scylla::Session;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    messages::Message,
    realtime::{self, FriendshipUpdate, PublishFriendship, PublishRemoveFriendship},
    repository::{
        messages::{GetLastMessagesOfUserRequest, InsertMessageRequest},
        users::*,
    },
};

/// Contains all functions that only requires the Uuid of the User.
#[async_trait]
pub trait Userlike: Sized {
    fn get_uuid(&self) -> Uuid;

    fn delete(&self) -> DeleteUserRequest {
        DeleteUserRequest::new(self.get_uuid())
    }

    fn friend_with(&self, other: impl Userlike) -> InsertFriendshipRequest {
        InsertFriendshipRequest::new(self.get_uuid(), other.get_uuid())
    }

    fn remove_friend(&self, other: impl Userlike) -> RemoveFriendshipRequest {
        RemoveFriendshipRequest::new(self.get_uuid(), other.get_uuid())
    }

    fn insert_message(&self, content: String) -> InsertMessageRequest {
        InsertMessageRequest::new(self.get_uuid(), content)
    }

    fn get_messages(&self) -> GetLastMessagesOfUserRequest {
        GetLastMessagesOfUserRequest::new(self.get_uuid())
    }

    fn get_friends(&self) -> GetFriendsOfUserRequest {
        GetFriendsOfUserRequest::new(self.get_uuid())
    }

    fn insert(name: String) -> InsertUserRequest {
        InsertUserRequest::new(name)
    }

    fn realtime_friend_with(self, other: impl Userlike) -> PublishFriendship {
        PublishFriendship::new(self, other)
    }

    fn realtime_remove_friend(self, other: impl Userlike) -> PublishRemoveFriendship {
        PublishRemoveFriendship::new(self, other)
    }

    fn downgrade(&self) -> UserRef {
        UserRef::new(self.get_uuid())
    }
}

impl Userlike for Uuid {
    fn get_uuid(&self) -> Uuid {
        *self
    }
}

/// Stores only the Uuid of the user.
/// Provides methods to easily get the full user infos at the expense of a request to DB.
#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub struct UserRef(pub Uuid);

impl Userlike for UserRef {
    fn get_uuid(&self) -> Uuid {
        self.0
    }
}

impl Userlike for &UserRef {
    fn get_uuid(&self) -> Uuid {
        self.0
    }
}

impl UserRef {
    pub fn from_str_uuid(user_id: impl AsRef<str>) -> Result<Self, Error> {
        let uuid = Uuid::try_parse(user_id.as_ref())
            .map_err(|e| Error::from(e).context("malformed UUID"))?;

        Ok(Self::new(uuid))
    }

    pub fn new(user_id: Uuid) -> Self {
        Self(user_id)
    }

    /// Retrieves full user information from DB and returns a `User`.
    pub async fn upgrade(self, conn: &PgPool) -> Result<User, Error> {
        GetUser::new(self.0).execute(conn).await
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
        enum FriendUpdate {
            New(UserRef),
            Removed(UserRef),
        }

        let initial_friends = self
            .get_friends()
            .stream(pg)
            .map_ok(|f| FriendUpdate::New(f));

        let updates =
            realtime::friendships_updates(nats.clone()).filter_map(move |f| async move {
                match f {
                    Ok(
                        FriendshipUpdate::New(user, friend) | FriendshipUpdate::New(friend, user),
                    ) if user == self.downgrade() => Some(Ok(FriendUpdate::New(friend))),
                    Ok(
                        FriendshipUpdate::Removed(user, friend)
                        | FriendshipUpdate::Removed(friend, user),
                    ) if user == self.downgrade() => Some(Ok(FriendUpdate::Removed(friend))),
                    Ok(_other) => None,
                    Err(e) => Some(Err(e)),
                }
            });

        let friends = initial_friends.chain(updates);
        let messages = realtime::new_messages(nats.clone());

        let stream = select(friends.map(Either::Left), messages.map(Either::Right));

        let stream = stream
            .scan(HashSet::<UserRef>::new(), |user_list, either| {
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
                    Either::Right(Ok(message))
                        if user_list.contains(&UserRef::new(message.user_id)) =>
                    {
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

#[derive(Clone, Debug)]
pub struct User {
    pub id: Uuid,
    pub name: String,
}

impl User {
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

    pub fn real_time_timeline<'a>(
        &'a self,
        pg: &'a PgPool,
        nats: Client,
    ) -> impl Stream<Item = Result<Message, Error>> + 'a {
        self.downgrade().real_time_timeline(pg, nats)
    }
}

impl Userlike for User {
    fn get_uuid(&self) -> Uuid {
        self.id
    }
}

impl Userlike for &User {
    fn get_uuid(&self) -> Uuid {
        self.id
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
        .collect::<Vec<Result<UserRef, Error>>>()
        .await;

    let friends_streams: Vec<_> = friends
        .into_iter()
        .filter_map(|f| f.ok())
        .map(|f| Box::pin(f.get_messages().stream(session)))
        .collect();

    let stream = select_all(friends_streams);

    stream
}
