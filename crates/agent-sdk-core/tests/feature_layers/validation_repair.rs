use serde::Serialize;
use serde_json::{Value, json};

use agent_sdk_core::ValidationRepairOutcome;
use agent_sdk_core::{
    AgentErrorKind, CandidateContentRepairPolicy, JsonSchemaSubsetValidator,
    LocalValidationRepairService, OutputCandidate, OutputContract, OutputSchemaId, PrivacyClass,
    RepairExhaustedBehavior, RetryBudget, RetryClassification, SchemaVersion,
    StructuredOutputRecord, StructuredOutputValidator, ValidationAttemptId, ValidationErrorCode,
    ValidationRecordKind,
    domain::{AttemptId, ContentRef},
    journal::JournalRecordPayload,
    testing::{normalize_json_value, read_fixture},
};

#[test]
fn valid_output_returns_local_validation_record() {
    let contract = todo_contract();
    let validator = JsonSchemaSubsetValidator::default();
    let candidate = candidate(
        "attempt.valid",
        "content.validation.valid",
        r#"{"title":"ship validation","priority":"high"}"#,
    );

    let success = validator
        .validate_candidate(
            &contract,
            ValidationAttemptId::new("validation.attempt.valid"),
            &candidate,
        )
        .expect("valid structured output");

    assert_eq!(success.canonical_value["title"], "ship validation");
    assert_eq!(
        normalized(&success.record),
        read_fixture("tests/fixtures/validation/valid-output-record.json")
            .expect("valid output fixture")
    );
}

#[test]
fn invalid_output_returns_redacted_validation_errors() {
    let contract = todo_contract();
    let validator = JsonSchemaSubsetValidator::default();
    let secret = "sk_live_secret_should_not_escape";
    let candidate = candidate(
        "attempt.invalid",
        "content.validation.invalid",
        format!(r#"{{"priority":"urgent","secret":"{secret}"}}"#),
    );

    let report = validator
        .validate_candidate(
            &contract,
            ValidationAttemptId::new("validation.attempt.invalid"),
            &candidate,
        )
        .expect_err("invalid output");

    assert_eq!(report.errors.len(), 3);
    assert_eq!(
        report.as_agent_error().kind(),
        AgentErrorKind::StructuredOutputFailure
    );
    assert_eq!(
        report.errors[0].code,
        ValidationErrorCode::MissingRequiredField
    );
    assert_eq!(report.errors[1].code, ValidationErrorCode::EnumMismatch);
    assert_eq!(
        normalized(&report.record),
        read_fixture("tests/fixtures/validation/invalid-output-record.json")
            .expect("invalid output fixture")
    );
    assert_no_raw_content(secret, &report);
}

#[test]
fn strict_validation_policy_denies_additional_properties_even_when_schema_omits_keyword() {
    let contract = OutputContract::inline_json_schema(
        OutputSchemaId::new("schema.strict_without_additional_properties"),
        SchemaVersion::new(1, 0, 0),
        json!({
            "type": "object",
            "required": ["title"],
            "properties": {
                "title": {"type": "string"}
            }
        }),
    );
    let validator = JsonSchemaSubsetValidator::default();
    let candidate = candidate(
        "attempt.strict_extra",
        "content.validation.strict_extra",
        r#"{"title":"ship validation","noise":"provider extra"}"#,
    );

    let report = validator
        .validate_candidate(
            &contract,
            ValidationAttemptId::new("validation.attempt.strict_extra"),
            &candidate,
        )
        .expect_err("strict policy denies provider noise without schema opt-in");

    assert!(
        report
            .errors
            .iter()
            .any(|error| error.code == ValidationErrorCode::AdditionalPropertyDenied)
    );
}

#[test]
fn repair_success_records_attempt_before_validated_candidate() {
    let contract = todo_contract();
    let service = LocalValidationRepairService::default_json_schema_subset();
    let outcome = service
        .validate_candidates(
            &contract,
            [
                candidate(
                    "attempt.repair_initial",
                    "content.validation.repair_initial",
                    r#"{"priority":"high"}"#,
                ),
                candidate(
                    "attempt.repair_fixed",
                    "content.validation.repair_fixed",
                    r#"{"title":"ship validation","priority":"high"}"#,
                ),
            ],
        )
        .expect("repair sequence runs");

    let ValidationRepairOutcome::Validated {
        success,
        validation_records,
        repair_records,
        repair_attempts,
    } = outcome
    else {
        panic!("expected repaired candidate to validate");
    };

    assert_eq!(success.canonical_value["title"], "ship validation");
    assert_eq!(validation_records.len(), 2);
    assert_eq!(repair_records.len(), 1);
    assert_eq!(repair_attempts.len(), 1);
    assert_eq!(
        normalized(&repair_records[0]),
        read_fixture("tests/fixtures/validation/repair-request-record.json")
            .expect("repair request fixture")
    );
}

#[test]
fn repair_exhaustion_returns_typed_terminal_failure() {
    let mut contract = todo_contract();
    contract.repair.max_repair_attempts = 1;
    contract.repair.on_exhausted = RepairExhaustedBehavior::ReturnValidationError;
    contract.retry_budget = RetryBudget::attempts(1);
    let service = LocalValidationRepairService::default_json_schema_subset();
    let outcome = service
        .validate_candidates(
            &contract,
            [
                candidate(
                    "attempt.exhaust_initial",
                    "content.validation.exhaust_initial",
                    r#"{"priority":"high"}"#,
                ),
                candidate(
                    "attempt.exhaust_retry",
                    "content.validation.exhaust_retry",
                    r#"{"priority":"high"}"#,
                ),
            ],
        )
        .expect("repair exhaustion runs");

    let ValidationRepairOutcome::Failed {
        failure,
        validation_records,
        repair_records,
        exhaustion_record,
    } = outcome
    else {
        panic!("expected terminal validation failure");
    };

    assert!(failure.retry_exhausted);
    assert_eq!(
        failure.as_agent_error().retry(),
        RetryClassification::NotRetryable
    );
    assert_eq!(failure.validation_attempts.len(), 2);
    assert_eq!(failure.repair_attempts.len(), 1);
    assert_eq!(validation_records.len(), 3);
    assert_eq!(repair_records.len(), 1);
    assert_eq!(
        normalized(&exhaustion_record),
        read_fixture("tests/fixtures/validation/repair-exhausted-record.json")
            .expect("repair exhausted fixture")
    );
}

#[test]
fn hostile_schema_is_rejected_without_repair_attempt() {
    let contract = OutputContract::inline_json_schema(
        OutputSchemaId::new("schema.hostile.validation"),
        SchemaVersion::new(1, 0, 0),
        json!({
            "$ref": "https://example.invalid/remote-schema.json"
        }),
    );
    let service = LocalValidationRepairService::default_json_schema_subset();
    let outcome = service
        .validate_candidates(
            &contract,
            [candidate(
                "attempt.hostile",
                "content.validation.hostile",
                r#"{"title":"ignored"}"#,
            )],
        )
        .expect("hostile schema checked");

    let ValidationRepairOutcome::Failed {
        failure,
        validation_records,
        repair_records,
        ..
    } = outcome
    else {
        panic!("expected hostile schema rejection");
    };

    assert!(!failure.retry_exhausted);
    assert!(repair_records.is_empty());
    assert_eq!(
        validation_records[0].record_kind,
        ValidationRecordKind::SchemaRejected
    );
}

#[test]
fn validation_and_repair_records_lower_into_shared_structured_output_journal_payloads() {
    let contract = todo_contract();
    let service = LocalValidationRepairService::default_json_schema_subset();
    let outcome = service
        .validate_candidates(
            &contract,
            [
                candidate(
                    "attempt.journal_initial",
                    "content.validation.journal_initial",
                    r#"{"priority":"high"}"#,
                ),
                candidate(
                    "attempt.journal_fixed",
                    "content.validation.journal_fixed",
                    r#"{"title":"ship validation","priority":"high"}"#,
                ),
            ],
        )
        .expect("repair sequence runs");

    let ValidationRepairOutcome::Validated {
        validation_records,
        repair_records,
        ..
    } = outcome
    else {
        panic!("expected repaired candidate to validate");
    };

    let validation_payload = JournalRecordPayload::StructuredOutput(
        StructuredOutputRecord::Validation(validation_records[0].clone()),
    );
    let repair_payload = JournalRecordPayload::StructuredOutput(StructuredOutputRecord::Repair(
        repair_records[0].clone(),
    ));

    assert_eq!(normalized(&validation_payload)["type"], "structured_output");
    assert_eq!(normalized(&validation_payload)["record_type"], "validation");
    assert_eq!(normalized(&repair_payload)["record_type"], "repair");
}

#[test]
fn repair_prompt_and_records_do_not_include_raw_private_candidate() {
    let mut contract = todo_contract();
    contract.repair.include_candidate_content = CandidateContentRepairPolicy::RedactedCandidate;
    let service = LocalValidationRepairService::default_json_schema_subset();
    let secret = "raw_private_candidate_should_not_escape";
    let outcome = service
        .validate_candidates(
            &contract,
            [
                candidate("attempt.secret", "content.validation.secret", secret)
                    .with_privacy(PrivacyClass::Secret),
            ],
        )
        .expect("redaction flow runs");

    let ValidationRepairOutcome::RepairRequested {
        latest_report,
        prompt,
        validation_records,
        repair_records,
    } = outcome
    else {
        panic!("expected repair request for invalid private candidate");
    };

    assert_no_raw_content(secret, &latest_report);
    assert_no_raw_content(secret, &prompt);
    assert_no_raw_content(secret, &validation_records);
    assert_no_raw_content(secret, &repair_records);
}

fn todo_contract() -> OutputContract {
    OutputContract::inline_json_schema(
        OutputSchemaId::new("schema.todo_validation"),
        SchemaVersion::new(1, 0, 0),
        json!({
            "type": "object",
            "required": ["title"],
            "properties": {
                "title": {"type": "string"},
                "priority": {"type": "string", "enum": ["low", "high"]}
            },
            "additionalProperties": false
        }),
    )
}

fn candidate(
    source_attempt_id: &str,
    candidate_content_ref: &str,
    text: impl Into<String>,
) -> OutputCandidate {
    OutputCandidate::new(
        AttemptId::new(source_attempt_id),
        ContentRef::new(candidate_content_ref),
        text,
    )
}

fn normalized(value: &impl Serialize) -> Value {
    normalize_json_value(serde_json::to_value(value).expect("serializes to JSON"))
}

fn assert_no_raw_content(secret: &str, value: &impl Serialize) {
    let serialized = serde_json::to_string(value).expect("serializes to JSON");
    assert!(
        !serialized.contains(secret),
        "raw candidate content leaked into record: {serialized}"
    );
}
