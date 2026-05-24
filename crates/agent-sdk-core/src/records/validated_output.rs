use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    content::ContentRef,
    domain::{
        AgentError, AgentErrorKind, AttemptId, EntityRef, LineageRef, OutputSchemaId, PolicyRef,
        PrivacyClass, RepairAttemptId, RetryClassification, RunId, ValidatedOutputId,
        ValidationAttemptId,
    },
    output::{ContentHash, SchemaVersion},
    typed_output_ports::TypedOutputDeserializer,
};

pub const VALIDATED_OUTPUT_RECORD_SCHEMA_VERSION: u16 = 1;
pub const VALIDATION_REPORT_RECORD_SCHEMA_VERSION: u16 = 1;
pub const TYPED_RESULT_PUBLICATION_RECORD_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ValidatedOutput {
    pub record_schema_version: u16,
    pub output_id: ValidatedOutputId,
    pub schema_id: OutputSchemaId,
    pub schema_version: SchemaVersion,
    pub schema_fingerprint: ContentHash,
    pub canonical_value_ref: ContentRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub validation_report_refs: Vec<ValidationReportRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub validation_attempts: Vec<ValidationAttemptId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub repair_attempts: Vec<RepairAttemptId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_attempt_ids: Vec<AttemptId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content_refs: Vec<ContentRef>,
    pub lineage: OutputLineage,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policy_refs: Vec<PolicyRef>,
    pub privacy: PrivacyClass,
    pub redacted_summary: String,
}

impl ValidatedOutput {
    pub fn from_validation_report(
        params: ValidatedOutputParams,
        report: &ValidationReportRecord,
    ) -> Result<Self, TypedOutputError> {
        if !report.status.is_success() {
            return Err(TypedOutputError::ValidationReportFailed {
                validation_attempt_id: report.validation_attempt_id.clone(),
            });
        }
        if report.schema_id != params.schema_id || report.schema_version != params.schema_version {
            return Err(TypedOutputError::SchemaMismatch {
                expected_schema_id: params.schema_id,
                actual_schema_id: report.schema_id.clone(),
            });
        }

        let mut content_refs = params.content_refs;
        push_unique_content_ref(&mut content_refs, params.canonical_value_ref.clone());
        push_unique_content_ref(&mut content_refs, report.candidate_content_ref.clone());
        push_unique_content_ref(&mut content_refs, report.validation_report_ref.clone());

        let mut policy_refs = params.policy_refs;
        for policy_ref in &report.policy_refs {
            push_unique_policy_ref(&mut policy_refs, policy_ref.clone());
        }

        let mut source_attempt_ids = params.source_attempt_ids;
        if !source_attempt_ids.contains(&report.source_attempt_id) {
            source_attempt_ids.push(report.source_attempt_id.clone());
        }

        let output = Self {
            record_schema_version: VALIDATED_OUTPUT_RECORD_SCHEMA_VERSION,
            output_id: params.output_id,
            schema_id: params.schema_id,
            schema_version: params.schema_version,
            schema_fingerprint: params.schema_fingerprint,
            canonical_value_ref: params.canonical_value_ref,
            validation_report_refs: vec![report.to_ref()],
            validation_attempts: vec![report.validation_attempt_id.clone()],
            repair_attempts: params.repair_attempts,
            source_attempt_ids,
            content_refs,
            lineage: params.lineage,
            policy_refs,
            privacy: params.privacy,
            redacted_summary: params.redacted_summary,
        };
        output.validate_shape()?;
        Ok(output)
    }

    pub fn validate_shape(&self) -> Result<(), TypedOutputError> {
        if self.record_schema_version != VALIDATED_OUTPUT_RECORD_SCHEMA_VERSION {
            return Err(TypedOutputError::SchemaVersionUnsupported {
                record_schema_version: self.record_schema_version,
            });
        }
        if !is_sha256_fingerprint(self.schema_fingerprint.as_str()) {
            return Err(TypedOutputError::InvalidSchemaFingerprint);
        }
        if self.validation_report_refs.is_empty() {
            return Err(TypedOutputError::MissingValidationReport {
                output_id: self.output_id.clone(),
            });
        }
        if self.source_attempt_ids.is_empty() {
            return Err(TypedOutputError::MissingSourceAttempt {
                output_id: self.output_id.clone(),
            });
        }
        if self.redacted_summary.is_empty() {
            return Err(TypedOutputError::MissingRedactedSummary {
                output_id: self.output_id.clone(),
            });
        }
        if !content_refs_include(&self.content_refs, &self.canonical_value_ref) {
            return Err(TypedOutputError::MissingCanonicalContentRef {
                output_id: self.output_id.clone(),
            });
        }
        for report_ref in &self.validation_report_refs {
            if !report_ref.status.is_success() {
                return Err(TypedOutputError::ValidationReportFailed {
                    validation_attempt_id: report_ref.validation_attempt_id.clone(),
                });
            }
            if !content_refs_include(&self.content_refs, &report_ref.report_ref) {
                return Err(TypedOutputError::MissingValidationReport {
                    output_id: self.output_id.clone(),
                });
            }
        }
        Ok(())
    }

    pub fn validation_report_keys(&self) -> Vec<String> {
        self.validation_report_refs
            .iter()
            .map(|report_ref| content_ref_key(&report_ref.report_ref))
            .collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedOutputParams {
    pub output_id: ValidatedOutputId,
    pub schema_id: OutputSchemaId,
    pub schema_version: SchemaVersion,
    pub schema_fingerprint: ContentHash,
    pub canonical_value_ref: ContentRef,
    pub repair_attempts: Vec<RepairAttemptId>,
    pub source_attempt_ids: Vec<AttemptId>,
    pub content_refs: Vec<ContentRef>,
    pub lineage: OutputLineage,
    pub policy_refs: Vec<PolicyRef>,
    pub privacy: PrivacyClass,
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OutputLineage {
    pub lineage_ref: LineageRef,
    pub produced_by: EntityRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub derived_from: Vec<EntityRef>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ValidationReportRef {
    pub validation_attempt_id: ValidationAttemptId,
    pub report_ref: ContentRef,
    pub status: ValidationStatus,
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ValidationReportRecord {
    pub record_schema_version: u16,
    pub validation_attempt_id: ValidationAttemptId,
    pub schema_id: OutputSchemaId,
    pub schema_version: SchemaVersion,
    pub source_attempt_id: AttemptId,
    pub candidate_content_ref: ContentRef,
    pub validation_report_ref: ContentRef,
    pub status: ValidationStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redacted_error_summary: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policy_refs: Vec<PolicyRef>,
    pub privacy: PrivacyClass,
    pub redacted_summary: String,
}

impl ValidationReportRecord {
    pub fn passed(
        validation_attempt_id: ValidationAttemptId,
        schema_id: OutputSchemaId,
        schema_version: SchemaVersion,
        source_attempt_id: AttemptId,
        candidate_content_ref: ContentRef,
        validation_report_ref: ContentRef,
        redacted_summary: impl Into<String>,
    ) -> Self {
        Self {
            record_schema_version: VALIDATION_REPORT_RECORD_SCHEMA_VERSION,
            validation_attempt_id,
            schema_id,
            schema_version,
            source_attempt_id,
            candidate_content_ref,
            validation_report_ref,
            status: ValidationStatus::Passed,
            redacted_error_summary: None,
            policy_refs: Vec::new(),
            privacy: PrivacyClass::ContentRefsOnly,
            redacted_summary: redacted_summary.into(),
        }
    }

    pub fn failed(
        validation_attempt_id: ValidationAttemptId,
        schema_id: OutputSchemaId,
        schema_version: SchemaVersion,
        source_attempt_id: AttemptId,
        candidate_content_ref: ContentRef,
        validation_report_ref: ContentRef,
        redacted_error_summary: impl Into<String>,
    ) -> Self {
        let redacted_error_summary = redacted_error_summary.into();
        Self {
            record_schema_version: VALIDATION_REPORT_RECORD_SCHEMA_VERSION,
            validation_attempt_id,
            schema_id,
            schema_version,
            source_attempt_id,
            candidate_content_ref,
            validation_report_ref,
            status: ValidationStatus::Failed,
            redacted_error_summary: Some(redacted_error_summary.clone()),
            policy_refs: Vec::new(),
            privacy: PrivacyClass::ContentRefsOnly,
            redacted_summary: redacted_error_summary,
        }
    }

    pub fn to_ref(&self) -> ValidationReportRef {
        ValidationReportRef {
            validation_attempt_id: self.validation_attempt_id.clone(),
            report_ref: self.validation_report_ref.clone(),
            status: self.status,
            redacted_summary: self.redacted_summary.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStatus {
    Passed,
    Failed,
}

impl ValidationStatus {
    pub fn is_success(self) -> bool {
        matches!(self, Self::Passed)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TypedResultPublicationRecord {
    pub record_schema_version: u16,
    pub validated_output_id: ValidatedOutputId,
    pub schema_id: OutputSchemaId,
    pub schema_version: SchemaVersion,
    pub canonical_value_ref: ContentRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub validation_report_refs: Vec<ValidationReportRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_attempt_ids: Vec<AttemptId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policy_refs: Vec<PolicyRef>,
    pub status: TypedResultPublicationStatus,
    pub privacy: PrivacyClass,
    pub redacted_summary: String,
}

impl TypedResultPublicationRecord {
    pub fn published(validated_output: &ValidatedOutput) -> Result<Self, TypedOutputError> {
        validated_output.validate_shape()?;
        Ok(Self {
            record_schema_version: TYPED_RESULT_PUBLICATION_RECORD_SCHEMA_VERSION,
            validated_output_id: validated_output.output_id.clone(),
            schema_id: validated_output.schema_id.clone(),
            schema_version: validated_output.schema_version,
            canonical_value_ref: validated_output.canonical_value_ref.clone(),
            validation_report_refs: validated_output.validation_report_refs.clone(),
            source_attempt_ids: validated_output.source_attempt_ids.clone(),
            policy_refs: validated_output.policy_refs.clone(),
            status: TypedResultPublicationStatus::Published,
            privacy: validated_output.privacy,
            redacted_summary: validated_output.redacted_summary.clone(),
        })
    }

    pub fn policy_denied(
        validated_output: &ValidatedOutput,
        redacted_summary: impl Into<String>,
    ) -> Result<Self, TypedOutputError> {
        validated_output.validate_shape()?;
        Ok(Self {
            record_schema_version: TYPED_RESULT_PUBLICATION_RECORD_SCHEMA_VERSION,
            validated_output_id: validated_output.output_id.clone(),
            schema_id: validated_output.schema_id.clone(),
            schema_version: validated_output.schema_version,
            canonical_value_ref: validated_output.canonical_value_ref.clone(),
            validation_report_refs: validated_output.validation_report_refs.clone(),
            source_attempt_ids: validated_output.source_attempt_ids.clone(),
            policy_refs: validated_output.policy_refs.clone(),
            status: TypedResultPublicationStatus::PolicyDenied,
            privacy: validated_output.privacy,
            redacted_summary: redacted_summary.into(),
        })
    }

    pub fn validate_against_output(
        &self,
        validated_output: &ValidatedOutput,
    ) -> Result<(), TypedOutputError> {
        if self.record_schema_version != TYPED_RESULT_PUBLICATION_RECORD_SCHEMA_VERSION {
            return Err(TypedOutputError::SchemaVersionUnsupported {
                record_schema_version: self.record_schema_version,
            });
        }
        if self.status != TypedResultPublicationStatus::Published {
            return Err(TypedOutputError::PublicationPolicyDenied {
                validated_output_id: self.validated_output_id.clone(),
            });
        }
        validated_output.validate_shape()?;
        if self.validation_report_refs.is_empty() {
            return Err(TypedOutputError::MissingValidationReport {
                output_id: validated_output.output_id.clone(),
            });
        }
        if self.validated_output_id != validated_output.output_id
            || self.schema_id != validated_output.schema_id
            || self.schema_version != validated_output.schema_version
            || content_ref_key(&self.canonical_value_ref)
                != content_ref_key(&validated_output.canonical_value_ref)
            || self.validation_report_refs != validated_output.validation_report_refs
        {
            return Err(TypedOutputError::PublicationEvidenceMismatch {
                validated_output_id: self.validated_output_id.clone(),
            });
        }
        Ok(())
    }

    pub fn validation_report_keys(&self) -> Vec<String> {
        self.validation_report_refs
            .iter()
            .map(|report_ref| content_ref_key(&report_ref.report_ref))
            .collect()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TypedResultPublicationStatus {
    Published,
    PolicyDenied,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct DecodedTypedOutput<T> {
    pub content_ref: ContentRef,
    pub output: T,
}

impl<T> DecodedTypedOutput<T> {
    pub fn new(content_ref: ContentRef, output: T) -> Self {
        Self {
            content_ref,
            output,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct StructuredOutputResult<T> {
    pub schema_id: OutputSchemaId,
    pub schema_version: SchemaVersion,
    pub validated_output_id: ValidatedOutputId,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub validation_attempts: Vec<ValidationAttemptId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub repair_attempts: Vec<RepairAttemptId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_attempt_ids: Vec<AttemptId>,
    pub output: T,
    pub output_ref: ContentRef,
    pub lineage: OutputLineage,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policy_refs: Vec<PolicyRef>,
    pub privacy: PrivacyClass,
    pub redacted_summary: String,
}

impl<T> StructuredOutputResult<T> {
    pub fn from_publication<D>(
        validated_output: &ValidatedOutput,
        publication: &TypedResultPublicationRecord,
        deserializer: &D,
    ) -> Result<Self, TypedOutputError>
    where
        D: TypedOutputDeserializer<T>,
    {
        publication.validate_against_output(validated_output)?;
        let decoded = deserializer.deserialize(&validated_output.canonical_value_ref)?;
        if content_ref_key(&decoded.content_ref)
            != content_ref_key(&validated_output.canonical_value_ref)
        {
            return Err(TypedOutputError::CanonicalValueRefMismatch {
                validated_output_id: validated_output.output_id.clone(),
            });
        }
        Ok(Self {
            schema_id: validated_output.schema_id.clone(),
            schema_version: validated_output.schema_version,
            validated_output_id: validated_output.output_id.clone(),
            validation_attempts: validated_output.validation_attempts.clone(),
            repair_attempts: validated_output.repair_attempts.clone(),
            source_attempt_ids: validated_output.source_attempt_ids.clone(),
            output: decoded.output,
            output_ref: validated_output.canonical_value_ref.clone(),
            lineage: validated_output.lineage.clone(),
            policy_refs: validated_output.policy_refs.clone(),
            privacy: validated_output.privacy,
            redacted_summary: validated_output.redacted_summary.clone(),
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", content = "record", rename_all = "snake_case")]
pub enum ValidatedOutputPublicationStep {
    ValidationReport(ValidationReportRecord),
    ValidatedOutput(ValidatedOutput),
    TypedResultPublication(TypedResultPublicationRecord),
}

pub fn validate_typed_result_publication_order(
    steps: &[ValidatedOutputPublicationStep],
) -> Result<(), TypedOutputError> {
    let mut successful_reports = BTreeMap::<String, ValidationReportRef>::new();
    let mut validated_outputs = BTreeMap::<String, BTreeMap<String, ValidationReportRef>>::new();

    for step in steps {
        match step {
            ValidatedOutputPublicationStep::ValidationReport(report) => {
                if report.status.is_success() {
                    successful_reports.insert(
                        content_ref_key(&report.validation_report_ref),
                        report.to_ref(),
                    );
                }
            }
            ValidatedOutputPublicationStep::ValidatedOutput(output) => {
                output.validate_shape()?;
                let output_report_refs = validation_report_ref_map(&output.validation_report_refs);
                for (report_key, output_report_ref) in &output_report_refs {
                    let Some(successful_report_ref) = successful_reports.get(report_key) else {
                        return Err(TypedOutputError::PublicationBeforeValidation {
                            validated_output_id: output.output_id.clone(),
                        });
                    };
                    if successful_report_ref != output_report_ref {
                        return Err(TypedOutputError::PublicationEvidenceMismatch {
                            validated_output_id: output.output_id.clone(),
                        });
                    }
                }
                validated_outputs.insert(output.output_id.as_str().to_string(), output_report_refs);
            }
            ValidatedOutputPublicationStep::TypedResultPublication(publication) => {
                let Some(output_report_refs) =
                    validated_outputs.get(publication.validated_output_id.as_str())
                else {
                    return Err(TypedOutputError::PublicationBeforeValidation {
                        validated_output_id: publication.validated_output_id.clone(),
                    });
                };

                let publication_report_keys = publication
                    .validation_report_keys()
                    .into_iter()
                    .collect::<BTreeSet<_>>();
                if publication_report_keys.is_empty() {
                    return Err(TypedOutputError::MissingValidationReport {
                        output_id: publication.validated_output_id.clone(),
                    });
                }
                let publication_report_refs =
                    validation_report_ref_map(&publication.validation_report_refs);
                if &publication_report_refs != output_report_refs {
                    return Err(TypedOutputError::PublicationEvidenceMismatch {
                        validated_output_id: publication.validated_output_id.clone(),
                    });
                }

                for report_ref in &publication.validation_report_refs {
                    let report_key = content_ref_key(&report_ref.report_ref);
                    if !successful_reports.contains_key(&report_key)
                        || !output_report_refs.contains_key(&report_key)
                    {
                        return Err(TypedOutputError::PublicationBeforeValidation {
                            validated_output_id: publication.validated_output_id.clone(),
                        });
                    }
                }
            }
        }
    }

    Ok(())
}

#[derive(Clone, Debug, Deserialize, Error, Eq, PartialEq, Serialize)]
pub enum TypedOutputError {
    #[error("validated output record schema version {record_schema_version} is unsupported")]
    SchemaVersionUnsupported { record_schema_version: u16 },
    #[error("validated output schema fingerprint must be a sha256 digest")]
    InvalidSchemaFingerprint,
    #[error("validated output is missing validation report evidence")]
    MissingValidationReport { output_id: ValidatedOutputId },
    #[error("validated output is missing a source model attempt")]
    MissingSourceAttempt { output_id: ValidatedOutputId },
    #[error("validated output is missing its canonical content ref")]
    MissingCanonicalContentRef { output_id: ValidatedOutputId },
    #[error("validated output is missing a redacted summary")]
    MissingRedactedSummary { output_id: ValidatedOutputId },
    #[error("validation report did not pass")]
    ValidationReportFailed {
        validation_attempt_id: ValidationAttemptId,
    },
    #[error("validated output schema does not match validation report schema")]
    SchemaMismatch {
        expected_schema_id: OutputSchemaId,
        actual_schema_id: OutputSchemaId,
    },
    #[error("typed result publication happened before validated output evidence")]
    PublicationBeforeValidation {
        validated_output_id: ValidatedOutputId,
    },
    #[error("validated output publication was denied by output policy")]
    PublicationPolicyDenied {
        validated_output_id: ValidatedOutputId,
    },
    #[error("typed result publication evidence does not match validated output")]
    PublicationEvidenceMismatch {
        validated_output_id: ValidatedOutputId,
    },
    #[error("typed result decoder returned content from a different canonical value ref")]
    CanonicalValueRefMismatch {
        validated_output_id: ValidatedOutputId,
    },
    #[error("run {run_id:?} does not contain validated structured output")]
    MissingValidatedOutput { run_id: RunId },
}

impl TypedOutputError {
    pub fn retry_classification(&self) -> RetryClassification {
        match self {
            Self::PublicationPolicyDenied { .. } | Self::ValidationReportFailed { .. } => {
                RetryClassification::NotRetryable
            }
            Self::SchemaVersionUnsupported { .. }
            | Self::InvalidSchemaFingerprint
            | Self::MissingValidationReport { .. }
            | Self::MissingSourceAttempt { .. }
            | Self::MissingCanonicalContentRef { .. }
            | Self::MissingRedactedSummary { .. }
            | Self::SchemaMismatch { .. }
            | Self::PublicationBeforeValidation { .. }
            | Self::PublicationEvidenceMismatch { .. }
            | Self::CanonicalValueRefMismatch { .. }
            | Self::MissingValidatedOutput { .. } => RetryClassification::RepairNeeded,
        }
    }
}

impl From<TypedOutputError> for AgentError {
    fn from(error: TypedOutputError) -> Self {
        AgentError::new(
            AgentErrorKind::StructuredOutputFailure,
            error.retry_classification(),
            error.to_string(),
        )
    }
}

fn push_unique_content_ref(content_refs: &mut Vec<ContentRef>, content_ref: ContentRef) {
    if !content_refs_include(content_refs, &content_ref) {
        content_refs.push(content_ref);
    }
}

fn push_unique_policy_ref(policy_refs: &mut Vec<PolicyRef>, policy_ref: PolicyRef) {
    if !policy_refs.contains(&policy_ref) {
        policy_refs.push(policy_ref);
    }
}

fn content_refs_include(content_refs: &[ContentRef], expected: &ContentRef) -> bool {
    let expected_key = content_ref_key(expected);
    content_refs
        .iter()
        .any(|content_ref| content_ref_key(content_ref) == expected_key)
}

fn validation_report_ref_map(
    report_refs: &[ValidationReportRef],
) -> BTreeMap<String, ValidationReportRef> {
    report_refs
        .iter()
        .map(|report_ref| (content_ref_key(&report_ref.report_ref), report_ref.clone()))
        .collect()
}

fn content_ref_key(content_ref: &ContentRef) -> String {
    format!(
        "{}@{}",
        content_ref.content_id.as_str(),
        content_ref.version.as_str()
    )
}

fn is_sha256_fingerprint(value: &str) -> bool {
    let Some(digest) = value.strip_prefix("sha256:") else {
        return false;
    };
    digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
}
