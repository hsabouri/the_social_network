use anyhow::Error;
use config::ServerConfig;
use once_cell::sync::OnceCell;
use scylla::{Session, SessionBuilder};
use sqlx::PgPool;

static PG_POOL: OnceCell<PgPool> = OnceCell::new();
static SCYLLA_SESSION: OnceCell<Session> = OnceCell::new();

#[derive(Clone)]
pub struct ServerConnections;

impl ServerConnections {
    pub async fn new(config: &ServerConfig) -> Result<Self, Error> {
        if PG_POOL.get().is_none() {
            let pg_pool = PgPool::connect_with(config.postgresql.into_options()).await?;
            PG_POOL.set(pg_pool).expect("PG_POOL already initialized");
        }

        if SCYLLA_SESSION.get().is_none() {
            let scylla_session: SessionBuilder = config.scylladb.into_builder();
            let scylla_session = scylla_session.build().await?;

            SCYLLA_SESSION
                .set(scylla_session)
                .expect("SCYLLA_SESSION already initialized");
        }

        Ok(Self)
    }

    pub fn get_scylla(&self) -> &'static Session {
        SCYLLA_SESSION.get().unwrap()
    }

    pub fn get_pg(&self) -> &'static PgPool {
        PG_POOL.get().unwrap()
    }
}
