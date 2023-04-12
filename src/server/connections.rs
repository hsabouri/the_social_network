use std::sync::Arc;

use anyhow::Error;
use async_nats::Client as NatsClient;
use config::ServerConfig;
use once_cell::sync::OnceCell;
use scylla::Session;
use sqlx::PgPool;

static PG_POOL: OnceCell<PgPool> = OnceCell::new();

#[derive(Clone)]
pub struct ServerConnections {
    nats_client: NatsClient,
    scylla_session: Arc<Session>,
}

impl ServerConnections {
    pub async fn new(config: &ServerConfig) -> Result<Self, Error> {
        if PG_POOL.get().is_none() {
            let pg_pool = PgPool::connect_with(config.postgresql.into_connect_options()).await?;
            println!("Connected to PostreSQL");
            PG_POOL.set(pg_pool).expect("PG_POOL already initialized");
        }

        let scylla_session = config.scylladb.into_session_builder().build().await?;
        println!("Connected to ScyllaDB");

        let nats_client = config.nats.into_connect_options().connect().await?;
        println!("Connected to NATS");

        Ok(Self {
            nats_client,
            scylla_session: Arc::new(scylla_session),
        })
    }

    pub fn get_scylla(&self) -> &Session {
        self.scylla_session.as_ref()
    }

    pub fn get_pg(&self) -> &'static PgPool {
        PG_POOL.get().unwrap()
    }

    pub fn get_nats(&self) -> NatsClient {
        self.nats_client.clone()
    }
}
