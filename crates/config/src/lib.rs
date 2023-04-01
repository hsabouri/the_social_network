use std::{fs::File, net::SocketAddr, ops::Deref, path::Path, str::FromStr, sync::Arc};

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

pub type ScyllaHost = String;
pub type PgHost = String;

#[derive(Debug, Serialize, Deserialize)]
pub struct ScyllaDbConfig {
    pub hostnames: Vec<ScyllaHost>,
}

impl Into<scylla::SessionBuilder> for ScyllaDbConfig {
    fn into(self) -> scylla::SessionBuilder {
        scylla::SessionBuilder::new().known_nodes(self.hostnames.as_slice())
    }
}

// FIXME: Find a better way to store user/password
#[derive(Debug, Serialize, Deserialize)]
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

impl Into<PgConnectOptions> for PostgreSqlConfig {
    fn into(self) -> PgConnectOptions {
        PgConnectOptions::new()
            .host(self.host.as_str())
            .port(self.port)
            .username(&self.username)
            .password(&self.password)
            .database(&self.database)
            .ssl_mode(self.ssl_strategy)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InnerServerConfig {
    pub listening_addr: SocketAddr,
    pub scylladb: ScyllaDbConfig,
    pub postgresql: PostgreSqlConfig,
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
