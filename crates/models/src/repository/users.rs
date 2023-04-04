use anyhow::Error;
use futures::{stream::StreamExt, Stream};
use sqlx::PgPool;
use uuid::Uuid;

use crate::users::{User, UserRef};

pub struct GetUser {
    pub user_id: Uuid,
}

impl GetUser {
    pub fn new(user_id: Uuid) -> Self {
        Self { user_id }
    }

    pub async fn execute(self, conn: &PgPool) -> Result<User, Error> {
        let res = sqlx::query!(
            // language=PostgreSQL
            r#"
                SELECT name FROM users WHERE user_id = $1
            "#,
            self.user_id,
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
            id: res.user_id,
            name: res.name,
        })
    }
}

/// Delete a user in database
#[derive(Copy, Clone)]
pub struct DeleteUserRequest {
    pub uuid: Uuid,
}

impl DeleteUserRequest {
    pub fn new(uuid: Uuid) -> Self {
        Self { uuid }
    }

    pub async fn execute(self, conn: &PgPool) -> Result<(), Error> {
        let _res = sqlx::query!(
            // language=PostgreSQL
            r#"
                DELETE FROM users WHERE user_id = $1
            "#,
            self.uuid,
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
    pub user_a: Uuid,
    pub user_b: Uuid,
}

impl InsertFriendshipRequest {
    pub fn new(user_a: Uuid, user_b: Uuid) -> Self {
        Self { user_a, user_b }
    }

    pub async fn execute(self, conn: &PgPool) -> Result<(), Error> {
        let mut t = conn.begin().await?;

        sqlx::query!(
            // language=PostgreSQL
            r#"
                INSERT INTO friendships (user_id, friend_id)
                    VALUES ($1, $2);
            "#,
            self.user_a,
            self.user_b,
        )
        .execute(&mut t)
        .await?;

        sqlx::query!(
            // language=PostgreSQL
            r#"
                INSERT INTO friendships (user_id, friend_id)
                    VALUES ($2, $1);
            "#,
            self.user_a,
            self.user_b,
        )
        .execute(&mut t)
        .await?;

        t.commit().await?;

        Ok(())
    }
}

#[derive(Copy, Clone)]
pub struct GetFriendsOfUserRequest {
    pub user_id: Uuid,
}

impl GetFriendsOfUserRequest {
    pub fn new(user_id: Uuid) -> Self {
        Self { user_id }
    }

    pub fn stream<'a>(self, conn: &'a PgPool) -> impl Stream<Item = Result<UserRef, Error>> + 'a {
        sqlx::query!(
            // language=PostgreSQL
            r#"
                SELECT user_id FROM friendships WHERE user_id = $1
            "#,
            self.user_id,
        )
        .fetch(conn)
        .map(|record| Ok(record.map(|record| UserRef(record.user_id))?))
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
