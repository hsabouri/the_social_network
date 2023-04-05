use anyhow::Error;
use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt};
use scylla::Session;
use sqlx::PgPool;
use uuid::Uuid;

use stream_helpers::MergeSortedTryStreams;

use crate::{
    messages::Message,
    repository::{
        messages::{GetLastMessagesOfUserRequest, InsertMessageRequest},
        users::*,
    },
};

/// Contains all functions that only requires the Uuid of the User.
#[async_trait]
pub trait Userlike {
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

    async fn get_timeline<'a>(
        &self,
        conn: &'a PgPool,
        session: &'a Session,
    ) -> Result<BoxStream<'a, Result<Message, Error>>, Error> {
        let friends = self
            .get_friends()
            .stream(conn)
            .collect::<Vec<Result<UserRef, Error>>>()
            .await;

        let friends_streams: Vec<_> = friends
            .into_iter()
            .filter_map(|f| f.ok())
            .map(|f| Box::pin(f.get_messages().stream(&*session)))
            .collect();

        Ok(Box::pin(MergeSortedTryStreams::new(friends_streams)))
    }
}

impl Userlike for Uuid {
    fn get_uuid(&self) -> Uuid {
        *self
    }
}

/// Stores only the Uuid of the user.
/// Provides methods to easily get the full user infos at the expense of a request to DB.
#[derive(Clone, Copy)]
pub struct UserRef(pub Uuid);

impl Userlike for UserRef {
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
    pub async fn get_full_user(self, conn: &PgPool) -> Result<User, Error> {
        GetUser::new(self.0).execute(conn).await
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
}

impl Userlike for User {
    fn get_uuid(&self) -> Uuid {
        self.id
    }
}
