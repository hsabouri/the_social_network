use sqlx::PgExecutor;
use uuid::Uuid;

use crate::repository::users::{InsertUserRequest, DeleteUserRequest, InsertFriendshipRequest};

#[derive(Clone, Debug)]
pub struct User {
    pub id: Uuid,
    pub name: String,
}

impl User {
    pub fn create<'a, T: PgExecutor<'a> + Copy>(name: String) -> InsertUserRequest<T> {
        InsertUserRequest::new(name)
    }

    pub fn delete<'a, T: PgExecutor<'a> + Copy>(&self) -> DeleteUserRequest<T> {
        DeleteUserRequest::new(self.id)
    }

    pub fn friend_with(&self, other: &User) -> InsertFriendshipRequest {
        InsertFriendshipRequest::new(self.id, other.id)
    }
}