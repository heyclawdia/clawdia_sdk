use std::sync::Arc;

use agent_sdk_core::AgentError;
use serde_json::Value;

#[derive(Clone, Debug, Eq, PartialEq)]
/// SQL request sent through the host-owned Postgres transport.
pub struct PostgresSqlRequest {
    /// SQL statement or prepared statement name.
    pub statement: String,
    /// Bound parameters as JSON for deterministic scripted tests.
    pub params: Vec<Value>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
/// SQL response returned by the host-owned Postgres transport.
pub struct PostgresSqlResponse {
    /// Rows represented as JSON objects.
    pub rows: Vec<Value>,
    /// Number of affected rows.
    pub affected: u64,
}

impl PostgresSqlResponse {
    /// Creates a response with rows.
    pub fn rows(rows: impl IntoIterator<Item = Value>) -> Self {
        Self {
            rows: rows.into_iter().collect(),
            affected: 0,
        }
    }

    /// Creates an affected-row response.
    pub fn affected(affected: u64) -> Self {
        Self {
            rows: Vec::new(),
            affected,
        }
    }
}

/// Host-owned SQL transport for Postgres-style stores.
pub trait PostgresSqlTransport: Send + Sync {
    /// Executes a SQL request. Implementations may talk to a database; tests can script responses.
    fn execute(&self, request: PostgresSqlRequest) -> Result<PostgresSqlResponse, AgentError>;
}

#[derive(Clone, Debug)]
/// Postgres store configuration.
pub struct PostgresStoreConfig {
    /// SQL schema name used in generated statements.
    pub schema: String,
    /// Store scope used to partition rows.
    pub store_scope: String,
}

impl PostgresStoreConfig {
    /// Creates a Postgres store config.
    pub fn new(schema: impl Into<String>, store_scope: impl Into<String>) -> Self {
        Self {
            schema: schema.into(),
            store_scope: store_scope.into(),
        }
    }
}

#[derive(Clone)]
/// Shared Postgres client over a host-owned SQL transport.
pub struct PostgresStoreClient {
    pub(crate) config: PostgresStoreConfig,
    transport: Arc<dyn PostgresSqlTransport>,
}

impl PostgresStoreClient {
    /// Creates a client over a scripted or host-provided transport.
    pub fn new(config: PostgresStoreConfig, transport: Arc<dyn PostgresSqlTransport>) -> Self {
        Self { config, transport }
    }

    pub(crate) fn execute(
        &self,
        statement: impl Into<String>,
        params: Vec<Value>,
    ) -> Result<PostgresSqlResponse, AgentError> {
        self.transport.execute(PostgresSqlRequest {
            statement: statement.into(),
            params,
        })
    }

    pub(crate) fn table(&self, table: &str) -> String {
        format!("{}.{}", self.config.schema, table)
    }

    pub(crate) fn scope(&self) -> Value {
        Value::String(self.config.store_scope.clone())
    }
}
