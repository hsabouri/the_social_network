use anyhow::Error;
use futures::{stream::StreamExt, Stream};
use sqlx::PgPool;

use models::users::{Userlike, UserId, User};
use uuid::Uuid;

pub struct GetUser {
    pub user_id: UserId,
}

impl GetUser {
    pub fn new(user: impl Userlike) -> Self {
        Self {
            user_id: user.get_id(),
        }
    }

    pub async fn execute(self, conn: &PgPool) -> Result<User, Error> {
        let uuid: Uuid = self.user_id.into();

        let res = sqlx::query!(
            // language=PostgreSQL
            r#"
                SELECT name FROM users WHERE user_id = $1
            "#,
            uuid,
        )
        .fetch_one(conn)
        .await?;

        Ok(User {
            id: self.user_id,
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
            id: UserId::from(res.user_id),
            name: res.name,
        })
    }
}

/// Delete a user in database
#[derive(Copy, Clone)]
pub struct DeleteUserRequest {
    pub user_id: UserId,
}

impl DeleteUserRequest {
    pub fn new(user: impl Userlike) -> Self {
        Self {
            user_id: user.get_id(),
        }
    }

    pub async fn execute(self, conn: &PgPool) -> Result<(), Error> {
        let uuid: Uuid = self.user_id.into();

        let _res = sqlx::query!(
            // language=PostgreSQL
            r#"
                DELETE FROM users WHERE user_id = $1
            "#,
            uuid,
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
    pub user_a: UserId,
    pub user_b: UserId,
}

impl InsertFriendshipRequest {
    pub fn new(user_a: impl Userlike, user_b: impl Userlike) -> Self {
        Self {
            user_a: user_a.get_id(),
            user_b: user_b.get_id(),
        }
    }

    pub async fn execute(self, conn: &PgPool) -> Result<(), Error> {
        let uuid_a: Uuid = self.user_a.into();
        let uuid_b: Uuid = self.user_b.into();

        sqlx::query!(
            // language=PostgreSQL
            r#"
                INSERT INTO friendships (user_id, friend_id)
                    VALUES ($1, $2);
            "#,
            uuid_a,
            uuid_b,
        )
        .fetch_one(conn)
        .await?;

        Ok(())
    }
}

/// Removes frienship in both ways in a transaction
#[derive(Copy, Clone)]
pub struct RemoveFriendshipRequest {
    pub user_a: UserId,
    pub user_b: UserId,
}

impl RemoveFriendshipRequest {
    pub fn new(user_a: impl Userlike, user_b: impl Userlike) -> Self {
        Self {
            user_a: user_a.get_id(),
            user_b: user_b.get_id(),
        }
    }

    pub async fn execute(self, conn: &PgPool) -> Result<(), Error> {
        let uuid_a: Uuid = self.user_a.into();
        let uuid_b: Uuid = self.user_b.into();

        sqlx::query!(
            // language=PostgreSQL
            r#"
                DELETE FROM friendships
                    WHERE (user_id = $1
                        AND friend_id = $2)
                    OR (user_id = $2
                        AND friend_id = $1)
            "#,
            uuid_a,
            uuid_b,
        )
        .execute(conn)
        .await?;

        Ok(())
    }
}

#[derive(Copy, Clone)]
pub struct GetFriendsOfUserRequest {
    pub user_id: UserId,
}

impl GetFriendsOfUserRequest {
    pub fn new(user: impl Userlike) -> Self {
        Self {
            user_id: user.get_id(),
        }
    }

    pub fn stream<'a>(self, conn: &'a PgPool) -> impl Stream<Item = Result<UserId, Error>> + 'a {
        let uuid: Uuid = self.user_id.into();

        sqlx::query!(
            // language=PostgreSQL
            r#"
                SELECT friend_id FROM friendships WHERE user_id = $1 OR friend_id = $1
            "#,
            uuid,
        )
        .fetch(conn)
        .map(|record| Ok(record.map(|record| UserId::from(record.friend_id))?))
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
            id: res.user_id.into(),
            name: self.name,
        })
    }
}
