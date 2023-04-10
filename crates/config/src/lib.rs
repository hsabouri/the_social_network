use std::{fs::File, net::SocketAddr, ops::Deref, path::Path, str::FromStr, sync::Arc};

use async_nats::ConnectOptions as NatsConnectOptions;
use serde::{de, ser, Deserialize, Serialize};
use sqlx::postgres::{PgConnectOptions, PgSslMode};

fn deserialize_from_str<'de, T: FromStr, D>(deserializer: D) -> Result<T, D::Error>
where
    D: de::Deserializer<'de>,
    T::Err: std::fmt::Display,
{
    let s = String::deserialize(deserializer)?;
    T::from_str(&s).map_err(de::Error::custom)
}

fn serialize_pg_ssl_mode<S>(m: &PgSslMode, s: S) -> Result<S::Ok, S::Error>
where
    S: ser::Serializer,
{
    s.serialize_str(match m {
        PgSslMode::Disable => "disable",
        PgSslMode::Allow => "allow",
        PgSslMode::Prefer => "prefer",
        PgSslMode::Require => "require",
        PgSslMode::VerifyCa => "verify-ca",
        PgSslMode::VerifyFull => "verify-full",
    })
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScyllaHost(String);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PgHost(String);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NatsHost(String);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScyllaDbConfig {
    pub hostnames: Vec<ScyllaHost>,
    pub keyspace: String,
}

impl ScyllaDbConfig {
    pub fn into_session_builder(&self) -> scylla::SessionBuilder {
        let known_nodes: Vec<&String> = self.hostnames.iter().map(|n| &n.0).collect();

        scylla::SessionBuilder::new()
            .known_nodes(known_nodes.as_slice())
            .use_keyspace(&self.keyspace, false)
    }
}

// FIXME: Find a better way to store user/password
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PostgreSqlConfig {
    pub host: PgHost,
    #[serde(deserialize_with = "deserialize_from_str")]
    pub port: u16,
    #[serde(skip_serializing)]
    username: String,
    #[serde(skip_serializing)]
    password: String,
    pub database: String,
    #[serde(
        deserialize_with = "deserialize_from_str",
        serialize_with = "serialize_pg_ssl_mode"
    )]
    pub ssl_strategy: PgSslMode,
}

impl PostgreSqlConfig {
    pub fn into_connect_options(&self) -> PgConnectOptions {
        PgConnectOptions::new()
            .host(self.host.0.as_str())
            .port(self.port)
            .username(&self.username)
            .password(&self.password)
            .database(&self.database)
            .ssl_mode(self.ssl_strategy)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NatsConfig {
    pub host: NatsHost,
}

impl NatsConfig {
    pub fn into_connect_options(&self) -> NatsConnectOptionsWrapper {
        let options =
            NatsConnectOptions::new().connection_timeout(std::time::Duration::from_secs(3));
        NatsConnectOptionsWrapper {
            host: self.host.clone(),
            options,
        }
    }
}

/// The way `async_nats::ConnectOptions` is implemented is not compatible with creating
/// the connect options apart than connection.
pub struct NatsConnectOptionsWrapper {
    pub host: NatsHost,
    pub options: NatsConnectOptions,
}

impl NatsConnectOptionsWrapper {
    pub async fn connect(self) -> Result<async_nats::Client, async_nats::ConnectError> {
        self.options.connect(self.host.0).await
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InnerServerConfig {
    pub listening_addr: SocketAddr,
    pub scylladb: ScyllaDbConfig,
    pub postgresql: PostgreSqlConfig,
    pub nats: NatsConfig,
}

/// Can be shared between threads by using `Clone`. uses an `Arc` internally so cloning is cheap
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(flatten)]
    inner: Arc<InnerServerConfig>,
}

impl Deref for ServerConfig {
    type Target = InnerServerConfig;

    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

impl ServerConfig {
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, std::io::Error> {
        let file_content = File::open(path)?;

        let config = serde_json::from_reader(file_content)?;
        Ok(config)
    }
}
