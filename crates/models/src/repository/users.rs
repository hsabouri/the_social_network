use std::marker::PhantomData;

use anyhow::Error;
use sqlx::{postgres::PgExecutor, PgPool};
use uuid::Uuid;

use crate::users::User;

/// Insert a user in database
#[derive(Clone)]
pub struct InsertUserRequest<T> {
    pub name: String,
    _f: PhantomData<T>,
}

impl<'a, T> InsertUserRequest<T>
where
    T: PgExecutor<'a> + Copy,
{
    pub fn new(name: String) -> Self {
        Self {
            name,
            _f: PhantomData::<T>::default(),
        }
    }

    pub async fn execute(self, conn: T) -> Result<User, Error> {
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
pub struct DeleteUserRequest<T> {
    pub uuid: Uuid,
    _f: PhantomData<T>,
}

impl<'a, T> DeleteUserRequest<T>
where
    T: PgExecutor<'a> + Copy,
{
    pub fn new(uuid: Uuid) -> Self {
        Self {
            uuid,
            _f: PhantomData::<T>::default(),
        }
    }

    pub async fn execute(self, conn: T) -> Result<(), Error> {
        let res = sqlx::query!(
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

    pub async fn execute(self, conn: PgPool) -> Result<(), Error> {
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
