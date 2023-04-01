use sqlx::PgExecutor;
use uuid::Uuid;

use crate::repository::{
    messages::{GetLastMessagesOfUserRequest, InsertMessageRequest},
    users::{DeleteUserRequest, InsertFriendshipRequest, InsertUserRequest},
};

#[derive(Clone, Debug)]
pub struct User {
    pub id: Uuid,
    pub name: String,
}

impl User {
    pub fn insert<'a, T: PgExecutor<'a> + Copy>(name: String) -> InsertUserRequest<T> {
        InsertUserRequest::new(name)
    }

    pub fn delete<'a, T: PgExecutor<'a> + Copy>(&self) -> DeleteUserRequest<T> {
        DeleteUserRequest::new(self.id)
    }

    pub fn friend_with(&self, other: &User) -> InsertFriendshipRequest {
        InsertFriendshipRequest::new(self.id, other.id)
    }

    pub fn insert_message(&self, content: String) -> InsertMessageRequest {
        InsertMessageRequest::new(self.id, content)
    }

    pub fn get_messages(&self) -> GetLastMessagesOfUserRequest {
        GetLastMessagesOfUserRequest::new(self.id)
    }
}
