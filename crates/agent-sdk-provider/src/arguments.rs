use agent_sdk_core::{
    AgentError, ProviderArgumentStore, domain::ContentRef as ContentRefId,
    tool_records::CanonicalToolName,
};

/// Optional host-owned sink for raw provider tool-call arguments.
///
/// Adapters call this before returning `ProviderToolCall` so raw arguments can
/// stay behind content-policy resolution. Implementations must avoid logging or
/// journaling raw arguments directly.
pub trait ProviderToolArgumentSink: Send + Sync {
    /// Stores raw provider tool-call arguments and returns a content ref when
    /// executors should resolve arguments through normal content policy.
    fn store_tool_arguments(
        &self,
        provider_ref: &str,
        call_id: &str,
        canonical_tool_name: &CanonicalToolName,
        raw_arguments: &str,
    ) -> Result<Option<ContentRefId>, AgentError>;
}

impl<T> ProviderToolArgumentSink for T
where
    T: ProviderArgumentStore,
{
    fn store_tool_arguments(
        &self,
        provider_ref: &str,
        call_id: &str,
        canonical_tool_name: &CanonicalToolName,
        raw_arguments: &str,
    ) -> Result<Option<ContentRefId>, AgentError> {
        self.store_provider_arguments(provider_ref, call_id, canonical_tool_name, raw_arguments)
    }
}
