use anyhow::Error;
use sqlx::postgres::PgExecutor;

use crate::users::User;

#[derive(Clone)]
pub struct NewUser {
    pub name: String,
}

impl NewUser {
    pub async fn execute<'a, T: PgExecutor<'a>>(self, conn: &'a T) -> Result<User, Error> {
        conn.execute(sqlx::query!(
            // language=PostgreSQL
            r#"
                insert into "user"(username, password_hash)
                values ($1, $2)
            "#,
            username,
            password_hash
        )).await?;

        todo!()
    }
}