use anyhow::Error;
use futures::stream::{StreamExt, BoxStream};
use sqlx::PgExecutor;
use uuid::Uuid;

use crate::{repository::{
    messages::{GetLastMessagesOfUserRequest, InsertMessageRequest},
    users::{
        DeleteUserRequest, GetFriendsOfUserRequest, GetUser, InsertFriendshipRequest,
        InsertUserRequest,
    },
}, messages::Message};

/// Contains all functions that only requires the Uuid of the User.
pub trait Userlike {
    fn get_uuid(&self) -> Uuid;

    fn delete<'a, T: PgExecutor<'a> + Copy>(&self) -> DeleteUserRequest<T> {
        DeleteUserRequest::new(self.get_uuid())
    }

    fn friend_with(&self, other: impl Userlike) -> InsertFriendshipRequest {
        InsertFriendshipRequest::new(self.get_uuid(), other.get_uuid())
    }

    fn insert_message(&self, content: String) -> InsertMessageRequest {
        InsertMessageRequest::new(self.get_uuid(), content)
    }

    fn get_messages(&self) -> GetLastMessagesOfUserRequest {
        GetLastMessagesOfUserRequest::new(self.get_uuid())
    }

    fn get_friends<'a, T: PgExecutor<'a> + Copy>(&self) -> GetFriendsOfUserRequest<T> {
        GetFriendsOfUserRequest::new(self.get_uuid())
    }

    fn insert<'a, T: PgExecutor<'a> + Copy>(name: String) -> InsertUserRequest<T> {
        InsertUserRequest::new(name)
    }

    fn get_timeline<'a, T: 'a + PgExecutor<'a> + Copy>(&self, conn: T) -> BoxStream<'a, Result<Message, Error>> {
        let friends = self.get_friends().stream(conn);

        todo!()
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
    /// Retrieves full user information from DB and returns a `User`.
    pub async fn get_full_user<'a, T: PgExecutor<'a> + Copy>(self, conn: T) -> Result<User, Error> {
        GetUser::new(self.0).get(conn).await
    }
}

#[derive(Clone, Debug)]
pub struct User {
    pub id: Uuid,
    pub name: String,
}

impl Userlike for User {
    fn get_uuid(&self) -> Uuid {
        self.id
    }
}
