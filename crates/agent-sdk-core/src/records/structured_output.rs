use serde::{Deserialize, Serialize};

use crate::{
    domain::{
        AttemptId, ContentRef, OutputSchemaId, PrivacyClass, RepairAttemptId, ValidationAttemptId,
    },
    output::{ContentHash, RepairAdapterRef, SchemaVersion},
};

pub const VALIDATION_RECORD_SCHEMA_VERSION: u16 = 1;
pub const REPAIR_RECORD_SCHEMA_VERSION: u16 = 1;
pub const STRUCTURED_OUTPUT_LIFECYCLE_RECORD_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StructuredOutputLifecycleRecord {
    pub record_schema_version: u16,
    pub record_kind: StructuredOutputLifecycleKind,
    pub schema_id: OutputSchemaId,
    pub output_schema_version: SchemaVersion,
    pub schema_fingerprint: ContentHash,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_attempt_id: Option<AttemptId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate_content_ref: Option<ContentRef>,
    pub privacy: PrivacyClass,
    pub redacted_summary: String,
}

impl StructuredOutputLifecycleRecord {
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
pub enum StructuredOutputLifecycleKind {
    Requested,
    ValidationStarted,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ValidationRecord {
    pub record_schema_version: u16,
    pub record_kind: ValidationRecordKind,
    pub schema_id: OutputSchemaId,
    pub output_schema_version: SchemaVersion,
    pub schema_fingerprint: ContentHash,
    pub validation_attempt_id: ValidationAttemptId,
    pub source_attempt_id: AttemptId,
    pub candidate_content_ref: ContentRef,
    pub privacy: PrivacyClass,
    pub redacted_summary: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<ValidationErrorSummary>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub validation_attempts: Vec<ValidationAttemptId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub repair_attempts: Vec<RepairAttemptId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_attempt_ids: Vec<AttemptId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_exhausted: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationRecordKind {
    ValidationSucceeded,
    ValidationFailed,
    SchemaRejected,
    TerminalFailure,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ValidationErrorSummary {
    pub code: ValidationErrorCode,
    pub path: String,
    pub redacted_summary: String,
}

impl ValidationErrorSummary {
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
pub enum ValidationErrorCode {
    CandidateTooLarge,
    InvalidJson,
    UnsupportedDialect,
    UnsupportedSchemaRef,
    SchemaContractViolation,
    HostileSchema,
    MissingRequiredField,
    TypeMismatch,
    AdditionalPropertyDenied,
    EnumMismatch,
    MinLengthViolation,
    MaxLengthViolation,
    SemanticValidatorUnavailable,
}

impl ValidationErrorCode {
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
pub struct RepairPrompt {
    pub record_schema_version: u16,
    pub repair_attempt_id: RepairAttemptId,
    pub validation_attempt_id: ValidationAttemptId,
    pub source_attempt_id: AttemptId,
    pub schema_id: OutputSchemaId,
    pub output_schema_version: SchemaVersion,
    pub schema_fingerprint: ContentHash,
    pub repair_adapter_ref: RepairAdapterRef,
    pub attempt_index: u8,
    pub max_repair_attempts: u8,
    pub include_schema_in_prompt: bool,
    pub redacted_errors: Vec<ValidationErrorSummary>,
    pub candidate_content: RepairPromptCandidateContent,
    pub prompt_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RepairPromptCandidateContent {
    ContentRefOnly { candidate_content_ref: ContentRef },
    RedactedCandidate { redacted_summary: String },
    Omitted,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RepairRecord {
    pub record_schema_version: u16,
    pub record_kind: RepairRecordKind,
    pub repair_attempt_id: RepairAttemptId,
    pub validation_attempt_id: ValidationAttemptId,
    pub source_attempt_id: AttemptId,
    pub schema_id: OutputSchemaId,
    pub output_schema_version: SchemaVersion,
    pub schema_fingerprint: ContentHash,
    pub repair_adapter_ref: RepairAdapterRef,
    pub attempt_index: u8,
    pub max_repair_attempts: u8,
    pub prompt: RepairPrompt,
    pub redacted_summary: String,
    pub privacy: PrivacyClass,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RepairExhaustionRecord {
    pub record_schema_version: u16,
    pub record_kind: RepairRecordKind,
    pub schema_id: OutputSchemaId,
    pub output_schema_version: SchemaVersion,
    pub validation_attempts: Vec<ValidationAttemptId>,
    pub repair_attempts: Vec<RepairAttemptId>,
    pub source_attempt_ids: Vec<AttemptId>,
    pub candidate_content_ref: ContentRef,
    pub retry_exhausted: bool,
    pub redacted_summary: String,
    pub reason: String,
    pub privacy: PrivacyClass,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RepairRecordKind {
    RepairRequested,
    RepairExhausted,
}
