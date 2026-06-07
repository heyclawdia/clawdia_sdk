use agent_sdk_core::{
    AgentError, ProviderArgumentStore, domain::ContentRef, tool_records::CanonicalToolName,
};
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::{client::SupabaseClient, transport::supabase_error};

#[derive(Clone)]
/// Supabase-backed raw provider argument store.
pub struct SupabaseProviderArgumentStore {
    client: SupabaseClient,
}

impl SupabaseProviderArgumentStore {
    pub fn new(client: SupabaseClient) -> Self {
        Self { client }
    }
}

impl ProviderArgumentStore for SupabaseProviderArgumentStore {
    fn store_provider_arguments(
        &self,
        provider_ref: &str,
        call_id: &str,
        canonical_tool_name: &CanonicalToolName,
        raw_arguments: &str,
    ) -> Result<Option<ContentRef>, AgentError> {
        let digest = format!("{:x}", Sha256::digest(raw_arguments.as_bytes()));
        let content_ref = ContentRef::new(format!("content.provider_arguments.{}", &digest[..24]));
        let response = self.client.insert(
            "agent_sdk_provider_arguments",
            &json!({
                "store_scope": self.client.config().store_scope(),
                "provider_ref": provider_ref,
                "call_id": call_id,
                "canonical_tool_name": canonical_tool_name.as_str(),
                "content_ref": content_ref.as_str(),
                "raw_arguments": raw_arguments,
                "sha256": digest,
            }),
        )?;
        if !(200..300).contains(&response.status) {
            return Err(supabase_error(format!(
                "supabase provider argument insert failed with status {}",
                response.status
            )));
        }
        Ok(Some(content_ref))
    }

    fn load_provider_arguments_json(
        &self,
        content_ref: &ContentRef,
    ) -> Result<serde_json::Value, AgentError> {
        let query = format!(
            "store_scope=eq.{}&content_ref=eq.{}&select=raw_arguments&limit=1",
            self.client.config().store_scope(),
            content_ref.as_str()
        );
        let response = self.client.select("agent_sdk_provider_arguments", &query)?;
        if !(200..300).contains(&response.status) {
            return Err(supabase_error(format!(
                "supabase provider argument read failed with status {}",
                response.status
            )));
        }
        let rows = serde_json::from_slice::<Vec<serde_json::Value>>(&response.body)
            .map_err(|error| supabase_error(error.to_string()))?;
        let raw_arguments = rows
            .first()
            .and_then(|row| row["raw_arguments"].as_str())
            .ok_or_else(|| supabase_error("supabase provider argument content ref missing"))?;
        serde_json::from_str(raw_arguments).map_err(|error| {
            AgentError::contract_violation(format!(
                "stored provider arguments are not valid JSON: {error}"
            ))
        })
    }
}
