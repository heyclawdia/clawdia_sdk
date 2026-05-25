//! Durable and observable SDK records. Use these DTOs for events, journals, effects,
//! context, output, and feature evidence. Constructing records is data-only;
//! persistence, publication, and external actions happen through ports or application
//! coordinators. This file contains the output portion of that contract.
//!
use core::fmt;
use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize, de::Error as DeError};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::{
    domain::{AgentError, ContentRef, IdValidationError, OutputSchemaId, PolicyKind, PolicyRef},
    ids::validate_identifier,
    policy::ContentCapturePolicy,
    typed_output_ports::TypedOutputModel,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the output contract record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct OutputContract {
    /// Stable schema id used for typed lineage, lookup, or dedupe.
    pub schema_id: OutputSchemaId,
    /// Wire schema version used for compatibility checks.
    pub schema_version: SchemaVersion,
    /// Schema dialect used to interpret the output schema.
    /// Validators use it to select the supported JSON-schema subset and compatibility rules.
    pub dialect: OutputSchemaDialect,
    /// Schema reference or inline schema used to validate structured output.
    /// The runtime resolves this before treating model output as typed data.
    pub schema: OutputSchemaRef,
    /// Mode that selects how this operation or contract should behave.
    /// Callers use it to choose the explicit execution path instead of relying on hidden
    /// defaults.
    pub mode: OutputMode,
    /// Validation policy applied before output is accepted as typed data.
    /// It controls validator selection, bounds, failure visibility, and local validation
    /// behavior.
    pub validation: ValidationPolicy,
    /// Repair policy used after structured output validation fails.
    /// It controls whether repair is attempted and which policy gates must approve it.
    pub repair: RepairPolicy,
    /// Retry budget for validation, repair, or adapter attempts.
    /// Runtimes use it to stop bounded loops deterministically.
    pub retry_budget: RetryBudget,
    /// Content-capture policy that governs raw content, summaries, redaction, and retention.
    /// Projection, telemetry, and delivery paths must honor it before exposing content.
    pub content_policy: ContentCapturePolicy,
    /// Provider-facing projection hint for structured output requests.
    /// It can guide model prompting but does not replace local validation policy.
    pub projection_hint: OutputProjectionHint,
}

impl OutputContract {
    /// Builds the for type value with the documented defaults.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn for_type<T: TypedOutputModel>() -> Self {
        Self::from_typed_model::<T>(OutputPreset::StrictJsonSchema)
    }

    /// Returns an updated value with strict json schema configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn strict_json_schema<T: TypedOutputModel>() -> Self {
        Self::from_typed_model::<T>(OutputPreset::StrictJsonSchema)
    }

    /// Builds the fast lenient value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn fast_lenient<T: TypedOutputModel>() -> Self {
        Self::from_typed_model::<T>(OutputPreset::FastLenient)
    }

    /// Builds the provider assisted value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn provider_assisted<T: TypedOutputModel>() -> Self {
        Self::from_typed_model::<T>(OutputPreset::ProviderAssisted)
    }

    /// Builds the inline json schema value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn inline_json_schema(
        schema_id: OutputSchemaId,
        schema_version: SchemaVersion,
        redacted_schema: Value,
    ) -> Self {
        let content_hash = canonical_content_hash(&redacted_schema);
        Self::new(
            schema_id,
            schema_version,
            OutputSchemaDialect::JsonSchema2020_12Subset,
            OutputSchemaRef::InlineJson {
                content_hash,
                redacted_schema,
            },
            OutputPreset::StrictJsonSchema,
        )
    }

    /// Creates a new records::output value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new(
        schema_id: OutputSchemaId,
        schema_version: SchemaVersion,
        dialect: OutputSchemaDialect,
        schema: OutputSchemaRef,
        preset: OutputPreset,
    ) -> Self {
        let mut validation = ValidationPolicy::strict_defaults();
        let mut repair = RepairPolicy::safe_defaults();
        let mut retry_budget = RetryBudget::attempts(2);
        let mut projection_hint = OutputProjectionHint::schema_ref_only();

        match preset {
            OutputPreset::StrictJsonSchema => {}
            OutputPreset::FastLenient => {
                validation.allow_additional_properties = true;
                validation.max_errors_returned = 8;
                repair.max_repair_attempts = 1;
                retry_budget = RetryBudget::attempts(1);
                projection_hint.provider_hint_policy = ProviderHintPolicy::SchemaOptional;
            }
            OutputPreset::ProviderAssisted => {
                projection_hint.provider_hint_policy = ProviderHintPolicy::ProviderNativeHint;
            }
        }

        Self {
            schema_id,
            schema_version,
            dialect,
            schema,
            mode: OutputMode::FinalOnly,
            validation,
            repair,
            retry_budget,
            content_policy: ContentCapturePolicy::safe_defaults(PolicyRef::with_kind(
                PolicyKind::Privacy,
                "policy.output.content_capture.safe_defaults",
            )),
            projection_hint,
        }
    }

    /// Computes the stable schema fingerprint for this records::output
    /// value. The computation is deterministic and side-effect free so
    /// it can be used in package, journal, or test evidence.
    pub fn schema_fingerprint(&self) -> ContentHash {
        self.schema.content_hash()
    }

    /// Validates the records::output invariants and returns a typed
    /// error on failure. Validation is pure and does not perform I/O,
    /// dispatch, journal appends, or adapter calls.
    pub fn validate_shape(&self) -> Result<(), AgentError> {
        if self.retry_budget.max_attempts < self.repair.max_repair_attempts {
            return Err(AgentError::contract_violation(
                "output retry budget must cover repair attempts",
            ));
        }
        if self.validation.max_candidate_bytes == 0 {
            return Err(AgentError::missing_required_field(
                "output.validation.max_candidate_bytes",
            ));
        }
        self.schema.validate_shape()
    }

    fn from_typed_model<T: TypedOutputModel>(preset: OutputPreset) -> Self {
        Self::new(
            OutputSchemaId::new(T::SCHEMA_ID),
            T::SCHEMA_VERSION,
            T::DIALECT,
            T::schema_ref(),
            preset,
        )
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
/// Carries the schema version record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct SchemaVersion {
    /// Major used by this record or request.
    pub major: u16,
    /// Minor used by this record or request.
    pub minor: u16,
    /// Patch used by this record or request.
    pub patch: u16,
}

impl SchemaVersion {
    /// Constant value for the records::output contract. Use it to keep
    /// SDK records and tests aligned on the same stable value.
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite output schema dialect cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum OutputSchemaDialect {
    /// Use this variant when the contract needs to represent json schema2020 12 subset; selecting it has no side effect by itself.
    JsonSchema2020_12Subset,
    /// Use this variant when the contract needs to represent rust serde type name; selecting it has no side effect by itself.
    RustSerdeTypeName,
    /// Use this variant when the contract needs to represent host registered; selecting it has no side effect by itself.
    HostRegistered,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
/// Enumerates the finite output schema ref cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum OutputSchemaRef {
    /// Use this variant when the contract needs to represent inline json; selecting it has no side effect by itself.
    InlineJson {
        /// Stable hash for the bytes or canonical payload used for stale
        /// checks and fingerprints.
        content_hash: ContentHash,
        /// Schema body safe to expose after redaction.
        /// It can be logged or shown without revealing private schema content beyond policy.
        redacted_schema: Value,
    },
    /// Use this variant when the contract needs to represent content store; selecting it has no side effect by itself.
    ContentStore {
        /// Content reference where payload bytes or structured tool output
        /// are stored.
        content_ref: ContentRef,
        /// Stable hash for the bytes or canonical payload used for stale
        /// checks and fingerprints.
        content_hash: ContentHash,
    },
    /// Use this variant when the contract needs to represent rust serde; selecting it has no side effect by itself.
    RustSerde {
        /// Type name used by this record or request.
        type_name: TypeName,
        /// Crate name used by this record or request.
        crate_name: CrateName,
        /// Version string for this capability, package, or protocol surface.
        /// Use it for compatibility checks during package or adapter resolution.
        crate_version: String,
        /// Typed generated schema ref reference. Resolving or executing it is
        /// a separate policy-gated step.
        generated_schema_ref: ContentRef,
        /// Stable hash for the bytes or canonical payload used for stale
        /// checks and fingerprints.
        content_hash: ContentHash,
    },
    /// Use this variant when the contract needs to represent host registered; selecting it has no side effect by itself.
    HostRegistered {
        /// Typed validator ref reference. Resolving or executing it is a
        /// separate policy-gated step.
        validator_ref: OutputValidatorRef,
        /// Version string for this capability, package, or protocol surface.
        /// Use it for compatibility checks during package or adapter resolution.
        contract_version: SchemaVersion,
        /// Stable hash for the bytes or canonical payload used for stale
        /// checks and fingerprints.
        content_hash: ContentHash,
    },
}

impl Eq for OutputSchemaRef {}

impl OutputSchemaRef {
    /// Computes the stable content hash for this records::output value.
    /// The computation is deterministic and side-effect free so it can
    /// be used in package, journal, or test evidence.
    pub fn content_hash(&self) -> ContentHash {
        match self {
            Self::InlineJson { content_hash, .. }
            | Self::ContentStore { content_hash, .. }
            | Self::RustSerde { content_hash, .. }
            | Self::HostRegistered { content_hash, .. } => content_hash.clone(),
        }
    }

    /// Validates the records::output invariants and returns a typed
    /// error on failure. Validation is pure and does not perform I/O,
    /// dispatch, journal appends, or adapter calls.
    pub fn validate_shape(&self) -> Result<(), AgentError> {
        match self {
            Self::InlineJson {
                content_hash,
                redacted_schema,
            } => {
                content_hash.validate_shape()?;
                if redacted_schema.is_null() {
                    return Err(AgentError::missing_required_field(
                        "output.schema.redacted_schema",
                    ));
                }
            }
            Self::ContentStore { content_hash, .. }
            | Self::RustSerde { content_hash, .. }
            | Self::HostRegistered { content_hash, .. } => content_hash.validate_shape()?,
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite output mode cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum OutputMode {
    /// Use this variant when the contract needs to represent final only; selecting it has no side effect by itself.
    FinalOnly,
    /// Use this variant when the contract needs to represent incremental preview; selecting it has no side effect by itself.
    IncrementalPreview,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the validation policy record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct ValidationPolicy {
    /// Typed validator ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub validator_ref: OutputValidatorRef,
    /// max candidate bytes used for bounds checks, summaries, or truncation
    /// evidence.
    pub max_candidate_bytes: u64,
    /// Maximum number of validation errors to expose in a report.
    /// Use it to keep failure output bounded and safe for logs or events.
    pub max_errors_returned: u16,
    /// Whether JSON-schema validation permits properties not declared by the schema.
    /// Strict SDK output contracts should usually keep this false for deterministic
    /// typed-output validation.
    pub allow_additional_properties: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Additional semantic validator refs to run after schema validation.
    /// Each validator still resolves through policy-gated validator infrastructure.
    pub semantic_validators: Vec<SemanticValidatorRef>,
    /// Timeout budget in milliseconds for the requested operation.
    pub timeout_ms: u64,
    /// Visibility policy for validation failure details.
    /// It controls how much error detail can appear in reports, events, and repair prompts.
    pub failure_visibility: ValidationFailureVisibility,
}

impl ValidationPolicy {
    /// Returns an updated value with strict defaults configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn strict_defaults() -> Self {
        Self {
            validator_ref: OutputValidatorRef::new("validator.output.json_schema.local.v1"),
            max_candidate_bytes: 32 * 1024,
            max_errors_returned: 32,
            allow_additional_properties: false,
            semantic_validators: Vec::new(),
            timeout_ms: 10_000,
            failure_visibility: ValidationFailureVisibility::RedactedSummary,
        }
    }

    /// Returns validator ref policy for callers that need to inspect the contract state.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn validator_ref_policy(&self) -> PolicyRef {
        PolicyRef::with_kind(PolicyKind::RuntimePackage, self.validator_ref.as_str())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite validation failure visibility cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ValidationFailureVisibility {
    /// Use this variant when the contract needs to represent redacted summary; selecting it has no side effect by itself.
    RedactedSummary,
    /// Only stable validation error codes should be exposed. Selecting
    /// this variant is data-only and does not publish or persist errors.
    ErrorCodesOnly,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the repair policy record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct RepairPolicy {
    /// Typed repair adapter ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub repair_adapter_ref: RepairAdapterRef,
    /// Attempt identifier or attempt history for bounded retry/repair.
    /// Use it to preserve ordering and avoid retry loops that cannot be audited.
    pub max_repair_attempts: u8,
    /// Whether include schema in prompt is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub include_schema_in_prompt: bool,
    /// Whether include redacted errors is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub include_redacted_errors: bool,
    /// Whether include candidate content is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub include_candidate_content: CandidateContentRepairPolicy,
    /// Backoff used by this record or request.
    pub backoff: RetryBackoff,
    /// On exhausted used by this record or request.
    pub on_exhausted: RepairExhaustedBehavior,
}

impl RepairPolicy {
    /// Returns an updated value with safe defaults configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn safe_defaults() -> Self {
        Self {
            repair_adapter_ref: RepairAdapterRef::new("repair.output.provider_redacted.v1"),
            max_repair_attempts: 2,
            include_schema_in_prompt: true,
            include_redacted_errors: true,
            include_candidate_content: CandidateContentRepairPolicy::ContentRefOnly,
            backoff: RetryBackoff::None,
            on_exhausted: RepairExhaustedBehavior::FailRun,
        }
    }

    /// Returns repair adapter ref policy for callers that need to inspect the contract state.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn repair_adapter_ref_policy(&self) -> PolicyRef {
        PolicyRef::with_kind(PolicyKind::RuntimePackage, self.repair_adapter_ref.as_str())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the retry budget record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct RetryBudget {
    /// Attempt identifier or attempt history for bounded retry/repair.
    /// Use it to preserve ordering and avoid retry loops that cannot be audited.
    pub max_attempts: u8,
}

impl RetryBudget {
    /// Builds the attempts value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn attempts(max_attempts: u8) -> Self {
        Self { max_attempts }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite candidate content repair policy cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum CandidateContentRepairPolicy {
    /// Use this variant when the contract needs to represent content ref only; selecting it has no side effect by itself.
    ContentRefOnly,
    /// Use this variant when the contract needs to represent redacted candidate; selecting it has no side effect by itself.
    RedactedCandidate,
    /// Use this variant when the contract needs to represent omit candidate; selecting it has no side effect by itself.
    OmitCandidate,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite retry backoff cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum RetryBackoff {
    /// Use this variant when the contract needs to represent none; selecting it has no side effect by itself.
    None,
    /// Use this variant when the contract needs to represent linear; selecting it has no side effect by itself.
    Linear,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite repair exhausted behavior cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum RepairExhaustedBehavior {
    /// Use this variant when the contract needs to represent fail run; selecting it has no side effect by itself.
    FailRun,
    /// Use this variant when the contract needs to represent return validation error; selecting it has no side effect by itself.
    ReturnValidationError,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the output projection hint record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct OutputProjectionHint {
    /// Policy for provider-side structured-output hints.
    /// Hints may guide prompting but cannot replace SDK-owned validation.
    pub provider_hint_policy: ProviderHintPolicy,
    /// Typed include schema ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub include_schema_ref: bool,
}

impl OutputProjectionHint {
    /// Returns an updated value with schema ref only configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn schema_ref_only() -> Self {
        Self {
            provider_hint_policy: ProviderHintPolicy::SchemaRequired,
            include_schema_ref: true,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite provider hint policy cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ProviderHintPolicy {
    /// Use this variant when the contract needs to represent schema required; selecting it has no side effect by itself.
    SchemaRequired,
    /// Use this variant when the contract needs to represent schema optional; selecting it has no side effect by itself.
    SchemaOptional,
    /// Use this variant when the contract needs to represent provider native hint; selecting it has no side effect by itself.
    ProviderNativeHint,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Enumerates the finite output preset cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum OutputPreset {
    /// Use this variant when the contract needs to represent strict json schema; selecting it has no side effect by itself.
    StrictJsonSchema,
    /// Use this variant when the contract needs to represent fast lenient; selecting it has no side effect by itself.
    FastLenient,
    /// Use this variant when the contract needs to represent provider assisted; selecting it has no side effect by itself.
    ProviderAssisted,
}

macro_rules! output_string {
    ($name:ident) => {
        #[doc = concat!(
            "Typed output-string wrapper for `",
            stringify!($name),
            "`. Use it where output contracts need stable schema, hash, or validator refs; ",
            "constructing it is data-only and performs no side effects."
        )]
        #[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Creates a new records::output value with explicit
            /// caller-provided inputs. This constructor is data-only
            /// and performs no I/O or external side effects.
            ///
            /// # Panics
            ///
            /// Panics if constructor invariants fail, such as invalid identifier
            /// text or constructor-specific bounds. Use a fallible constructor such as
            /// `try_new` when one is available for untrusted input.
            pub fn new(value: impl Into<String>) -> Self {
                Self::try_new(value).expect(concat!(stringify!($name), " must be valid"))
            }

            /// Creates a new records::output value after validation.
            /// Returns an SDK error instead of panicking when the
            /// identifier or input does not satisfy the contract.
            pub fn try_new(value: impl Into<String>) -> Result<Self, IdValidationError> {
                let value = value.into();
                validate_identifier(&value)?;
                Ok(Self(value))
            }

            /// Returns this value as str. The accessor is side-effect
            /// free and keeps ownership with the caller.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::try_new(value).map_err(D::Error::custom)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(concat!(stringify!($name), "(redacted)"))
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(concat!(stringify!($name), "(redacted)"))
            }
        }
    };
}

output_string!(ContentHash);
output_string!(OutputValidatorRef);
output_string!(RepairAdapterRef);
output_string!(SemanticValidatorRef);
output_string!(TypeName);
output_string!(CrateName);

impl ContentHash {
    fn validate_shape(&self) -> Result<(), AgentError> {
        let Some(digest) = self.as_str().strip_prefix("sha256:") else {
            return Err(AgentError::contract_violation(
                "output schema content_hash must be sha256-prefixed",
            ));
        };
        if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(AgentError::contract_violation(
                "output schema content_hash must be a sha256 hex digest",
            ));
        }
        Ok(())
    }
}

fn canonical_content_hash(value: &Value) -> ContentHash {
    let normalized = normalize_json_value(value.clone());
    let bytes = serde_json::to_vec(&normalized).expect("serde_json::Value serializes");
    ContentHash::new(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn normalize_json_value(value: Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.into_iter().map(normalize_json_value).collect()),
        Value::Object(fields) => {
            let mut sorted = BTreeMap::new();
            for (key, value) in fields {
                sorted.insert(key, value);
            }
            let mut normalized = Map::new();
            for (key, value) in sorted {
                normalized.insert(key, normalize_json_value(value));
            }
            Value::Object(normalized)
        }
        scalar => scalar,
    }
}
