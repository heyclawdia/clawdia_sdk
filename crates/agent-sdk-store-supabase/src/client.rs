use std::sync::Arc;

use agent_sdk_core::AgentError;
use serde::Serialize;

use crate::{
    config::SupabaseStoreConfig,
    transport::{SupabaseHttpRequest, SupabaseHttpResponse, SupabaseHttpTransport, supabase_error},
};

#[derive(Clone)]
/// Supabase REST client for SDK store adapters.
pub struct SupabaseClient {
    config: SupabaseStoreConfig,
    transport: Arc<dyn SupabaseHttpTransport>,
}

impl SupabaseClient {
    /// Creates a Supabase client from config and transport.
    pub fn new<T>(config: SupabaseStoreConfig, transport: T) -> Self
    where
        T: SupabaseHttpTransport + 'static,
    {
        Self {
            config,
            transport: Arc::new(transport),
        }
    }

    /// Creates a Supabase client with a shared transport.
    pub fn with_shared_transport(
        config: SupabaseStoreConfig,
        transport: Arc<dyn SupabaseHttpTransport>,
    ) -> Self {
        Self { config, transport }
    }

    /// Returns the client config.
    pub fn config(&self) -> &SupabaseStoreConfig {
        &self.config
    }

    /// Sends an RPC call through Supabase PostgREST.
    pub fn rpc<T>(&self, function_name: &str, body: &T) -> Result<SupabaseHttpResponse, AgentError>
    where
        T: Serialize,
    {
        let request = self.request(
            "POST",
            &format!("/rest/v1/rpc/{function_name}"),
            Some(serde_json::to_vec(body).map_err(|error| supabase_error(error.to_string()))?),
        );
        self.transport.send(request)
    }

    /// Sends a table insert request through Supabase PostgREST.
    pub fn insert<T>(&self, table: &str, body: &T) -> Result<SupabaseHttpResponse, AgentError>
    where
        T: Serialize,
    {
        let request = self.request(
            "POST",
            &format!("/rest/v1/{table}"),
            Some(serde_json::to_vec(body).map_err(|error| supabase_error(error.to_string()))?),
        );
        self.transport.send(request)
    }

    /// Sends a table select request through Supabase PostgREST.
    pub fn select(&self, table: &str, query: &str) -> Result<SupabaseHttpResponse, AgentError> {
        let request = self.request("GET", &format!("/rest/v1/{table}?{query}"), None);
        self.transport.send(request)
    }

    fn request(&self, method: &str, path: &str, body: Option<Vec<u8>>) -> SupabaseHttpRequest {
        SupabaseHttpRequest {
            method: method.to_string(),
            url: format!("{}{}", self.config.project_url(), path),
            headers: vec![
                (
                    "apikey".to_string(),
                    self.config.auth().api_key().to_string(),
                ),
                (
                    "Authorization".to_string(),
                    format!("Bearer {}", self.config.auth().bearer_token()),
                ),
                ("Accept".to_string(), "application/json".to_string()),
                ("Content-Type".to_string(), "application/json".to_string()),
                (
                    "Accept-Profile".to_string(),
                    self.config.schema().to_string(),
                ),
                (
                    "Content-Profile".to_string(),
                    self.config.schema().to_string(),
                ),
            ],
            body,
        }
    }
}
