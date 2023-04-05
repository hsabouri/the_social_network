use anyhow::Error;
use async_nats::Client as NatsClient;
use config::ServerConfig;
use once_cell::sync::OnceCell;
use scylla::Session;
use sqlx::PgPool;

static PG_POOL: OnceCell<PgPool> = OnceCell::new();
static SCYLLA_SESSION: OnceCell<Session> = OnceCell::new();
static NATS_CLIENT: OnceCell<NatsClient> = OnceCell::new();

#[derive(Clone)]
pub struct ServerConnections;

impl ServerConnections {
    pub async fn new(config: &ServerConfig) -> Result<Self, Error> {
        if PG_POOL.get().is_none() {
            let pg_pool = PgPool::connect_with(config.postgresql.into_connect_options()).await?;
            println!("Connected to PostreSQL");
            PG_POOL.set(pg_pool).expect("PG_POOL already initialized");
        }

        if SCYLLA_SESSION.get().is_none() {
            let scylla_session = config.scylladb.into_session_builder().build().await?;
            println!("Connected to ScyllaDB");
            SCYLLA_SESSION
                .set(scylla_session)
                .expect("SCYLLA_SESSION already initialized");
        }

        if NATS_CLIENT.get().is_none() {
            let nats_client = config.nats.into_connect_options().connect().await?;
            println!("Connected to NATS");
            NATS_CLIENT
                .set(nats_client)
                .expect("NATS_CLIENT already initialized");
        }

        Ok(Self)
    }

    pub fn get_scylla(&self) -> &'static Session {
        SCYLLA_SESSION.get().unwrap()
    }

    pub fn get_pg(&self) -> &'static PgPool {
        PG_POOL.get().unwrap()
    }

    pub fn get_nats(&self) -> &'static NatsClient {
        NATS_CLIENT.get().unwrap()
    }
}
