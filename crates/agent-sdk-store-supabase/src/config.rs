use agent_sdk_core::AgentError;

use crate::auth::SupabaseAuth;

#[derive(Clone, Debug)]
/// Supabase store connection and namespace configuration.
pub struct SupabaseStoreConfig {
    project_url: String,
    schema: String,
    store_scope: String,
    auth: SupabaseAuth,
}

impl SupabaseStoreConfig {
    /// Creates a Supabase store configuration.
    pub fn new(
        project_url: impl Into<String>,
        schema: impl Into<String>,
        store_scope: impl Into<String>,
        auth: SupabaseAuth,
    ) -> Result<Self, AgentError> {
        let project_url = project_url.into().trim_end_matches('/').to_string();
        if project_url.is_empty() {
            return Err(AgentError::missing_required_field("supabase.project_url"));
        }
        let schema = schema.into();
        if schema.is_empty() {
            return Err(AgentError::missing_required_field("supabase.schema"));
        }
        let store_scope = store_scope.into();
        if store_scope.is_empty() {
            return Err(AgentError::missing_required_field("supabase.store_scope"));
        }
        Ok(Self {
            project_url,
            schema,
            store_scope,
            auth,
        })
    }

    /// Returns the Supabase project URL without a trailing slash.
    pub fn project_url(&self) -> &str {
        &self.project_url
    }

    /// Returns the PostgREST schema/profile.
    pub fn schema(&self) -> &str {
        &self.schema
    }

    /// Returns the logical SDK store partition.
    pub fn store_scope(&self) -> &str {
        &self.store_scope
    }

    /// Returns the redacted auth holder.
    pub fn auth(&self) -> &SupabaseAuth {
        &self.auth
    }
}
