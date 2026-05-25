//! Durable and observable SDK records. Use these DTOs for events, journals, effects,
//! context, output, and feature evidence. Constructing records is data-only;
//! persistence, publication, and external actions happen through ports or application
//! coordinators. This file contains the structured output portion of that contract.
//!
use serde::{Deserialize, Serialize};

use crate::{
    domain::{
        AttemptId, ContentRef, OutputSchemaId, PrivacyClass, RepairAttemptId, ValidationAttemptId,
    },
    output::{ContentHash, RepairAdapterRef, SchemaVersion},
};

/// Constant value for the records::structured_output contract. Use it
/// to keep SDK records and tests aligned on the same stable value.
pub const VALIDATION_RECORD_SCHEMA_VERSION: u16 = 1;
/// Constant value for the records::structured_output contract. Use it
/// to keep SDK records and tests aligned on the same stable value.
pub const REPAIR_RECORD_SCHEMA_VERSION: u16 = 1;
/// Constant value for the records::structured_output contract. Use it
/// to keep SDK records and tests aligned on the same stable value.
pub const STRUCTURED_OUTPUT_LIFECYCLE_RECORD_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the structured output lifecycle record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct StructuredOutputLifecycleRecord {
    /// Wire schema version for this record shape.
    /// Use it for compatibility checks before deserializing or replaying stored data.
    pub record_schema_version: u16,
    /// Kind discriminator for record kind.
    /// Use it to route finite match arms without parsing display text.
    pub record_kind: StructuredOutputLifecycleKind,
    /// Stable schema id used for typed lineage, lookup, or dedupe.
    pub schema_id: OutputSchemaId,
    /// Wire schema version for this record shape.
    /// Use it for compatibility checks before deserializing or replaying stored data.
    pub output_schema_version: SchemaVersion,
    /// Deterministic schema fingerprint used for stale checks, package
    /// evidence, or replay comparisons.
    pub schema_fingerprint: ContentHash,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stable source attempt id used for typed lineage, lookup, or dedupe.
    pub source_attempt_id: Option<AttemptId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Content reference for the candidate value being validated.
    pub candidate_content_ref: Option<ContentRef>,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy: PrivacyClass,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

impl StructuredOutputLifecycleRecord {
    /// Builds the requested record or result value.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn requested(
        schema_id: OutputSchemaId,
        output_schema_version: SchemaVersion,
        schema_fingerprint: ContentHash,
    ) -> Self {
        Self {
            record_schema_version: STRUCTURED_OUTPUT_LIFECYCLE_RECORD_SCHEMA_VERSION,
            record_kind: StructuredOutputLifecycleKind::Requested,
            schema_id,
            output_schema_version,
            schema_fingerprint,
            source_attempt_id: None,
            candidate_content_ref: None,
            privacy: PrivacyClass::ContentRefsOnly,
            redacted_summary: "structured output requested".to_string(),
        }
    }

    /// Builds the validation started record or result value.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn validation_started(
        schema_id: OutputSchemaId,
        output_schema_version: SchemaVersion,
        schema_fingerprint: ContentHash,
        source_attempt_id: AttemptId,
        candidate_content_ref: ContentRef,
    ) -> Self {
        Self {
            record_schema_version: STRUCTURED_OUTPUT_LIFECYCLE_RECORD_SCHEMA_VERSION,
            record_kind: StructuredOutputLifecycleKind::ValidationStarted,
            schema_id,
            output_schema_version,
            schema_fingerprint,
            source_attempt_id: Some(source_attempt_id),
            candidate_content_ref: Some(candidate_content_ref),
            privacy: PrivacyClass::ContentRefsOnly,
            redacted_summary: "structured output validation started".to_string(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite structured output lifecycle kind cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum StructuredOutputLifecycleKind {
    /// Use this variant when the contract needs to represent requested; selecting it has no side effect by itself.
    Requested,
    /// Use this variant when the contract needs to represent validation started; selecting it has no side effect by itself.
    ValidationStarted,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the validation record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct ValidationRecord {
    /// Wire schema version for this record shape.
    /// Use it for compatibility checks before deserializing or replaying stored data.
    pub record_schema_version: u16,
    /// Kind discriminator for record kind.
    /// Use it to route finite match arms without parsing display text.
    pub record_kind: ValidationRecordKind,
    /// Stable schema id used for typed lineage, lookup, or dedupe.
    pub schema_id: OutputSchemaId,
    /// Wire schema version for this record shape.
    /// Use it for compatibility checks before deserializing or replaying stored data.
    pub output_schema_version: SchemaVersion,
    /// Deterministic schema fingerprint used for stale checks, package
    /// evidence, or replay comparisons.
    pub schema_fingerprint: ContentHash,
    /// Stable validation attempt id used for typed lineage, lookup, or
    /// dedupe.
    pub validation_attempt_id: ValidationAttemptId,
    /// Stable source attempt id used for typed lineage, lookup, or dedupe.
    pub source_attempt_id: AttemptId,
    /// Content reference for the candidate value being validated.
    pub candidate_content_ref: ContentRef,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy: PrivacyClass,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Bounded errors included in this record. Limits and truncation are
    /// represented by companion metadata when applicable.
    pub errors: Vec<ValidationErrorSummary>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Validation policy applied before output is accepted as typed data.
    /// It controls validator selection, bounds, failure visibility, and local validation
    /// behavior.
    pub validation_attempts: Vec<ValidationAttemptId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Attempt identifier or attempt history for bounded retry/repair.
    /// Use it to preserve ordering and avoid retry loops that cannot be audited.
    pub repair_attempts: Vec<RepairAttemptId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Attempt identifier or attempt history for bounded retry/repair.
    /// Use it to preserve ordering and avoid retry loops that cannot be audited.
    pub source_attempt_ids: Vec<AttemptId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional retry exhausted value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub retry_exhausted: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite validation record kind cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ValidationRecordKind {
    /// Use this variant when the contract needs to represent validation succeeded; selecting it has no side effect by itself.
    ValidationSucceeded,
    /// Use this variant when the contract needs to represent validation failed; selecting it has no side effect by itself.
    ValidationFailed,
    /// Use this variant when the contract needs to represent schema rejected; selecting it has no side effect by itself.
    SchemaRejected,
    /// Use this variant when the contract needs to represent terminal failure; selecting it has no side effect by itself.
    TerminalFailure,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the validation error summary record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct ValidationErrorSummary {
    /// Code used by this record or request.
    pub code: ValidationErrorCode,
    /// Workspace-relative or resource path selected by the request or result.
    pub path: String,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

impl ValidationErrorSummary {
    /// Creates a new records::structured_output value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new(
        code: ValidationErrorCode,
        path: impl Into<String>,
        redacted_summary: impl Into<String>,
    ) -> Self {
        Self {
            code,
            path: path.into(),
            redacted_summary: redacted_summary.into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite validation error code cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ValidationErrorCode {
    /// Use this variant when the contract needs to represent candidate too large; selecting it has no side effect by itself.
    CandidateTooLarge,
    /// Use this variant when the contract needs to represent invalid json; selecting it has no side effect by itself.
    InvalidJson,
    /// Use this variant when the contract needs to represent unsupported dialect; selecting it has no side effect by itself.
    UnsupportedDialect,
    /// Use this variant when the contract needs to represent unsupported schema ref; selecting it has no side effect by itself.
    UnsupportedSchemaRef,
    /// Use this variant when the contract needs to represent schema contract violation; selecting it has no side effect by itself.
    SchemaContractViolation,
    /// Use this variant when the contract needs to represent hostile schema; selecting it has no side effect by itself.
    HostileSchema,
    /// Use this variant when the contract needs to represent missing required field; selecting it has no side effect by itself.
    MissingRequiredField,
    /// Use this variant when the contract needs to represent type mismatch; selecting it has no side effect by itself.
    TypeMismatch,
    /// Use this variant when the contract needs to represent additional property denied; selecting it has no side effect by itself.
    AdditionalPropertyDenied,
    /// Use this variant when the contract needs to represent enum mismatch; selecting it has no side effect by itself.
    EnumMismatch,
    /// Use this variant when the contract needs to represent min length violation; selecting it has no side effect by itself.
    MinLengthViolation,
    /// Use this variant when the contract needs to represent max length violation; selecting it has no side effect by itself.
    MaxLengthViolation,
    /// Use this variant when the contract needs to represent semantic validator unavailable; selecting it has no side effect by itself.
    SemanticValidatorUnavailable,
}

impl ValidationErrorCode {
    /// Reports whether this value is schema rejection. The check is
    /// pure and does not mutate SDK or host state.
    pub(crate) fn is_schema_rejection(&self) -> bool {
        matches!(
            self,
            Self::UnsupportedDialect
                | Self::UnsupportedSchemaRef
                | Self::SchemaContractViolation
                | Self::HostileSchema
                | Self::SemanticValidatorUnavailable
        )
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the repair prompt record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct RepairPrompt {
    /// Wire schema version for this record shape.
    /// Use it for compatibility checks before deserializing or replaying stored data.
    pub record_schema_version: u16,
    /// Stable repair attempt id used for typed lineage, lookup, or dedupe.
    pub repair_attempt_id: RepairAttemptId,
    /// Stable validation attempt id used for typed lineage, lookup, or
    /// dedupe.
    pub validation_attempt_id: ValidationAttemptId,
    /// Stable source attempt id used for typed lineage, lookup, or dedupe.
    pub source_attempt_id: AttemptId,
    /// Stable schema id used for typed lineage, lookup, or dedupe.
    pub schema_id: OutputSchemaId,
    /// Wire schema version for this record shape.
    /// Use it for compatibility checks before deserializing or replaying stored data.
    pub output_schema_version: SchemaVersion,
    /// Deterministic schema fingerprint used for stale checks, package
    /// evidence, or replay comparisons.
    pub schema_fingerprint: ContentHash,
    /// Typed repair adapter ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub repair_adapter_ref: RepairAdapterRef,
    /// Attempt identifier or attempt history for bounded retry/repair.
    /// Use it to preserve ordering and avoid retry loops that cannot be audited.
    pub attempt_index: u8,
    /// Attempt identifier or attempt history for bounded retry/repair.
    /// Use it to preserve ordering and avoid retry loops that cannot be audited.
    pub max_repair_attempts: u8,
    /// Whether include schema in prompt is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub include_schema_in_prompt: bool,
    /// Collection of redacted errors values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub redacted_errors: Vec<ValidationErrorSummary>,
    /// Candidate content used by this record or request.
    pub candidate_content: RepairPromptCandidateContent,
    /// Redacted summary for display, logs, events, or telemetry.
    /// It should describe the value without exposing raw private content.
    pub prompt_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
/// Enumerates the finite repair prompt candidate content cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum RepairPromptCandidateContent {
    /// Use this variant when the contract needs to represent content ref only; selecting it has no side effect by itself.
    ContentRefOnly {
        /// Content reference for the candidate value being validated.
        candidate_content_ref: ContentRef,
    },
    /// Use this variant when the contract needs to represent redacted candidate; selecting it has no side effect by itself.
    RedactedCandidate {
        /// Redacted human-readable summary safe for events, telemetry, and
        /// logs.
        redacted_summary: String,
    },
    /// Use this variant when the contract needs to represent omitted; selecting it has no side effect by itself.
    Omitted,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the repair record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct RepairRecord {
    /// Wire schema version for this record shape.
    /// Use it for compatibility checks before deserializing or replaying stored data.
    pub record_schema_version: u16,
    /// Kind discriminator for record kind.
    /// Use it to route finite match arms without parsing display text.
    pub record_kind: RepairRecordKind,
    /// Stable repair attempt id used for typed lineage, lookup, or dedupe.
    pub repair_attempt_id: RepairAttemptId,
    /// Stable validation attempt id used for typed lineage, lookup, or
    /// dedupe.
    pub validation_attempt_id: ValidationAttemptId,
    /// Stable source attempt id used for typed lineage, lookup, or dedupe.
    pub source_attempt_id: AttemptId,
    /// Stable schema id used for typed lineage, lookup, or dedupe.
    pub schema_id: OutputSchemaId,
    /// Wire schema version for this record shape.
    /// Use it for compatibility checks before deserializing or replaying stored data.
    pub output_schema_version: SchemaVersion,
    /// Deterministic schema fingerprint used for stale checks, package
    /// evidence, or replay comparisons.
    pub schema_fingerprint: ContentHash,
    /// Typed repair adapter ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub repair_adapter_ref: RepairAdapterRef,
    /// Attempt identifier or attempt history for bounded retry/repair.
    /// Use it to preserve ordering and avoid retry loops that cannot be audited.
    pub attempt_index: u8,
    /// Attempt identifier or attempt history for bounded retry/repair.
    /// Use it to preserve ordering and avoid retry loops that cannot be audited.
    pub max_repair_attempts: u8,
    /// Prompt used by this record or request.
    pub prompt: RepairPrompt,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy: PrivacyClass,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the repair exhaustion record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct RepairExhaustionRecord {
    /// Wire schema version for this record shape.
    /// Use it for compatibility checks before deserializing or replaying stored data.
    pub record_schema_version: u16,
    /// Kind discriminator for record kind.
    /// Use it to route finite match arms without parsing display text.
    pub record_kind: RepairRecordKind,
    /// Stable schema id used for typed lineage, lookup, or dedupe.
    pub schema_id: OutputSchemaId,
    /// Wire schema version for this record shape.
    /// Use it for compatibility checks before deserializing or replaying stored data.
    pub output_schema_version: SchemaVersion,
    /// Validation policy applied before output is accepted as typed data.
    /// It controls validator selection, bounds, failure visibility, and local validation
    /// behavior.
    pub validation_attempts: Vec<ValidationAttemptId>,
    /// Attempt identifier or attempt history for bounded retry/repair.
    /// Use it to preserve ordering and avoid retry loops that cannot be audited.
    pub repair_attempts: Vec<RepairAttemptId>,
    /// Attempt identifier or attempt history for bounded retry/repair.
    /// Use it to preserve ordering and avoid retry loops that cannot be audited.
    pub source_attempt_ids: Vec<AttemptId>,
    /// Content reference for the candidate value being validated.
    pub candidate_content_ref: ContentRef,
    /// Whether retry exhausted is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub retry_exhausted: bool,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
    /// Redacted explanation for a denial, failure, status, or package delta.
    pub reason: String,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy: PrivacyClass,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite repair record kind cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum RepairRecordKind {
    /// Use this variant when the contract needs to represent repair requested; selecting it has no side effect by itself.
    RepairRequested,
    /// Use this variant when the contract needs to represent repair exhausted; selecting it has no side effect by itself.
    RepairExhausted,
}
