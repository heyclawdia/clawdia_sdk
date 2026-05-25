//! Provider adapter contract and provider-facing projection DTOs. Hosts implement
//! this port to call model providers after core has projected context and policy-safe
//! metadata. Implementations may perform network I/O and must preserve redaction and
//! stop/usage semantics.
//!
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

/// Port or behavior contract for provider adapter. Implementors should
/// preserve policy, redaction, idempotency, and replay expectations
/// from the surrounding module. Implementations may perform side
/// effects only as described by the trait methods.
pub trait ProviderAdapter: Send + Sync {
    /// Returns adapter capability metadata for policy and package resolution.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    fn capabilities(&self) -> ProviderCapabilities;

    /// Projects admitted context into the provider's request shape.
    /// This projects admitted context into a provider request and must not fetch hidden raw
    /// content.
    fn project_request(
        &self,
        projection: &ContextProjection,
        policy: &ProviderProjectionPolicy,
    ) -> Result<ProviderRequest, AgentError> {
        project_context_projection(projection, policy)
    }

    /// Calls the provider for one non-streaming completion request.
    /// Implementations may call the model provider; caller-owned runtime code must handle
    /// policy, journaling, and event publication around it.
    fn complete(&self, request: &ProviderRequest) -> Result<ProviderResponse, AgentError>;

    /// Calls the provider for a streaming response.
    /// Implementations may call the model provider; caller-owned runtime code must handle
    /// policy, journaling, and event publication around it.
    fn stream(&self, request: &ProviderRequest) -> Result<Vec<ProviderStreamChunk>, AgentError> {
        let response = self.complete(request)?;
        Ok(vec![ProviderStreamChunk::final_text(
            response.output_text.clone(),
            response.stop_reason.clone(),
            response.usage.clone(),
        )])
    }

    /// Extracts provider usage accounting from a response.
    /// This derives usage accounting from a provider response and performs no provider call.
    fn extract_usage(&self, response: &ProviderResponse) -> ProviderUsage {
        response.usage.clone().unwrap_or_default()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries provider capabilities data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ProviderCapabilities {
    /// Typed provider ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub provider_ref: String,
    /// Boolean policy/capability flag for whether supports streaming is
    /// enabled.
    pub supports_streaming: bool,
    /// Boolean policy/capability flag for whether supports usage is enabled.
    pub supports_usage: bool,
    /// Maximum allowed input tokens.
    /// Use it to keep execution, output, or diagnostics bounded.
    pub max_input_tokens: Option<u32>,
    /// Collection of supported modalities values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub supported_modalities: Vec<ProviderModality>,
}

impl ProviderCapabilities {
    /// Returns an updated value with text only configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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
/// Enumerates the finite provider modality cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ProviderModality {
    /// Use this variant when the contract needs to represent text; selecting it has no side effect by itself.
    Text,
    /// Use this variant when the contract needs to represent image; selecting it has no side effect by itself.
    Image,
    /// Use this variant when the contract needs to represent audio; selecting it has no side effect by itself.
    Audio,
    /// Use this variant when the contract needs to represent video; selecting it has no side effect by itself.
    Video,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries provider projection policy data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ProviderProjectionPolicy {
    /// Whether provider projection may include private metadata shell fields.
    /// This does not allow raw private content; raw content still requires explicit resolver
    /// and policy gates.
    pub allow_private_metadata_projection: bool,
    /// Typed projection policy ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub projection_policy_ref: String,
}

impl ProviderProjectionPolicy {
    /// Returns an updated value with redacted configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn redacted(policy_ref: impl Into<String>) -> Self {
        Self {
            allow_private_metadata_projection: false,
            projection_policy_ref: policy_ref.into(),
        }
    }

    /// Returns an updated value with allow private metadata configured.
    /// This is data-only policy/configuration construction and does not call provider adapters,
    /// sinks, journals, or event buses.
    pub fn allow_private_metadata(policy_ref: impl Into<String>) -> Self {
        Self {
            allow_private_metadata_projection: true,
            projection_policy_ref: policy_ref.into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries provider request data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ProviderRequest {
    /// Wire schema version used for compatibility checks.
    pub schema_version: u16,
    /// Typed projection policy ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub projection_policy_ref: String,
    /// Bounded messages included in this record. Limits and truncation are
    /// represented by companion metadata when applicable.
    pub messages: Vec<ProviderMessage>,
    /// Count of projection item items observed or included in this record.
    pub projection_item_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional structured output hint value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub structured_output_hint: Option<ProviderStructuredOutputHint>,
}

impl ProviderRequest {
    /// Constant value for the ports::provider contract. Use it to keep
    /// SDK records and tests aligned on the same stable value.
    pub const SCHEMA_VERSION: u16 = 1;

    /// Returns this value with its structured output hint setting
    /// replaced. The method follows builder-style data construction and
    /// does not execute external work.
    pub fn with_structured_output_hint(mut self, contract: &OutputContract) -> Self {
        self.structured_output_hint = Some(ProviderStructuredOutputHint::from(contract));
        self
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries provider structured output hint data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ProviderStructuredOutputHint {
    /// Stable schema id used for typed lineage, lookup, or dedupe.
    pub schema_id: OutputSchemaId,
    /// Wire schema version used for compatibility checks.
    pub schema_version: SchemaVersion,
    /// Deterministic schema fingerprint used for stale checks, package
    /// evidence, or replay comparisons.
    pub schema_fingerprint: ContentHash,
    /// Policy for provider-side structured-output hints.
    /// Hints may guide prompting but cannot replace SDK-owned validation.
    pub provider_hint_policy: ProviderHintPolicy,
    /// Typed include schema ref reference. Resolving or executing it is a
    /// separate policy-gated step.
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
/// Carries provider message data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ProviderMessage {
    /// Role used by this record or request.
    pub role: ProviderMessageRole,
    /// Bounded textual content extracted for caller use; absent for binary
    /// summaries or denied raw access.
    pub content: String,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy: PrivacyClass,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional projected metadata value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub projected_metadata: Option<ProviderProjectedMetadata>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite provider message role cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ProviderMessageRole {
    /// Use this variant when the contract needs to represent system; selecting it has no side effect by itself.
    System,
    /// Use this variant when the contract needs to represent developer; selecting it has no side effect by itself.
    Developer,
    /// Use this variant when the contract needs to represent user; selecting it has no side effect by itself.
    User,
    /// Use this variant when the contract needs to represent assistant; selecting it has no side effect by itself.
    Assistant,
    /// Use this variant when the contract needs to represent tool; selecting it has no side effect by itself.
    Tool,
    /// Use this variant when the contract needs to represent context; selecting it has no side effect by itself.
    Context,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries provider projected metadata data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ProviderProjectedMetadata {
    /// Kind discriminator for source kind.
    /// Use it to route finite match arms without parsing display text.
    pub source_kind: SourceKind,
    /// Stable source id used for typed lineage, lookup, or dedupe.
    pub source_id: String,
    /// Kind discriminator for destination kind.
    /// Use it to route finite match arms without parsing display text.
    pub destination_kind: DestinationKind,
    /// Stable destination id used for typed lineage, lookup, or dedupe.
    pub destination_id: String,
    /// Kind discriminator for subject kind.
    /// Use it to route finite match arms without parsing display text.
    pub subject_kind: String,
    /// Stable subject id used for typed lineage, lookup, or dedupe.
    pub subject_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries provider response data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ProviderResponse {
    /// Wire schema version used for compatibility checks.
    pub schema_version: u16,
    /// Output text used by this record or request.
    pub output_text: String,
    /// Stop reason used by this record or request.
    pub stop_reason: ProviderStopReason,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional usage value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub usage: Option<ProviderUsage>,
}

impl ProviderResponse {
    /// Constant value for the ports::provider contract. Use it to keep
    /// SDK records and tests aligned on the same stable value.
    pub const SCHEMA_VERSION: u16 = 1;

    /// Builds the text value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn text(output_text: impl Into<String>) -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION,
            output_text: output_text.into(),
            stop_reason: ProviderStopReason::EndTurn,
            usage: None,
        }
    }

    /// Returns this value with its usage setting replaced. The method
    /// follows builder-style data construction and does not execute
    /// external work.
    pub fn with_usage(mut self, usage: ProviderUsage) -> Self {
        self.usage = Some(usage);
        self
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite provider stop reason cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ProviderStopReason {
    /// Use this variant when the contract needs to represent end turn; selecting it has no side effect by itself.
    EndTurn,
    /// Use this variant when the contract needs to represent max tokens; selecting it has no side effect by itself.
    MaxTokens,
    /// Use this variant when the contract needs to represent cancelled; selecting it has no side effect by itself.
    Cancelled,
    /// Use this variant when the contract needs to represent provider error; selecting it has no side effect by itself.
    ProviderError,
    /// Use this variant when the contract needs to represent unknown; selecting it has no side effect by itself.
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries provider stream chunk data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ProviderStreamChunk {
    /// Wire schema version used for compatibility checks.
    pub schema_version: u16,
    /// Chunk index used by this record or request.
    pub chunk_index: u32,
    /// Delta used by this record or request.
    pub delta: ProviderStreamDelta,
    /// Whether is terminal is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub is_terminal: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional usage value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub usage: Option<ProviderUsage>,
}

impl ProviderStreamChunk {
    /// Constant value for the ports::provider contract. Use it to keep
    /// SDK records and tests aligned on the same stable value.
    pub const SCHEMA_VERSION: u16 = 1;

    /// Builds the text value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
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

    /// Builds the final text value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
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
/// Enumerates the finite provider stream delta cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ProviderStreamDelta {
    /// Use this variant when the contract needs to represent text; selecting it has no side effect by itself.
    Text {
        /// Text used by this record or request.
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        /// Optional stop reason value.
        /// When absent, callers should use the documented default or skip that optional
        /// behavior.
        stop_reason: Option<ProviderStopReason>,
    },
    /// Use this variant when the contract needs to represent usage; selecting it has no side effect by itself.
    Usage {
        /// Usage used by this record or request.
        usage: ProviderUsage,
    },
    /// Provider stream or transport error. The payload must stay
    /// redacted so live event observers do not require raw content.
    Error {
        /// Redacted human-readable summary safe for events, telemetry, and
        /// logs.
        redacted_summary: String,
    },
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
/// Carries provider usage data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ProviderUsage {
    /// Optional input tokens value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub input_tokens: Option<u32>,
    /// Optional output tokens value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub output_tokens: Option<u32>,
    /// Optional total tokens value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub total_tokens: Option<u32>,
}

#[derive(Clone, Debug)]
/// Carries provider conformance case data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ProviderConformanceCase {
    /// Projection controls for exposing data to a provider or subscriber.
    /// Use it to keep provider-visible data separate from private SDK state.
    pub projection: ContextProjection,
    /// Policy used by this record or request.
    pub policy: ProviderProjectionPolicy,
}

impl ProviderConformanceCase {
    /// Creates a new ports::provider value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new(projection: ContextProjection) -> Self {
        Self {
            projection,
            policy: ProviderProjectionPolicy::redacted("policy.provider.redacted"),
        }
    }

    /// Builds the assert adapter value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
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
