use crate as sdk;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use sdk::{
    AgentError, AgentErrorKind, AttemptId, ContentHash, OutputContract, OutputMode,
    OutputSchemaDialect, OutputSchemaId, OutputSchemaRef, PrivacyClass, RetryClassification,
    SchemaVersion, ValidationAttemptId,
    domain::ContentRef,
    structured_output::{
        VALIDATION_RECORD_SCHEMA_VERSION, ValidationErrorCode, ValidationErrorSummary,
        ValidationRecord, ValidationRecordKind,
    },
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OutputCandidate {
    pub source_attempt_id: AttemptId,
    pub candidate_content_ref: ContentRef,
    pub text: String,
    pub privacy: PrivacyClass,
}

impl OutputCandidate {
    pub fn new(
        source_attempt_id: AttemptId,
        candidate_content_ref: ContentRef,
        text: impl Into<String>,
    ) -> Self {
        Self {
            source_attempt_id,
            candidate_content_ref,
            text: text.into(),
            privacy: PrivacyClass::ContentRefsOnly,
        }
    }

    pub fn with_privacy(mut self, privacy: PrivacyClass) -> Self {
        self.privacy = privacy;
        self
    }
}

pub trait StructuredOutputValidator {
    fn validate_candidate(
        &self,
        contract: &OutputContract,
        validation_attempt_id: ValidationAttemptId,
        candidate: &OutputCandidate,
    ) -> Result<ValidationSuccess, ValidationErrorReport>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JsonSchemaSubsetValidator {
    pub schema_limits: HostileSchemaLimits,
}

impl JsonSchemaSubsetValidator {
    pub fn new(schema_limits: HostileSchemaLimits) -> Self {
        Self { schema_limits }
    }
}

impl Default for JsonSchemaSubsetValidator {
    fn default() -> Self {
        Self::new(HostileSchemaLimits::default())
    }
}

impl StructuredOutputValidator for JsonSchemaSubsetValidator {
    fn validate_candidate(
        &self,
        contract: &OutputContract,
        validation_attempt_id: ValidationAttemptId,
        candidate: &OutputCandidate,
    ) -> Result<ValidationSuccess, ValidationErrorReport> {
        let mut schema_errors = ErrorCollector::new(contract.validation.max_errors_returned);

        if contract.dialect != OutputSchemaDialect::JsonSchema2020_12Subset {
            schema_errors.push(ValidationErrorSummary::new(
                ValidationErrorCode::UnsupportedDialect,
                "/dialect",
                "unsupported structured output schema dialect",
            ));
        }

        if contract.mode != OutputMode::FinalOnly {
            schema_errors.push(ValidationErrorSummary::new(
                ValidationErrorCode::SchemaContractViolation,
                "/mode",
                "incremental structured output validation is not owned by this phase",
            ));
        }

        if let Err(error) = contract.validate_shape() {
            schema_errors.push(ValidationErrorSummary::new(
                ValidationErrorCode::SchemaContractViolation,
                "/",
                error.context().message,
            ));
        }

        let inline_schema = match &contract.schema {
            OutputSchemaRef::InlineJson {
                redacted_schema, ..
            } => Some(redacted_schema),
            _ => {
                schema_errors.push(ValidationErrorSummary::new(
                    ValidationErrorCode::UnsupportedSchemaRef,
                    "/schema",
                    "schema content must be inline for local validation in this phase",
                ));
                None
            }
        };

        if let Some(schema) = inline_schema {
            validate_schema_limits(schema, &self.schema_limits, &mut schema_errors);
        }

        if !contract.validation.semantic_validators.is_empty() {
            schema_errors.push(ValidationErrorSummary::new(
                ValidationErrorCode::SemanticValidatorUnavailable,
                "/validation/semantic_validators",
                "host semantic validators are referenced but no local validator was supplied",
            ));
        }

        if !schema_errors.is_empty() {
            let schema_rejected = schema_errors
                .errors
                .iter()
                .any(|error| error.code.is_schema_rejection());
            return Err(ValidationErrorReport::new(
                contract,
                validation_attempt_id,
                candidate,
                schema_errors.into_errors(),
                schema_rejected,
            ));
        }

        if candidate.text.as_bytes().len() as u64 > contract.validation.max_candidate_bytes {
            return Err(ValidationErrorReport::new(
                contract,
                validation_attempt_id,
                candidate,
                vec![ValidationErrorSummary::new(
                    ValidationErrorCode::CandidateTooLarge,
                    "/",
                    "candidate exceeds configured structured output byte limit",
                )],
                false,
            ));
        }

        let value = match serde_json::from_str::<Value>(&candidate.text) {
            Ok(value) => value,
            Err(_) => {
                return Err(ValidationErrorReport::new(
                    contract,
                    validation_attempt_id,
                    candidate,
                    vec![ValidationErrorSummary::new(
                        ValidationErrorCode::InvalidJson,
                        "/",
                        "candidate is not valid JSON",
                    )],
                    false,
                ));
            }
        };

        let schema = inline_schema.expect("inline schema checked above");
        let mut candidate_errors = ErrorCollector::new(contract.validation.max_errors_returned);
        validate_value_against_schema(
            schema,
            &value,
            "/",
            contract.validation.allow_additional_properties,
            &mut candidate_errors,
        );

        if !candidate_errors.is_empty() {
            return Err(ValidationErrorReport::new(
                contract,
                validation_attempt_id,
                candidate,
                candidate_errors.into_errors(),
                false,
            ));
        }

        let record = validation_record_succeeded(
            contract,
            validation_attempt_id.clone(),
            candidate,
            "structured output candidate validated locally",
        );

        Ok(ValidationSuccess {
            schema_id: contract.schema_id.clone(),
            schema_version: contract.schema_version,
            schema_fingerprint: contract.schema_fingerprint(),
            validation_attempt_id,
            source_attempt_id: candidate.source_attempt_id.clone(),
            candidate_content_ref: candidate.candidate_content_ref.clone(),
            canonical_value: value,
            privacy: candidate.privacy,
            record,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HostileSchemaLimits {
    pub max_schema_bytes: usize,
    pub max_object_depth: usize,
    pub max_properties_per_object: usize,
    pub max_enum_values_per_field: usize,
    pub max_string_pattern_bytes: usize,
    pub allow_remote_refs: bool,
    pub allow_custom_formats: bool,
}

impl Default for HostileSchemaLimits {
    fn default() -> Self {
        Self {
            max_schema_bytes: 64 * 1024,
            max_object_depth: 24,
            max_properties_per_object: 256,
            max_enum_values_per_field: 512,
            max_string_pattern_bytes: 2 * 1024,
            allow_remote_refs: false,
            allow_custom_formats: false,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ValidationSuccess {
    pub schema_id: OutputSchemaId,
    pub schema_version: SchemaVersion,
    pub schema_fingerprint: ContentHash,
    pub validation_attempt_id: ValidationAttemptId,
    pub source_attempt_id: AttemptId,
    pub candidate_content_ref: ContentRef,
    pub canonical_value: Value,
    pub privacy: PrivacyClass,
    pub record: ValidationRecord,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ValidationErrorReport {
    pub schema_id: OutputSchemaId,
    pub schema_version: SchemaVersion,
    pub schema_fingerprint: ContentHash,
    pub validation_attempt_id: ValidationAttemptId,
    pub source_attempt_id: AttemptId,
    pub candidate_content_ref: ContentRef,
    pub errors: Vec<ValidationErrorSummary>,
    pub redacted_error_summary: String,
    pub schema_rejected: bool,
    pub retry_exhausted: bool,
    pub privacy: PrivacyClass,
    pub record: ValidationRecord,
}

impl ValidationErrorReport {
    fn new(
        contract: &OutputContract,
        validation_attempt_id: ValidationAttemptId,
        candidate: &OutputCandidate,
        errors: Vec<ValidationErrorSummary>,
        schema_rejected: bool,
    ) -> Self {
        let redacted_error_summary = summarize_errors(&errors);
        let record = if schema_rejected {
            validation_record_schema_rejected(
                contract,
                validation_attempt_id.clone(),
                candidate,
                redacted_error_summary.clone(),
                errors.clone(),
            )
        } else {
            validation_record_failed(
                contract,
                validation_attempt_id.clone(),
                candidate,
                redacted_error_summary.clone(),
                errors.clone(),
            )
        };

        Self {
            schema_id: contract.schema_id.clone(),
            schema_version: contract.schema_version,
            schema_fingerprint: contract.schema_fingerprint(),
            validation_attempt_id,
            source_attempt_id: candidate.source_attempt_id.clone(),
            candidate_content_ref: candidate.candidate_content_ref.clone(),
            errors,
            redacted_error_summary,
            schema_rejected,
            retry_exhausted: false,
            privacy: candidate.privacy,
            record,
        }
    }

    pub fn as_agent_error(&self) -> AgentError {
        let mut error = AgentError::new(
            AgentErrorKind::StructuredOutputFailure,
            if self.retry_exhausted || self.schema_rejected {
                RetryClassification::NotRetryable
            } else {
                RetryClassification::RepairNeeded
            },
            self.redacted_error_summary.clone(),
        );
        error = error.with_policy_ref(sdk::PolicyRef::with_kind(
            sdk::PolicyKind::RuntimePackage,
            self.schema_id.as_str(),
        ));
        error
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TerminalValidationFailure {
    pub schema_id: OutputSchemaId,
    pub schema_version: SchemaVersion,
    pub validation_attempts: Vec<ValidationAttemptId>,
    pub repair_attempts: Vec<sdk::RepairAttemptId>,
    pub source_attempt_ids: Vec<AttemptId>,
    pub redacted_error_summary: String,
    pub candidate_content_ref: ContentRef,
    pub retry_exhausted: bool,
    pub privacy: PrivacyClass,
    pub record: ValidationRecord,
}

impl TerminalValidationFailure {
    pub fn from_reports(
        reports: &[ValidationErrorReport],
        repair_attempts: Vec<sdk::RepairAttemptId>,
        retry_exhausted: bool,
    ) -> Self {
        let last = reports
            .last()
            .expect("terminal validation failure needs at least one report");
        let validation_attempts = reports
            .iter()
            .map(|report| report.validation_attempt_id.clone())
            .collect::<Vec<_>>();
        let mut source_attempt_ids = Vec::new();
        for report in reports {
            if !source_attempt_ids.contains(&report.source_attempt_id) {
                source_attempt_ids.push(report.source_attempt_id.clone());
            }
        }
        let redacted_error_summary = if retry_exhausted {
            format!(
                "structured output validation failed after {} validation attempt(s) and {} repair attempt(s): {}",
                validation_attempts.len(),
                repair_attempts.len(),
                last.redacted_error_summary
            )
        } else {
            last.redacted_error_summary.clone()
        };
        let record = validation_record_terminal_failure(
            last,
            validation_attempts.clone(),
            repair_attempts.clone(),
            source_attempt_ids.clone(),
            redacted_error_summary.clone(),
            retry_exhausted,
        );
        Self {
            schema_id: last.schema_id.clone(),
            schema_version: last.schema_version,
            validation_attempts,
            repair_attempts,
            source_attempt_ids,
            redacted_error_summary,
            candidate_content_ref: last.candidate_content_ref.clone(),
            retry_exhausted,
            privacy: last.privacy,
            record,
        }
    }

    pub fn as_agent_error(&self) -> AgentError {
        AgentError::new(
            AgentErrorKind::StructuredOutputFailure,
            RetryClassification::NotRetryable,
            self.redacted_error_summary.clone(),
        )
    }
}

fn validation_record_base(
    contract: &OutputContract,
    record_kind: ValidationRecordKind,
    validation_attempt_id: ValidationAttemptId,
    candidate: &OutputCandidate,
    redacted_summary: String,
) -> ValidationRecord {
    ValidationRecord {
        record_schema_version: VALIDATION_RECORD_SCHEMA_VERSION,
        record_kind,
        schema_id: contract.schema_id.clone(),
        output_schema_version: contract.schema_version,
        schema_fingerprint: contract.schema_fingerprint(),
        validation_attempt_id,
        source_attempt_id: candidate.source_attempt_id.clone(),
        candidate_content_ref: candidate.candidate_content_ref.clone(),
        privacy: candidate.privacy,
        redacted_summary,
        errors: Vec::new(),
        validation_attempts: Vec::new(),
        repair_attempts: Vec::new(),
        source_attempt_ids: Vec::new(),
        retry_exhausted: None,
    }
}

fn validation_record_succeeded(
    contract: &OutputContract,
    validation_attempt_id: ValidationAttemptId,
    candidate: &OutputCandidate,
    redacted_summary: impl Into<String>,
) -> ValidationRecord {
    validation_record_base(
        contract,
        ValidationRecordKind::ValidationSucceeded,
        validation_attempt_id,
        candidate,
        redacted_summary.into(),
    )
}

fn validation_record_failed(
    contract: &OutputContract,
    validation_attempt_id: ValidationAttemptId,
    candidate: &OutputCandidate,
    redacted_summary: String,
    errors: Vec<ValidationErrorSummary>,
) -> ValidationRecord {
    let mut record = validation_record_base(
        contract,
        ValidationRecordKind::ValidationFailed,
        validation_attempt_id,
        candidate,
        redacted_summary,
    );
    record.errors = errors;
    record
}

fn validation_record_schema_rejected(
    contract: &OutputContract,
    validation_attempt_id: ValidationAttemptId,
    candidate: &OutputCandidate,
    redacted_summary: String,
    errors: Vec<ValidationErrorSummary>,
) -> ValidationRecord {
    let mut record = validation_record_base(
        contract,
        ValidationRecordKind::SchemaRejected,
        validation_attempt_id,
        candidate,
        redacted_summary,
    );
    record.errors = errors;
    record
}

fn validation_record_terminal_failure(
    last_report: &ValidationErrorReport,
    validation_attempts: Vec<ValidationAttemptId>,
    repair_attempts: Vec<sdk::RepairAttemptId>,
    source_attempt_ids: Vec<AttemptId>,
    redacted_summary: String,
    retry_exhausted: bool,
) -> ValidationRecord {
    ValidationRecord {
        record_schema_version: VALIDATION_RECORD_SCHEMA_VERSION,
        record_kind: ValidationRecordKind::TerminalFailure,
        schema_id: last_report.schema_id.clone(),
        output_schema_version: last_report.schema_version,
        schema_fingerprint: last_report.schema_fingerprint.clone(),
        validation_attempt_id: last_report.validation_attempt_id.clone(),
        source_attempt_id: last_report.source_attempt_id.clone(),
        candidate_content_ref: last_report.candidate_content_ref.clone(),
        privacy: last_report.privacy,
        redacted_summary,
        errors: last_report.errors.clone(),
        validation_attempts,
        repair_attempts,
        source_attempt_ids,
        retry_exhausted: Some(retry_exhausted),
    }
}

struct ErrorCollector {
    max: usize,
    errors: Vec<ValidationErrorSummary>,
}

impl ErrorCollector {
    fn new(max_errors_returned: u16) -> Self {
        Self {
            max: usize::from(max_errors_returned.max(1)),
            errors: Vec::new(),
        }
    }

    fn push(&mut self, error: ValidationErrorSummary) {
        if self.errors.len() < self.max {
            self.errors.push(error);
        }
    }

    fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    fn into_errors(self) -> Vec<ValidationErrorSummary> {
        self.errors
    }
}

fn summarize_errors(errors: &[ValidationErrorSummary]) -> String {
    match errors {
        [] => "structured output validation produced no errors".to_string(),
        [error] => format!(
            "structured output validation failed: {} at {}",
            error.redacted_summary, error.path
        ),
        [first, rest @ ..] => format!(
            "structured output validation failed with {} error(s): {} at {}",
            rest.len() + 1,
            first.redacted_summary,
            first.path
        ),
    }
}

fn validate_schema_limits(
    schema: &Value,
    limits: &HostileSchemaLimits,
    errors: &mut ErrorCollector,
) {
    let schema_bytes = serde_json::to_vec(schema).expect("serde_json::Value serializes");
    if schema_bytes.len() > limits.max_schema_bytes {
        errors.push(ValidationErrorSummary::new(
            ValidationErrorCode::HostileSchema,
            "/schema",
            "schema exceeds maximum byte limit",
        ));
    }
    inspect_schema_node(schema, "/", 0, limits, errors);
}

fn inspect_schema_node(
    value: &Value,
    path: &str,
    depth: usize,
    limits: &HostileSchemaLimits,
    errors: &mut ErrorCollector,
) {
    if depth > limits.max_object_depth {
        errors.push(ValidationErrorSummary::new(
            ValidationErrorCode::HostileSchema,
            path,
            "schema exceeds maximum object depth",
        ));
        return;
    }

    match value {
        Value::Object(fields) => {
            if let Some(Value::String(reference)) = fields.get("$ref") {
                let is_remote = reference.starts_with("http://")
                    || reference.starts_with("https://")
                    || !reference.starts_with('#');
                if is_remote && !limits.allow_remote_refs {
                    errors.push(ValidationErrorSummary::new(
                        ValidationErrorCode::HostileSchema,
                        join_path(path, "$ref"),
                        "remote or external schema references are denied",
                    ));
                } else {
                    errors.push(ValidationErrorSummary::new(
                        ValidationErrorCode::SchemaContractViolation,
                        join_path(path, "$ref"),
                        "local schema references are not supported by this validator phase",
                    ));
                }
            }

            if fields.contains_key("format") && !limits.allow_custom_formats {
                errors.push(ValidationErrorSummary::new(
                    ValidationErrorCode::HostileSchema,
                    join_path(path, "format"),
                    "custom format validators are disabled by default",
                ));
            }

            if let Some(Value::String(pattern)) = fields.get("pattern") {
                if pattern.len() > limits.max_string_pattern_bytes {
                    errors.push(ValidationErrorSummary::new(
                        ValidationErrorCode::HostileSchema,
                        join_path(path, "pattern"),
                        "schema string pattern exceeds maximum byte limit",
                    ));
                }
            }

            if let Some(Value::Object(properties)) = fields.get("properties") {
                if properties.len() > limits.max_properties_per_object {
                    errors.push(ValidationErrorSummary::new(
                        ValidationErrorCode::HostileSchema,
                        join_path(path, "properties"),
                        "schema object has too many properties",
                    ));
                }
            }

            if let Some(Value::Array(values)) = fields.get("enum") {
                if values.len() > limits.max_enum_values_per_field {
                    errors.push(ValidationErrorSummary::new(
                        ValidationErrorCode::HostileSchema,
                        join_path(path, "enum"),
                        "schema enum has too many values",
                    ));
                }
            }

            for (field, child) in fields {
                inspect_schema_node(child, &join_path(path, field), depth + 1, limits, errors);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                inspect_schema_node(
                    child,
                    &join_path(path, &index.to_string()),
                    depth + 1,
                    limits,
                    errors,
                );
            }
        }
        _ => {}
    }
}

fn validate_value_against_schema(
    schema: &Value,
    value: &Value,
    path: &str,
    allow_additional_properties: bool,
    errors: &mut ErrorCollector,
) {
    if let Some(enum_values) = schema.get("enum").and_then(Value::as_array) {
        if !enum_values.iter().any(|enum_value| enum_value == value) {
            errors.push(ValidationErrorSummary::new(
                ValidationErrorCode::EnumMismatch,
                path,
                "candidate value is not one of the allowed schema values",
            ));
            return;
        }
    }

    let schema_type = schema.get("type").and_then(Value::as_str);
    match schema_type {
        Some("object") => {
            validate_object_schema(schema, value, path, allow_additional_properties, errors)
        }
        Some("array") => {
            validate_array_schema(schema, value, path, allow_additional_properties, errors)
        }
        Some("string") => validate_string_schema(schema, value, path, errors),
        Some("integer") => {
            if !value.as_i64().is_some() && !value.as_u64().is_some() {
                errors.push(type_mismatch(path, "integer"));
            }
        }
        Some("number") => {
            if !value.is_number() {
                errors.push(type_mismatch(path, "number"));
            }
        }
        Some("boolean") => {
            if !value.is_boolean() {
                errors.push(type_mismatch(path, "boolean"));
            }
        }
        Some("null") => {
            if !value.is_null() {
                errors.push(type_mismatch(path, "null"));
            }
        }
        Some(_) => errors.push(ValidationErrorSummary::new(
            ValidationErrorCode::SchemaContractViolation,
            path,
            "schema type is outside the supported local subset",
        )),
        None => {
            if schema.get("properties").is_some() || schema.get("required").is_some() {
                validate_object_schema(schema, value, path, allow_additional_properties, errors);
            }
        }
    }
}

fn validate_object_schema(
    schema: &Value,
    value: &Value,
    path: &str,
    allow_additional_properties: bool,
    errors: &mut ErrorCollector,
) {
    let Some(object) = value.as_object() else {
        errors.push(type_mismatch(path, "object"));
        return;
    };

    let properties = schema.get("properties").and_then(Value::as_object);

    if let Some(required) = schema.get("required").and_then(Value::as_array) {
        for required_field in required.iter().filter_map(Value::as_str) {
            if !object.contains_key(required_field) {
                errors.push(ValidationErrorSummary::new(
                    ValidationErrorCode::MissingRequiredField,
                    join_path(path, required_field),
                    "required schema field is missing",
                ));
            }
        }
    }

    if let Some(properties) = properties {
        for (field, field_schema) in properties {
            if let Some(field_value) = object.get(field) {
                validate_value_against_schema(
                    field_schema,
                    field_value,
                    &join_path(path, field),
                    allow_additional_properties,
                    errors,
                );
            }
        }
    }

    let schema_denies_additional = schema
        .get("additionalProperties")
        .and_then(Value::as_bool)
        .is_some_and(|allowed| !allowed);
    let denies_additional = schema_denies_additional || !allow_additional_properties;
    if denies_additional {
        for field in object.keys() {
            let known = properties.is_some_and(|properties| properties.contains_key(field));
            if !known {
                errors.push(ValidationErrorSummary::new(
                    ValidationErrorCode::AdditionalPropertyDenied,
                    path,
                    "candidate contains an additional property denied by schema",
                ));
            }
        }
    }
}

fn validate_array_schema(
    schema: &Value,
    value: &Value,
    path: &str,
    allow_additional_properties: bool,
    errors: &mut ErrorCollector,
) {
    let Some(items) = value.as_array() else {
        errors.push(type_mismatch(path, "array"));
        return;
    };
    if let Some(item_schema) = schema.get("items").filter(|item| item.is_object()) {
        for (index, item) in items.iter().enumerate() {
            validate_value_against_schema(
                item_schema,
                item,
                &join_path(path, &index.to_string()),
                allow_additional_properties,
                errors,
            );
        }
    }
}

fn validate_string_schema(schema: &Value, value: &Value, path: &str, errors: &mut ErrorCollector) {
    let Some(text) = value.as_str() else {
        errors.push(type_mismatch(path, "string"));
        return;
    };

    if let Some(min) = schema.get("minLength").and_then(Value::as_u64) {
        if text.chars().count() < min as usize {
            errors.push(ValidationErrorSummary::new(
                ValidationErrorCode::MinLengthViolation,
                path,
                "candidate string is shorter than the schema minimum",
            ));
        }
    }
    if let Some(max) = schema.get("maxLength").and_then(Value::as_u64) {
        if text.chars().count() > max as usize {
            errors.push(ValidationErrorSummary::new(
                ValidationErrorCode::MaxLengthViolation,
                path,
                "candidate string is longer than the schema maximum",
            ));
        }
    }
}

fn type_mismatch(path: &str, expected: &str) -> ValidationErrorSummary {
    ValidationErrorSummary::new(
        ValidationErrorCode::TypeMismatch,
        path,
        format!("candidate value does not match schema type {expected}"),
    )
}

fn join_path(parent: &str, child: &str) -> String {
    if parent == "/" {
        format!("/{child}")
    } else {
        format!("{parent}/{child}")
    }
}
