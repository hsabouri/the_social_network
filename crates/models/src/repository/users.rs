use anyhow::Error;
use futures::{stream::StreamExt, Stream};
use sqlx::PgPool;

use crate::users::{User, UserRef, Userlike};

pub struct GetUser {
    pub user: UserRef,
}

impl GetUser {
    pub fn new(user_id: impl Userlike) -> Self {
        Self {
            user: UserRef::new(user_id.get_uuid()),
        }
    }

    pub async fn execute(self, conn: &PgPool) -> Result<User, Error> {
        let res = sqlx::query!(
            // language=PostgreSQL
            r#"
                SELECT name FROM users WHERE user_id = $1
            "#,
            self.user.get_uuid(),
        )
        .fetch_one(conn)
        .await?;

        Ok(User {
            id: self.user.get_uuid(),
            name: res.name,
        })
    }
}

/// Insert a user in database
#[derive(Clone)]
pub struct InsertUserRequest {
    pub name: String,
}

impl InsertUserRequest {
    pub fn new(name: String) -> Self {
        Self { name }
    }

    pub async fn execute(self, conn: &PgPool) -> Result<User, Error> {
        let res = sqlx::query!(
            // language=PostgreSQL
            r#"
                INSERT INTO users (name)
                    values ($1)
                RETURNING user_id, name
            "#,
            self.name,
        )
        .fetch_one(conn)
        .await?;

        Ok(User {
            id: res.user_id,
            name: res.name,
        })
    }
}

/// Delete a user in database
#[derive(Copy, Clone)]
pub struct DeleteUserRequest {
    pub user: UserRef,
}

impl DeleteUserRequest {
    pub fn new(user: impl Userlike) -> Self {
        Self {
            user: UserRef::new(user.get_uuid()),
        }
    }

    pub async fn execute(self, conn: &PgPool) -> Result<(), Error> {
        let _res = sqlx::query!(
            // language=PostgreSQL
            r#"
                DELETE FROM users WHERE user_id = $1
            "#,
            self.user.get_uuid(),
        )
        .fetch_one(conn)
        .await?;

        Ok(())
    }
}

/// Insert two frienship rows as a transaction :
/// * A -> B
/// * B -> A
#[derive(Copy, Clone)]
pub struct InsertFriendshipRequest {
    pub user_a: UserRef,
    pub user_b: UserRef,
}

impl InsertFriendshipRequest {
    pub fn new(user_a: impl Userlike, user_b: impl Userlike) -> Self {
        Self {
            user_a: UserRef::new(user_a.get_uuid()),
            user_b: UserRef::new(user_b.get_uuid()),
        }
    }

    pub async fn execute(self, conn: &PgPool) -> Result<(), Error> {
        sqlx::query!(
            // language=PostgreSQL
            r#"
                INSERT INTO friendships (user_id, friend_id)
                    VALUES ($1, $2);
            "#,
            self.user_a.get_uuid(),
            self.user_b.get_uuid(),
        )
        .fetch_one(conn)
        .await?;

        Ok(())
    }
}

/// Removes frienship in both ways in a transaction
#[derive(Copy, Clone)]
pub struct RemoveFriendshipRequest {
    pub user_a: UserRef,
    pub user_b: UserRef,
}

impl RemoveFriendshipRequest {
    pub fn new(user_a: impl Userlike, user_b: impl Userlike) -> Self {
        Self {
            user_a: UserRef::new(user_a.get_uuid()),
            user_b: UserRef::new(user_b.get_uuid()),
        }
    }

    pub async fn execute(self, conn: &PgPool) -> Result<(), Error> {
        sqlx::query!(
            // language=PostgreSQL
            r#"
                DELETE FROM friendships
                    WHERE (user_id = $1
                        AND friend_id = $2)
                    OR (user_id = $2
                        AND friend_id = $1)
            "#,
            self.user_a.get_uuid(),
            self.user_b.get_uuid(),
        )
        .execute(conn)
        .await?;

        Ok(())
    }
}

#[derive(Copy, Clone)]
pub struct GetFriendsOfUserRequest {
    pub user: UserRef,
}

impl GetFriendsOfUserRequest {
    pub fn new(user: impl Userlike) -> Self {
        Self {
            user: UserRef::new(user.get_uuid()),
        }
    }

    pub fn stream<'a>(self, conn: &'a PgPool) -> impl Stream<Item = Result<UserRef, Error>> + 'a {
        sqlx::query!(
            // language=PostgreSQL
            r#"
                SELECT friend_id FROM friendships WHERE user_id = $1 OR friend_id = $1
            "#,
            self.user.get_uuid(),
        )
        .fetch(conn)
        .map(|record| Ok(record.map(|record| UserRef(record.friend_id))?))
    }
}

pub struct GetUserByNameRequest {
    pub name: String,
}

impl GetUserByNameRequest {
    pub fn new(name: String) -> Self {
        Self { name }
    }

    pub async fn execute(self, conn: &PgPool) -> Result<User, Error> {
        let res = sqlx::query!(
            // language=PostgreSQL
            r#"
                SELECT user_id FROM users WHERE name = $1
            "#,
            self.name,
        )
        .fetch_one(conn)
        .await?;

        Ok(User {
            id: res.user_id,
            name: self.name,
        })
    }
}
