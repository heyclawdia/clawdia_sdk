use agent_sdk_core::{
    AgentError, ProviderArgumentStore, domain::ContentRef as ProviderArgumentContentRef,
    tool_records::CanonicalToolName,
};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::PostgresStoreClient;

#[derive(Clone)]
pub struct PostgresProviderArgumentStore {
    client: PostgresStoreClient,
}

impl PostgresProviderArgumentStore {
    pub fn new(client: PostgresStoreClient) -> Self {
        Self { client }
    }
}

impl ProviderArgumentStore for PostgresProviderArgumentStore {
    fn store_provider_arguments(
        &self,
        provider_ref: &str,
        call_id: &str,
        canonical_tool_name: &CanonicalToolName,
        raw_arguments: &str,
    ) -> Result<Option<ProviderArgumentContentRef>, AgentError> {
        let digest = format!("{:x}", Sha256::digest(raw_arguments.as_bytes()));
        let content_ref = ProviderArgumentContentRef::new(format!(
            "content.provider_arguments.{}",
            &digest[..24]
        ));
        self.client.execute(
            format!("insert into {} (store_scope, provider_ref, call_id, canonical_tool_name, raw_arguments, content_ref) values ($1, $2, $3, $4, $5, $6) on conflict (store_scope, content_ref) do update set provider_ref = excluded.provider_ref, call_id = excluded.call_id, canonical_tool_name = excluded.canonical_tool_name, raw_arguments = excluded.raw_arguments", self.client.table("agent_sdk_provider_arguments")),
            vec![
                self.client.scope(),
                Value::String(provider_ref.to_string()),
                Value::String(call_id.to_string()),
                Value::String(canonical_tool_name.as_str().to_string()),
                Value::String(raw_arguments.to_string()),
                Value::String(content_ref.as_str().to_string()),
            ],
        )?;
        Ok(Some(content_ref))
    }

    fn load_provider_arguments_json(
        &self,
        content_ref: &ProviderArgumentContentRef,
    ) -> Result<Value, AgentError> {
        let response = self.client.execute(
            format!(
                "select raw_arguments from {} where store_scope = $1 and content_ref = $2",
                self.client.table("agent_sdk_provider_arguments")
            ),
            vec![
                self.client.scope(),
                Value::String(content_ref.as_str().to_string()),
            ],
        )?;
        let raw = response
            .rows
            .first()
            .and_then(|row| row.get("raw_arguments"))
            .and_then(Value::as_str)
            .ok_or_else(|| {
                AgentError::contract_violation("provider argument content ref is missing")
            })?;
        serde_json::from_str(raw).map_err(|error| {
            AgentError::contract_violation(format!(
                "stored provider arguments are not valid JSON: {error}"
            ))
        })
    }
}
