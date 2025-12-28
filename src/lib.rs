//! ClickHouse-BB8 is a [ClickHouse client](https://docs.rs/clickhouse/latest/clickhouse/)
//! pool manager compatible with [BB8](https://docs.rs/bb8/latest/bb8).
//!
//! It allows you to configure a pool of Rust clients for ClickHouse that can be reused without
//! sharing the underlying HTTP client connection.
//!
//! # Example
//!
//! ```no_run
//! use clickhouse_bb8::{ConnectionBuilder, ConnectionManager, Pool};
//!
//! # async fn example() {
//! let builder = ConnectionBuilder::new()
//!     .with_url("http://localhost:8123")
//!     .with_database("my_database");
//!
//! let manager = ConnectionManager::new(builder);
//! let pool = Pool::builder()
//!     .build(manager)
//!     .await
//!     .unwrap();
//!
//! let conn = pool.get().await.unwrap();
//! // Use conn for queries
//! # }
//! ```

use async_trait::async_trait;
use bb8::ManageConnection;
use clickhouse::{Client, Compression};
use std::ops::{Deref, DerefMut};
use thiserror::Error;

/// Errors that can occur during connection management.
#[derive(Error, Debug)]
pub enum ClickHouseError {
    #[error("Failed to create connection")]
    ConnectionFailed(#[from] clickhouse::error::Error),

    #[error("Health check failed")]
    HealthCheckFailed(#[from] Box<clickhouse::error::Error>),
}

/// Builder for creating ClickHouse clients with custom configuration.
///
/// This builder captures the configuration needed to create multiple ClickHouse client instances.
/// It exposes the same `with_*` methods as the `clickhouse::Client` struct.
#[derive(Clone)]
pub struct ConnectionBuilder {
    url: Option<String>,
    database: Option<String>,
    user: Option<String>,
    password: Option<String>,
    access_token: Option<String>,
    compression: Option<Compression>,
    headers: Vec<(String, String)>,
    options: Vec<(String, String)>,
    roles: Option<Vec<String>>,
    default_roles: bool,
    product_name: Option<String>,
    product_version: Option<String>,
    validation: bool,
}

impl Default for ConnectionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionBuilder {
    /// Creates a new `ConnectionBuilder` with default settings.
    pub fn new() -> Self {
        Self {
            url: None,
            database: None,
            user: None,
            password: None,
            access_token: None,
            compression: None,
            headers: Vec::new(),
            options: Vec::new(),
            roles: None,
            default_roles: false,
            product_name: None,
            product_version: None,
            validation: false,
        }
    }

    /// Sets the ClickHouse server URL.
    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Sets the default database.
    pub fn with_database(mut self, database: impl Into<String>) -> Self {
        self.database = Some(database.into());
        self
    }

    /// Sets the database user.
    pub fn with_user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }

    /// Sets the database password.
    pub fn with_password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    /// Sets the access token for authentication.
    pub fn with_access_token(mut self, access_token: impl Into<String>) -> Self {
        self.access_token = Some(access_token.into());
        self
    }

    /// Sets the compression method.
    pub fn with_compression(mut self, compression: Compression) -> Self {
        self.compression = Some(compression);
        self
    }

    /// Adds a custom HTTP header.
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Adds a ClickHouse query option.
    pub fn with_option(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.options.push((name.into(), value.into()));
        self
    }

    /// Sets the roles for the connection.
    pub fn with_roles<I>(mut self, roles: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<String>,
    {
        self.roles = Some(roles.into_iter().map(|r| r.into()).collect());
        self
    }

    /// Enables default roles for the connection.
    pub fn with_default_roles(mut self) -> Self {
        self.default_roles = true;
        self
    }

    /// Sets product information for the client.
    pub fn with_product_info(
        mut self,
        product_name: impl Into<String>,
        product_version: impl Into<String>,
    ) -> Self {
        self.product_name = Some(product_name.into());
        self.product_version = Some(product_version.into());
        self
    }

    /// Enables or disables request validation.
    pub fn with_validation(mut self, enabled: bool) -> Self {
        self.validation = enabled;
        self
    }

    /// Builds a `clickhouse::Client` from this builder.
    fn build_client(&self) -> Client {
        let mut client = Client::default();

        // Apply settings that are supported by clickhouse::Client
        if let Some(url) = &self.url {
            client = client.with_url(url.clone());
        }

        if let Some(database) = &self.database {
            client = client.with_database(database.clone());
        }

        if let Some(user) = &self.user {
            client = client.with_user(user.clone());
        }

        if let Some(password) = &self.password {
            client = client.with_password(password.clone());
        }

        if let Some(compression) = self.compression {
            client = client.with_compression(compression);
        }

        for (name, value) in &self.options {
            client = client.with_option(name.clone(), value.clone());
        }

        client
    }
}

/// A connection to ClickHouse with health status tracking.
///
/// This wrapper around `clickhouse::Client` adds tracking of whether the connection
/// is still valid. It implements `Deref` and `DerefMut` so it can be used transparently
/// like a `clickhouse::Client`.
pub struct Connection {
    client: Client,
    is_broken: bool,
}

impl Connection {
    /// Creates a new connection from a client.
    fn new(client: Client) -> Self {
        Self {
            client,
            is_broken: false,
        }
    }

    /// Marks the connection as broken.
    fn mark_broken(&mut self) {
        self.is_broken = true;
    }

    /// Checks if the connection is broken.
    pub fn is_broken(&self) -> bool {
        self.is_broken
    }
}

impl Deref for Connection {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl DerefMut for Connection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.client
    }
}

/// Connection manager for ClickHouse clients that implements BB8's `ManageConnection` trait.
///
/// This manager handles creation, validation, and recycling of ClickHouse client connections.
/// It uses a `ConnectionBuilder` to configure new clients and periodically executes `select 1;`
/// to verify that clients are still valid.
pub struct ConnectionManager {
    builder: ConnectionBuilder,
}

impl ConnectionManager {
    /// Creates a new `ConnectionManager` with the provided builder.
    pub fn new(builder: ConnectionBuilder) -> Self {
        Self { builder }
    }
}

#[async_trait]
impl ManageConnection for ConnectionManager {
    type Connection = Connection;
    type Error = ClickHouseError;

    /// Creates a new connection.
    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        let client = self.builder.build_client();
        Ok(Connection::new(client))
    }

    /// Checks if a connection is still valid.
    async fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        match conn.query("select 1").fetch_optional::<u8>().await {
            Ok(_) => Ok(()),
            Err(e) => {
                conn.mark_broken();
                Err(ClickHouseError::HealthCheckFailed(Box::new(e)))
            }
        }
    }

    /// Returns whether a connection is broken and should be recycled.
    fn has_broken(&self, conn: &mut Self::Connection) -> bool {
        conn.is_broken()
    }
}

/// Type alias for a BB8 pool of ClickHouse connections.
///
/// This is a convenience type that wraps `bb8::Pool<ConnectionManager>` to avoid
/// requiring users to import the BB8 crate directly.
pub type Pool = bb8::Pool<ConnectionManager>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_builder_with_all_options() {
        let builder = ConnectionBuilder::new()
            .with_url("http://localhost:8123")
            .with_database("default")
            .with_user("default")
            .with_password("password")
            .with_access_token("token123")
            .with_compression(Compression::Lz4)
            .with_header("X-Custom", "value")
            .with_option("max_rows_to_read", "1000")
            .with_roles(vec!["role1"])
            .with_default_roles()
            .with_product_info("myapp", "1.0.0")
            .with_validation(true);

        assert_eq!(builder.url, Some("http://localhost:8123".to_string()));
        assert_eq!(builder.database, Some("default".to_string()));
        assert_eq!(builder.user, Some("default".to_string()));
        assert_eq!(builder.password, Some("password".to_string()));
        assert_eq!(builder.access_token, Some("token123".to_string()));
        assert!(builder.compression.is_some());
        assert_eq!(builder.headers.len(), 1);
        assert_eq!(builder.options.len(), 1);
        assert_eq!(builder.roles.as_ref().map(|r| r.len()), Some(1));
        assert!(builder.default_roles);
        assert_eq!(builder.product_name, Some("myapp".to_string()));
        assert_eq!(builder.product_version, Some("1.0.0".to_string()));
        assert!(builder.validation);
    }

    #[test]
    fn test_connection_creation() {
        let client = Client::default();
        let conn = Connection::new(client);
        assert!(!conn.is_broken());
    }

    #[test]
    fn test_connection_mark_broken() {
        let client = Client::default();
        let mut conn = Connection::new(client);
        assert!(!conn.is_broken());

        conn.mark_broken();
        assert!(conn.is_broken());
    }
}
