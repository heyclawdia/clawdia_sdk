use agent_sdk_core::{
    AdapterRef, AgentErrorKind, AttemptId, ContentHash, ContentId, ContentKind, ContentRef,
    ContentScope, ContentVersion, DecodedTypedOutput, DestinationKind, DestinationRef, EntityKind,
    EntityRef, LineageId, LineageRef, OutputLineage, OutputSchemaId, PolicyKind, PolicyRef,
    PrivacyClass, RetentionClass, SchemaVersion, SourceKind, SourceRef, StructuredOutputRecord,
    StructuredOutputResult, TrustClass, TypedOutputDeserializer, TypedOutputError,
    TypedResultPublicationRecord, ValidatedOutput, ValidatedOutputId, ValidatedOutputParams,
    ValidatedOutputPublicationStep, ValidationAttemptId, ValidationReportRecord,
    journal::JournalRecordPayload,
    testing::{normalize_json_value, read_fixture},
    validate_typed_result_publication_order,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct TodoExtraction {
    title: String,
    priority: String,
}

#[test]
fn validated_output_record_matches_golden_fixture() {
    let output = validated_output();
    output.validate_shape().expect("validated output shape");

    let actual = normalized(&output);
    let expected = read_fixture("tests/fixtures/validated_output/validated-output-record.json")
        .expect("validated output fixture");

    assert_eq!(actual, expected);
    assert_eq!(actual["record_schema_version"], 1);
    assert_eq!(actual["privacy"], "content_refs_only");
    assert!(actual.get("raw_content").is_none());
}

#[test]
fn typed_result_publication_record_is_ref_only_and_matches_fixture() {
    let output = validated_output();
    let publication = TypedResultPublicationRecord::published(&output).expect("publication");

    let actual = normalized(&publication);
    let expected =
        read_fixture("tests/fixtures/validated_output/typed-result-publication-record.json")
            .expect("typed result publication fixture");

    assert_eq!(actual, expected);
    assert_eq!(actual["status"], "published");
    assert!(actual.get("output").is_none());
    assert!(
        !serde_json::to_string(&actual)
            .expect("publication JSON")
            .contains("raw_content")
    );
}

#[test]
fn typed_result_publication_requires_validation_record_first() {
    let report = validation_report();
    let output = validated_output_from_report(&report);
    let publication = TypedResultPublicationRecord::published(&output).expect("publication");

    let publication_without_evidence = validate_typed_result_publication_order(&[
        ValidatedOutputPublicationStep::TypedResultPublication(publication.clone()),
    ])
    .expect_err("publication before validation must fail");
    assert!(matches!(
        publication_without_evidence,
        TypedOutputError::PublicationBeforeValidation { .. }
    ));

    let output_before_report = validate_typed_result_publication_order(&[
        ValidatedOutputPublicationStep::ValidatedOutput(output.clone()),
        ValidatedOutputPublicationStep::ValidationReport(report.clone()),
        ValidatedOutputPublicationStep::TypedResultPublication(publication.clone()),
    ])
    .expect_err("validated output before report must fail");
    assert!(matches!(
        output_before_report,
        TypedOutputError::PublicationBeforeValidation { .. }
    ));

    validate_typed_result_publication_order(&[
        ValidatedOutputPublicationStep::ValidationReport(report),
        ValidatedOutputPublicationStep::ValidatedOutput(output),
        ValidatedOutputPublicationStep::TypedResultPublication(publication),
    ])
    .expect("validation report precedes typed result publication");
}

#[test]
fn typed_result_publication_requires_non_empty_matching_validation_refs() {
    let report = validation_report();
    let output = validated_output_from_report(&report);
    let mut publication = TypedResultPublicationRecord::published(&output).expect("publication");
    publication.validation_report_refs.clear();

    let direct_error = publication
        .validate_against_output(&output)
        .expect_err("empty report refs must fail direct publication validation");
    assert!(matches!(
        direct_error,
        TypedOutputError::MissingValidationReport { .. }
    ));

    let order_error = validate_typed_result_publication_order(&[
        ValidatedOutputPublicationStep::ValidationReport(report),
        ValidatedOutputPublicationStep::ValidatedOutput(output),
        ValidatedOutputPublicationStep::TypedResultPublication(publication),
    ])
    .expect_err("empty report refs must fail publication ordering");
    assert!(matches!(
        order_error,
        TypedOutputError::MissingValidationReport { .. }
    ));
}

#[test]
fn typed_result_publication_order_rejects_report_ref_metadata_mismatch() {
    let report = validation_report();
    let output = validated_output_from_report(&report);
    let mut publication = TypedResultPublicationRecord::published(&output).expect("publication");
    publication.validation_report_refs[0].validation_attempt_id =
        ValidationAttemptId::new("validation.attempt.spoofed");
    publication.validation_report_refs[0].redacted_summary = "spoofed report metadata".to_string();

    let order_error = validate_typed_result_publication_order(&[
        ValidatedOutputPublicationStep::ValidationReport(report),
        ValidatedOutputPublicationStep::ValidatedOutput(output),
        ValidatedOutputPublicationStep::TypedResultPublication(publication),
    ])
    .expect_err("same report ref with different metadata must fail");

    assert!(matches!(
        order_error,
        TypedOutputError::PublicationEvidenceMismatch { .. }
    ));
}

#[test]
fn validated_output_order_rejects_report_ref_metadata_mismatch() {
    let report = validation_report();
    let mut output = validated_output_from_report(&report);
    output.validation_report_refs[0].validation_attempt_id =
        ValidationAttemptId::new("validation.attempt.spoofed_output");
    output.validation_report_refs[0].redacted_summary =
        "spoofed validated output metadata".to_string();
    let publication = TypedResultPublicationRecord::published(&output).expect("publication");

    let order_error = validate_typed_result_publication_order(&[
        ValidatedOutputPublicationStep::ValidationReport(report),
        ValidatedOutputPublicationStep::ValidatedOutput(output),
        ValidatedOutputPublicationStep::TypedResultPublication(publication),
    ])
    .expect_err("validated output must match the prior validation report metadata");

    assert!(matches!(
        order_error,
        TypedOutputError::PublicationEvidenceMismatch { .. }
    ));
}

#[test]
fn failed_validation_report_cannot_create_validated_output() {
    let error = ValidatedOutput::from_validation_report(params(), &failed_validation_report())
        .expect_err("failed report must not create output");

    assert!(matches!(
        error,
        TypedOutputError::ValidationReportFailed { .. }
    ));
    let agent_error: agent_sdk_core::AgentError = error.into();
    assert_eq!(agent_error.kind(), AgentErrorKind::StructuredOutputFailure);
}

#[test]
fn structured_output_result_preserves_validated_evidence() {
    let output = validated_output();
    let publication = TypedResultPublicationRecord::published(&output).expect("publication");
    let typed_value = TodoExtraction {
        title: "Pay invoice".to_string(),
        priority: "high".to_string(),
    };
    let deserializer =
        FakeTodoDeserializer::new(output.canonical_value_ref.clone(), typed_value.clone());

    let result = StructuredOutputResult::from_publication(&output, &publication, &deserializer)
        .expect("typed result");

    assert_eq!(result.output, typed_value);
    assert_eq!(result.validated_output_id, output.output_id);
    assert_eq!(
        result.output_ref.content_id,
        output.canonical_value_ref.content_id
    );
    assert_eq!(result.validation_attempts, output.validation_attempts);
    assert_eq!(result.source_attempt_ids, output.source_attempt_ids);
}

#[test]
fn typed_result_rejects_decoder_content_ref_mismatch() {
    let output = validated_output();
    let publication = TypedResultPublicationRecord::published(&output).expect("publication");
    let deserializer = FakeTodoDeserializer::new(
        content_ref(
            "content.output.canonical.other",
            ContentKind::OutputPayload,
            "different canonical JSON",
        ),
        TodoExtraction {
            title: "Pay invoice".to_string(),
            priority: "high".to_string(),
        },
    );

    let error = StructuredOutputResult::from_publication(&output, &publication, &deserializer)
        .expect_err("decoder must return the validated canonical content ref");

    assert!(matches!(
        error,
        TypedOutputError::CanonicalValueRefMismatch { .. }
    ));
}

#[test]
fn policy_denial_blocks_typed_result_without_best_effort_value() {
    let output = validated_output();
    let denied = TypedResultPublicationRecord::policy_denied(
        &output,
        "validated output publication denied by output policy",
    )
    .expect("policy denial record");

    let error = StructuredOutputResult::from_publication(
        &output,
        &denied,
        &FakeTodoDeserializer::new(
            output.canonical_value_ref.clone(),
            TodoExtraction {
                title: "Pay invoice".to_string(),
                priority: "high".to_string(),
            },
        ),
    )
    .expect_err("policy denial blocks typed result");

    assert!(matches!(
        error,
        TypedOutputError::PublicationPolicyDenied { .. }
    ));
}

#[test]
fn typed_result_phase_does_not_depend_on_output_delivery() {
    let source = include_str!("../../src/records/validated_output.rs");
    for forbidden in ["OutputSink", "OutputDelivery", "output_delivery"] {
        assert!(
            !source.contains(forbidden),
            "{forbidden} belongs to output-delivery integration, not Phase 07B"
        );
    }
}

#[test]
fn validated_output_records_lower_into_shared_structured_output_journal_payloads() {
    let report = validation_report();
    let output = validated_output_from_report(&report);
    let publication = TypedResultPublicationRecord::published(&output).expect("publication");

    let report_payload = JournalRecordPayload::StructuredOutput(
        StructuredOutputRecord::ValidationReport(report.clone()),
    );
    let output_payload =
        JournalRecordPayload::StructuredOutput(StructuredOutputRecord::ValidatedOutput(output));
    let publication_payload = JournalRecordPayload::StructuredOutput(
        StructuredOutputRecord::TypedResultPublication(publication),
    );

    assert_eq!(normalized(&report_payload)["type"], "structured_output");
    assert_eq!(
        normalized(&report_payload)["record_type"],
        "validation_report"
    );
    assert_eq!(
        normalized(&output_payload)["record_type"],
        "validated_output"
    );
    assert_eq!(
        normalized(&publication_payload)["record_type"],
        "typed_result_publication"
    );
}

fn validated_output() -> ValidatedOutput {
    let report = validation_report();
    validated_output_from_report(&report)
}

fn validated_output_from_report(report: &ValidationReportRecord) -> ValidatedOutput {
    ValidatedOutput::from_validation_report(params(), report).expect("validated output")
}

struct FakeTodoDeserializer {
    decoded_ref: ContentRef,
    value: TodoExtraction,
}

impl FakeTodoDeserializer {
    fn new(decoded_ref: ContentRef, value: TodoExtraction) -> Self {
        Self { decoded_ref, value }
    }
}

impl TypedOutputDeserializer<TodoExtraction> for FakeTodoDeserializer {
    fn deserialize(
        &self,
        canonical_value_ref: &ContentRef,
    ) -> Result<DecodedTypedOutput<TodoExtraction>, TypedOutputError> {
        assert_eq!(
            canonical_value_ref.content_id.as_str(),
            "content.output.canonical.1",
            "typed extraction requests the validated canonical output ref"
        );
        Ok(DecodedTypedOutput::new(
            self.decoded_ref.clone(),
            self.value.clone(),
        ))
    }
}

fn params() -> ValidatedOutputParams {
    ValidatedOutputParams {
        output_id: ValidatedOutputId::new("validated.output.todo.1"),
        schema_id: schema_id(),
        schema_version: SchemaVersion::new(1, 0, 0),
        schema_fingerprint: ContentHash::new(
            "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        ),
        canonical_value_ref: content_ref(
            "content.output.canonical.1",
            ContentKind::OutputPayload,
            "validated todo extraction canonical JSON",
        ),
        repair_attempts: Vec::new(),
        source_attempt_ids: Vec::new(),
        content_refs: Vec::new(),
        lineage: output_lineage(),
        policy_refs: vec![output_policy()],
        privacy: PrivacyClass::ContentRefsOnly,
        redacted_summary: "todo extraction validated with refs only".to_string(),
    }
}

fn validation_report() -> ValidationReportRecord {
    let mut report = ValidationReportRecord::passed(
        ValidationAttemptId::new("validation.attempt.todo.1"),
        schema_id(),
        SchemaVersion::new(1, 0, 0),
        AttemptId::new("attempt.model.todo.1"),
        content_ref(
            "content.output.candidate.1",
            ContentKind::OutputPayload,
            "candidate JSON stored by content ref",
        ),
        content_ref(
            "content.validation.report.1",
            ContentKind::Document,
            "validation report with zero redacted errors",
        ),
        "local validation passed with zero redacted errors",
    );
    report.policy_refs = vec![output_policy()];
    report
}

fn failed_validation_report() -> ValidationReportRecord {
    ValidationReportRecord::failed(
        ValidationAttemptId::new("validation.attempt.todo.failed"),
        schema_id(),
        SchemaVersion::new(1, 0, 0),
        AttemptId::new("attempt.model.todo.failed"),
        content_ref(
            "content.output.candidate.failed",
            ContentKind::OutputPayload,
            "candidate JSON missing required field",
        ),
        content_ref(
            "content.validation.report.failed",
            ContentKind::Document,
            "validation report with redacted errors",
        ),
        "required field title missing",
    )
}

fn content_ref(id: &str, kind: ContentKind, summary: &str) -> ContentRef {
    let mut content_ref = ContentRef::new(
        ContentId::new(id),
        ContentVersion::new("v1"),
        kind,
        ContentScope::Run,
        EntityRef::new(EntityKind::Attempt, AttemptId::new("attempt.model.todo.1")),
        source_ref(),
        AdapterRef::new("resolver.content.fake"),
        summary,
    );
    content_ref.mime = Some("application/json".to_string());
    content_ref.size_bytes = Some(128);
    content_ref.content_hash =
        Some("sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".to_string());
    content_ref.privacy_class = PrivacyClass::ContentRefsOnly;
    content_ref.retention_class = RetentionClass::RunScoped;
    content_ref.trust_class = TrustClass::SdkGenerated;
    content_ref
}

fn output_lineage() -> OutputLineage {
    OutputLineage {
        lineage_ref: LineageRef {
            lineage_id: LineageId::new("lineage.validated.output.todo.1"),
            source: source_ref(),
            destination: Some(DestinationRef::with_kind(
                DestinationKind::Host,
                "destination.typed.result",
            )),
            policy_refs: vec![output_policy()],
        },
        produced_by: EntityRef::new(EntityKind::Attempt, AttemptId::new("attempt.model.todo.1")),
        derived_from: vec![EntityRef::new(
            EntityKind::Content,
            "content.output.candidate.1",
        )],
    }
}

fn schema_id() -> OutputSchemaId {
    OutputSchemaId::new("schema.todo_extraction")
}

fn source_ref() -> SourceRef {
    SourceRef::with_kind(SourceKind::Sdk, "source.structured_output.validator")
}

fn output_policy() -> PolicyRef {
    PolicyRef::with_kind(PolicyKind::Privacy, "policy.output.refs_only")
}

fn normalized(value: &impl Serialize) -> Value {
    normalize_json_value(serde_json::to_value(value).expect("record JSON"))
}
