//! Provider adapter contract and provider-facing projection DTOs. Hosts implement
//! this port to call model providers after core has projected context and policy-safe
//! metadata. Implementations may perform network I/O and must preserve redaction and
//! stop/usage semantics.
//!
use core::fmt;

use serde::{Deserialize, Serialize};

use crate::{
    capability::{CapabilityId, CapabilityNamespace, PackageSidecarRef},
    context::ContextProjection,
    domain::{
        AgentError, AgentErrorKind, ContentRef, DestinationKind, OutputSchemaId, PolicyRef,
        PrivacyClass, RetryClassification, SourceKind, ToolCallId,
    },
    output::{ContentHash, OutputContract, OutputSchemaRef, ProviderHintPolicy, SchemaVersion},
    projection::project_context_projection,
    tool_records::CanonicalToolName,
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
        if response.tool_calls.is_empty() {
            return Ok(vec![ProviderStreamChunk::final_text(
                response.output_text.clone(),
                response.stop_reason.clone(),
                response.usage.clone(),
            )]);
        }

        let mut chunks = Vec::new();
        let mut chunk_index = 0;
        if !response.output_text.is_empty() {
            chunks.push(ProviderStreamChunk::text(
                chunk_index,
                response.output_text.clone(),
            ));
            chunk_index += 1;
        }
        chunks.push(ProviderStreamChunk::final_tool_calls(
            chunk_index,
            response.tool_calls.clone(),
            response.stop_reason.clone(),
            response.usage.clone(),
        ));
        Ok(chunks)
    }

    /// Extracts provider usage accounting from a response.
    /// This derives usage accounting from a provider response and performs no provider call.
    fn extract_usage(&self, response: &ProviderResponse) -> ProviderUsage {
        response.usage.clone().unwrap_or_default()
    }
}

/// Read/write store contract for raw provider tool-call arguments.
///
/// Implementations must keep raw arguments out of journals, events, summaries,
/// and debug output, returning content refs for later policy-checked access.
pub trait ProviderArgumentStore: Send + Sync {
    /// Stores raw provider tool arguments and returns a content ref when the
    /// host wants executors to resolve arguments through normal content policy.
    fn store_provider_arguments(
        &self,
        provider_ref: &str,
        call_id: &str,
        canonical_tool_name: &CanonicalToolName,
        raw_arguments: &str,
    ) -> Result<Option<ContentRef>, AgentError>;

    /// Loads stored provider tool arguments as JSON through the same content
    /// ref returned by `store_provider_arguments`.
    fn load_provider_arguments_json(
        &self,
        content_ref: &ContentRef,
    ) -> Result<serde_json::Value, AgentError>;
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

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Provider-visible tool declarations projected from the effective runtime package.
    /// These are declarations only: provider adapters may render them into native
    /// function/tool shapes, but core remains authoritative for routing, approval,
    /// execution, journaling, and redaction.
    pub tools: Vec<ProviderToolSpec>,
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

    /// Returns this value with provider-visible tool declarations attached.
    /// This is data-only projection; it does not execute tools or resolve schema refs.
    pub fn with_tools(mut self, tools: impl IntoIterator<Item = ProviderToolSpec>) -> Self {
        self.tools = tools.into_iter().collect();
        self
    }
}

impl fmt::Debug for ProviderRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderRequest")
            .field("schema_version", &self.schema_version)
            .field("projection_policy_ref", &self.projection_policy_ref)
            .field("message_count", &self.messages.len())
            .field("messages", &"<redacted>")
            .field("projection_item_count", &self.projection_item_count)
            .field("structured_output_hint", &self.structured_output_hint)
            .field("tool_count", &self.tools.len())
            .field("tools", &"<redacted>")
            .finish()
    }
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
/// Provider-visible tool declaration projected from runtime-package state.
///
/// This DTO is safe for provider adapters because it carries only stable names,
/// refs, policy identifiers, and optional redacted schema bodies. Raw tool
/// arguments and executable handles never appear here.
pub struct ProviderToolSpec {
    /// Provider-visible canonical function/tool name.
    pub name: String,
    /// Stable capability id that owns this provider-visible declaration.
    pub capability_id: CapabilityId,
    /// Capability namespace carried for lineage and collision debugging.
    pub namespace: CapabilityNamespace,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Bounded provider-visible tool description from the runtime package.
    ///
    /// This is projection metadata only. It does not affect tool execution,
    /// approval, policy, or runtime-package identity.
    pub description: Option<String>,
    /// Tool input schema ref. Resolving it is separate from constructing a
    /// provider request.
    pub schema_ref: PackageSidecarRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Policy refs that must still be evaluated by core before execution.
    pub policy_refs: Vec<PolicyRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Redacted inline schema body, when a package helper has made one safe for
    /// provider projection. Adapters must use a bounded fallback when absent.
    pub redacted_schema: Option<serde_json::Value>,
}

impl ProviderToolSpec {
    /// Builds a provider tool declaration from package and runtime routing data.
    /// This is projection only and performs no provider call or tool execution.
    pub fn new(
        name: impl Into<String>,
        capability_id: CapabilityId,
        namespace: CapabilityNamespace,
        schema_ref: PackageSidecarRef,
        policy_refs: Vec<PolicyRef>,
    ) -> Self {
        Self {
            name: name.into(),
            capability_id,
            namespace,
            description: None,
            schema_ref,
            policy_refs,
            redacted_schema: None,
        }
    }

    /// Returns this declaration with a provider-visible description attached.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        let description = description.into();
        if !description.trim().is_empty() {
            self.description = Some(description);
        }
        self
    }

    /// Returns this declaration with an inline redacted schema attached.
    /// The caller is responsible for supplying only provider-safe schema data.
    pub fn with_redacted_schema(mut self, schema: serde_json::Value) -> Self {
        self.redacted_schema = Some(schema);
        self
    }

    /// Returns a provider-safe JSON schema body. If no inline schema body is
    /// present, the fallback is an object schema with SDK schema-ref metadata.
    pub fn provider_schema(&self) -> serde_json::Value {
        self.redacted_schema.clone().unwrap_or_else(|| {
            serde_json::json!({
                "type": "object",
                "additionalProperties": true,
                "x-agent-sdk-schema-ref": self.schema_ref.sidecar_id,
                "x-agent-sdk-schema-kind": self.schema_ref.kind,
                "x-agent-sdk-schema-version": self.schema_ref.version,
                "x-agent-sdk-schema-content-hash": self.schema_ref.content_hash,
            })
        })
    }
}

impl fmt::Debug for ProviderToolSpec {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderToolSpec")
            .field("name", &self.name)
            .field("capability_id", &self.capability_id)
            .field("namespace", &self.namespace)
            .field("description_present", &self.description.is_some())
            .field("schema_ref", &self.schema_ref)
            .field("policy_ref_count", &self.policy_refs.len())
            .field("redacted_schema_present", &self.redacted_schema.is_some())
            .finish()
    }
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Optional redacted inline schema body available for provider-native
    /// structured-output hints. This is only populated when the output
    /// contract already carries an inline schema safe for provider projection;
    /// SDK-owned validation remains authoritative.
    pub redacted_schema: Option<serde_json::Value>,
}

impl From<&OutputContract> for ProviderStructuredOutputHint {
    fn from(contract: &OutputContract) -> Self {
        let redacted_schema = match &contract.schema {
            OutputSchemaRef::InlineJson {
                redacted_schema, ..
            } => Some(redacted_schema.clone()),
            _ => None,
        };
        Self {
            schema_id: contract.schema_id.clone(),
            schema_version: contract.schema_version,
            schema_fingerprint: contract.schema_fingerprint(),
            provider_hint_policy: contract.projection_hint.provider_hint_policy.clone(),
            include_schema_ref: contract.projection_hint.include_schema_ref,
            redacted_schema,
        }
    }
}

impl fmt::Debug for ProviderStructuredOutputHint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderStructuredOutputHint")
            .field("schema_id", &self.schema_id)
            .field("schema_version", &self.schema_version)
            .field("schema_fingerprint", &self.schema_fingerprint)
            .field("provider_hint_policy", &self.provider_hint_policy)
            .field("include_schema_ref", &self.include_schema_ref)
            .field("redacted_schema_present", &self.redacted_schema.is_some())
            .finish()
    }
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
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

impl fmt::Debug for ProviderMessage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderMessage")
            .field("role", &self.role)
            .field("content", &"<redacted>")
            .field("content_chars", &self.content.chars().count())
            .field("privacy", &self.privacy)
            .field("projected_metadata", &self.projected_metadata)
            .finish()
    }
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

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
/// Carries provider response data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ProviderResponse {
    /// Wire schema version used for compatibility checks.
    pub schema_version: u16,
    /// Output text used by this record or request.
    pub output_text: String,
    /// Stop reason used by this record or request.
    pub stop_reason: ProviderStopReason,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Provider-requested tool calls. These are model-visible tool call
    /// requests only; execution still must pass through the SDK tool router,
    /// policy, journal intent/result, and effect contracts.
    pub tool_calls: Vec<ProviderToolCall>,
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
            tool_calls: Vec::new(),
            usage: None,
        }
    }

    /// Builds a provider response that requests tool execution through
    /// canonical SDK tool-call material. This does not execute any tool;
    /// runtime code must lower these requests through `ToolRouter`,
    /// policy, journal, and effect contracts before calling an executor.
    pub fn tool_use(tool_calls: impl IntoIterator<Item = ProviderToolCall>) -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION,
            output_text: String::new(),
            stop_reason: ProviderStopReason::ToolUse,
            tool_calls: tool_calls.into_iter().collect(),
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

impl fmt::Debug for ProviderResponse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderResponse")
            .field("schema_version", &self.schema_version)
            .field("output_text", &"<redacted>")
            .field("output_text_chars", &self.output_text.chars().count())
            .field("stop_reason", &self.stop_reason)
            .field("tool_calls", &self.tool_calls)
            .field("usage", &self.usage)
            .finish()
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
    /// Use this variant when the contract needs to represent a provider
    /// request for tool execution. Selecting it does not execute a tool;
    /// runtime code must route tool calls through SDK policy, journal, and
    /// effect contracts.
    ToolUse,
    /// Use this variant when the contract needs to represent cancelled; selecting it has no side effect by itself.
    Cancelled,
    /// Use this variant when the contract needs to represent provider error; selecting it has no side effect by itself.
    ProviderError,
    /// Use this variant when the contract needs to represent unknown; selecting it has no side effect by itself.
    Unknown,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
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

    /// Builds a tool-call delta. This is provider output data only and
    /// does not route, approve, journal, or execute tools.
    pub fn tool_calls(
        chunk_index: u32,
        tool_calls: impl IntoIterator<Item = ProviderToolCall>,
    ) -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION,
            chunk_index,
            delta: ProviderStreamDelta::ToolCalls {
                tool_calls: tool_calls.into_iter().collect(),
                stop_reason: None,
            },
            is_terminal: false,
            usage: None,
        }
    }

    /// Builds a terminal tool-call delta. This is provider output data
    /// only and does not route, approve, journal, or execute tools.
    pub fn final_tool_calls(
        chunk_index: u32,
        tool_calls: impl IntoIterator<Item = ProviderToolCall>,
        stop_reason: ProviderStopReason,
        usage: Option<ProviderUsage>,
    ) -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION,
            chunk_index,
            delta: ProviderStreamDelta::ToolCalls {
                tool_calls: tool_calls.into_iter().collect(),
                stop_reason: Some(stop_reason),
            },
            is_terminal: true,
            usage,
        }
    }
}

impl fmt::Debug for ProviderStreamChunk {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderStreamChunk")
            .field("schema_version", &self.schema_version)
            .field("chunk_index", &self.chunk_index)
            .field("delta", &self.delta)
            .field("is_terminal", &self.is_terminal)
            .field("usage", &self.usage)
            .finish()
    }
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
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
    /// Provider requested one or more tool calls. This is a request
    /// envelope only; runtime code must lower it into `ToolCallRequest`
    /// values and route through package policy, journal, events, and
    /// effect records before executing anything.
    ToolCalls {
        /// Tool calls requested by the provider output.
        tool_calls: Vec<ProviderToolCall>,
        #[serde(skip_serializing_if = "Option::is_none")]
        /// Optional stop reason value.
        /// When absent, callers should use the documented default or skip that optional
        /// behavior.
        stop_reason: Option<ProviderStopReason>,
    },
    /// Provider stream or transport error. The payload must stay
    /// redacted so live event observers do not require raw content.
    Error {
        /// Redacted human-readable summary safe for events, telemetry, and
        /// logs.
        redacted_summary: String,
    },
}

impl fmt::Debug for ProviderStreamDelta {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text { text, stop_reason } => formatter
                .debug_struct("Text")
                .field("text", &"<redacted>")
                .field("text_chars", &text.chars().count())
                .field("stop_reason", stop_reason)
                .finish(),
            Self::Usage { usage } => formatter
                .debug_struct("Usage")
                .field("usage", usage)
                .finish(),
            Self::ToolCalls {
                tool_calls,
                stop_reason,
            } => formatter
                .debug_struct("ToolCalls")
                .field("tool_calls", tool_calls)
                .field("stop_reason", stop_reason)
                .finish(),
            Self::Error { redacted_summary } => formatter
                .debug_struct("Error")
                .field("redacted_summary", redacted_summary)
                .finish(),
        }
    }
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
/// Canonical provider-side request for a tool call. Constructing this
/// value does not execute a tool, resolve arguments, append journals, or
/// publish events. Runtime code must lower it into the SDK tool router,
/// policy, journal intent/result, event, and effect contracts.
pub struct ProviderToolCall {
    /// Stable tool call id used for typed lineage, lookup, or dedupe.
    pub tool_call_id: ToolCallId,
    /// Canonical tool name requested by the provider output.
    pub canonical_tool_name: CanonicalToolName,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Typed requested args refs references. Resolving them is separate from
    /// constructing this record.
    pub requested_args_refs: Vec<ContentRef>,
    /// Redacted summary for display, logs, events, or telemetry.
    /// It should describe the value without exposing raw private content.
    pub redacted_args_summary: String,
}

impl ProviderToolCall {
    /// Creates a provider tool-call DTO. This is data construction only;
    /// the runtime must perform routing, approval, journaling, and
    /// execution separately.
    pub fn new(
        tool_call_id: ToolCallId,
        canonical_tool_name: CanonicalToolName,
        redacted_args_summary: impl Into<String>,
    ) -> Self {
        Self {
            tool_call_id,
            canonical_tool_name,
            requested_args_refs: Vec::new(),
            redacted_args_summary: redacted_args_summary.into(),
        }
    }

    /// Returns this value with one requested argument content ref added.
    /// The ref is metadata only; resolving it is a separate policy-gated
    /// content operation.
    pub fn with_args_ref(mut self, args_ref: ContentRef) -> Self {
        self.requested_args_refs.push(args_ref);
        self
    }
}

impl fmt::Debug for ProviderToolCall {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderToolCall")
            .field("tool_call_id", &self.tool_call_id)
            .field("canonical_tool_name", &self.canonical_tool_name)
            .field("requested_args_refs", &self.requested_args_refs)
            .field(
                "redacted_args_summary_chars",
                &self.redacted_args_summary.chars().count(),
            )
            .finish()
    }
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
