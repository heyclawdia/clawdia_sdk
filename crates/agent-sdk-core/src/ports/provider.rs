use serde::{Deserialize, Serialize};

use crate::{
    context::ContextProjection,
    domain::{
        AgentError, AgentErrorKind, DestinationKind, OutputSchemaId, PrivacyClass,
        RetryClassification, SourceKind,
    },
    output::{ContentHash, OutputContract, ProviderHintPolicy, SchemaVersion},
    projection::project_context_projection,
};

pub trait ProviderAdapter: Send + Sync {
    fn capabilities(&self) -> ProviderCapabilities;

    fn project_request(
        &self,
        projection: &ContextProjection,
        policy: &ProviderProjectionPolicy,
    ) -> Result<ProviderRequest, AgentError> {
        project_context_projection(projection, policy)
    }

    fn complete(&self, request: &ProviderRequest) -> Result<ProviderResponse, AgentError>;

    fn stream(&self, request: &ProviderRequest) -> Result<Vec<ProviderStreamChunk>, AgentError> {
        let response = self.complete(request)?;
        Ok(vec![ProviderStreamChunk::final_text(
            response.output_text.clone(),
            response.stop_reason.clone(),
            response.usage.clone(),
        )])
    }

    fn extract_usage(&self, response: &ProviderResponse) -> ProviderUsage {
        response.usage.clone().unwrap_or_default()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProviderCapabilities {
    pub provider_ref: String,
    pub supports_streaming: bool,
    pub supports_usage: bool,
    pub max_input_tokens: Option<u32>,
    pub supported_modalities: Vec<ProviderModality>,
}

impl ProviderCapabilities {
    pub fn text_only(provider_ref: impl Into<String>) -> Self {
        Self {
            provider_ref: provider_ref.into(),
            supports_streaming: false,
            supports_usage: true,
            max_input_tokens: None,
            supported_modalities: vec![ProviderModality::Text],
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderModality {
    Text,
    Image,
    Audio,
    Video,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProviderProjectionPolicy {
    pub allow_private_metadata_projection: bool,
    pub projection_policy_ref: String,
}

impl ProviderProjectionPolicy {
    pub fn redacted(policy_ref: impl Into<String>) -> Self {
        Self {
            allow_private_metadata_projection: false,
            projection_policy_ref: policy_ref.into(),
        }
    }

    pub fn allow_private_metadata(policy_ref: impl Into<String>) -> Self {
        Self {
            allow_private_metadata_projection: true,
            projection_policy_ref: policy_ref.into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProviderRequest {
    pub schema_version: u16,
    pub projection_policy_ref: String,
    pub messages: Vec<ProviderMessage>,
    pub projection_item_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_output_hint: Option<ProviderStructuredOutputHint>,
}

impl ProviderRequest {
    pub const SCHEMA_VERSION: u16 = 1;

    pub fn with_structured_output_hint(mut self, contract: &OutputContract) -> Self {
        self.structured_output_hint = Some(ProviderStructuredOutputHint::from(contract));
        self
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProviderStructuredOutputHint {
    pub schema_id: OutputSchemaId,
    pub schema_version: SchemaVersion,
    pub schema_fingerprint: ContentHash,
    pub provider_hint_policy: ProviderHintPolicy,
    pub include_schema_ref: bool,
}

impl From<&OutputContract> for ProviderStructuredOutputHint {
    fn from(contract: &OutputContract) -> Self {
        Self {
            schema_id: contract.schema_id.clone(),
            schema_version: contract.schema_version,
            schema_fingerprint: contract.schema_fingerprint(),
            provider_hint_policy: contract.projection_hint.provider_hint_policy.clone(),
            include_schema_ref: contract.projection_hint.include_schema_ref,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProviderMessage {
    pub role: ProviderMessageRole,
    pub content: String,
    pub privacy: PrivacyClass,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub projected_metadata: Option<ProviderProjectedMetadata>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderMessageRole {
    System,
    Developer,
    User,
    Assistant,
    Tool,
    Context,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProviderProjectedMetadata {
    pub source_kind: SourceKind,
    pub source_id: String,
    pub destination_kind: DestinationKind,
    pub destination_id: String,
    pub subject_kind: String,
    pub subject_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProviderResponse {
    pub schema_version: u16,
    pub output_text: String,
    pub stop_reason: ProviderStopReason,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<ProviderUsage>,
}

impl ProviderResponse {
    pub const SCHEMA_VERSION: u16 = 1;

    pub fn text(output_text: impl Into<String>) -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION,
            output_text: output_text.into(),
            stop_reason: ProviderStopReason::EndTurn,
            usage: None,
        }
    }

    pub fn with_usage(mut self, usage: ProviderUsage) -> Self {
        self.usage = Some(usage);
        self
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderStopReason {
    EndTurn,
    MaxTokens,
    Cancelled,
    ProviderError,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProviderStreamChunk {
    pub schema_version: u16,
    pub chunk_index: u32,
    pub delta: ProviderStreamDelta,
    pub is_terminal: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<ProviderUsage>,
}

impl ProviderStreamChunk {
    pub const SCHEMA_VERSION: u16 = 1;

    pub fn text(chunk_index: u32, text: impl Into<String>) -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION,
            chunk_index,
            delta: ProviderStreamDelta::Text {
                text: text.into(),
                stop_reason: None,
            },
            is_terminal: false,
            usage: None,
        }
    }

    pub fn final_text(
        text: impl Into<String>,
        stop_reason: ProviderStopReason,
        usage: Option<ProviderUsage>,
    ) -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION,
            chunk_index: 0,
            delta: ProviderStreamDelta::Text {
                text: text.into(),
                stop_reason: Some(stop_reason),
            },
            is_terminal: true,
            usage,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProviderStreamDelta {
    Text {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        stop_reason: Option<ProviderStopReason>,
    },
    Usage {
        usage: ProviderUsage,
    },
    Error {
        redacted_summary: String,
    },
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProviderUsage {
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
}

#[derive(Clone, Debug)]
pub struct ProviderConformanceCase {
    pub projection: ContextProjection,
    pub policy: ProviderProjectionPolicy,
}

impl ProviderConformanceCase {
    pub fn new(projection: ContextProjection) -> Self {
        Self {
            projection,
            policy: ProviderProjectionPolicy::redacted("policy.provider.redacted"),
        }
    }

    pub fn assert_adapter<A: ProviderAdapter>(
        &self,
        adapter: &A,
    ) -> Result<ProviderUsage, AgentError> {
        let capabilities = adapter.capabilities();
        if capabilities.provider_ref.is_empty() {
            return Err(AgentError::new(
                AgentErrorKind::ProviderFailure,
                RetryClassification::HostConfigurationNeeded,
                "provider capabilities must name a provider ref",
            ));
        }

        let request = adapter.project_request(&self.projection, &self.policy)?;
        if request.projection_item_count != self.projection.items.len() {
            return Err(AgentError::new(
                AgentErrorKind::ProjectionFailure,
                RetryClassification::RepairNeeded,
                "provider request item count must match the context projection",
            ));
        }

        let response = adapter.complete(&request)?;
        Ok(adapter.extract_usage(&response))
    }
}
