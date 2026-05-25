//! Loop driver for the first text-run proof. Use it to connect the runtime package,
//! context projection, provider port, event bus, and journal in one canonical P0
//! execution path. Driving the loop may call provider adapters, append journals, and
//! publish events.
//!
use sha2::{Digest, Sha256};

use serde_json::Value;

use crate::{
    content::{ContentKind, ContentRef as StoredContentRef, ContentScope, ContentVersion},
    context::{
        AgentMessage, ContextBudgetSummary, ContextContribution, ContextContributionId,
        ContextContributionKind, ContextItem, ContextProjection, ProjectionRole,
        sdk_context_policy_ref,
    },
    domain::{
        AdapterRef, AgentError, AgentId, ContentId, ContentRef as ContentRefId, ContextItemId,
        ContextProjectionId, DestinationKind, DestinationRef, EffectId, EntityKind, EntityRef,
        EventId, IdempotencyKey, LineageId, LineageRef, MessageId, PolicyKind, PolicyRef,
        PrivacyClass, RetentionClass, RunId, SourceKind, SourceRef, SpanId, TraceId, TrustClass,
        TurnId, ValidatedOutputId, ValidationAttemptId,
    },
    effect::{EffectIntent, EffectKind, EffectResult, EffectTerminalStatus},
    event::{
        AgentEvent, ContentCaptureMode, EVENT_SCHEMA_VERSION, EventCorrelation,
        EventDeliverySemantics, EventEnvelope, EventFamily, EventFrame, EventKind,
        EventStreamScope,
    },
    journal::{
        ContextProjectionRecord, EventIndexProjection, JOURNAL_SCHEMA_VERSION, JournalRecord,
        JournalRecordBase, JournalRecordKind, JournalRecordPayload, MessageRecord,
        ModelAttemptRecord, RunLifecycleRecord, StructuredOutputRecord, TerminalResultMarker,
    },
    output::OutputContract,
    provider::{
        ProviderAdapter, ProviderMessage, ProviderMessageRole, ProviderProjectionPolicy,
        ProviderRequest, ProviderResponse, ProviderStopReason,
    },
    repair::{
        RepairAccounting, RepairDecision, RepairPolicyController,
        repair_exhaustion_record_from_failure,
    },
    run::{RunRequest, RunResult, RunStatus, StructuredOutputArtifacts},
    run_handle::RunHandle,
    runtime::AgentRuntime,
    structured_output::StructuredOutputLifecycleRecord,
    validated_output::{
        OutputLineage, TypedResultPublicationRecord, ValidatedOutput, ValidatedOutputParams,
        ValidatedOutputPublicationStep, ValidationReportRecord,
        validate_typed_result_publication_order,
    },
    validation::{JsonSchemaSubsetValidator, OutputCandidate, StructuredOutputValidator},
};

/// Drives the canonical P0 text run loop.
/// This resolves runtime ports, appends journal records, publishes run/model events, calls the
/// configured provider adapter, and seals terminal run-control state.
pub fn run_p0_text(
    runtime: &AgentRuntime,
    request: RunRequest,
    handle: RunHandle,
) -> Result<RunResult, AgentError> {
    let snapshot = runtime.run_snapshot(&request.run_id)?;
    let journal = runtime.journal_port(&request.run_id)?;
    let events = runtime.event_bus_port(&request.run_id)?;
    let provider = runtime.provider_for_route(&snapshot.provider_route_id, &request.run_id)?;
    let ids = P0Ids::new(&request.run_id);
    let mut event_ids = EventIdSequence::default();
    let fingerprint = snapshot.runtime_package_fingerprint.as_str().to_string();
    let source = SourceRef::with_kind(SourceKind::Sdk, "source.sdk.p0_text_run");

    let run_started = journal_record(
        &request.run_id,
        &request.agent_id,
        None,
        None,
        runtime.next_journal_seq(),
        ids.record("run.started"),
        JournalRecordKind::Run,
        "run",
        "started",
        source.clone(),
        fingerprint.clone(),
        JournalRecordPayload::RunLifecycle(RunLifecycleRecord {
            status: "started".to_string(),
            reason: "p0_text_run".to_string(),
        }),
    );
    let run_started_cursor = journal.append(run_started)?;
    events.publish(event_frame(
        &request.run_id,
        &request.agent_id,
        None,
        None,
        event_ids.next(),
        EventFamily::Run,
        EventKind::RunStarted,
        "run started",
        Some(run_started_cursor),
        fingerprint.clone(),
        &ids,
    ))?;

    let projection = build_text_projection(&request, &ids, &fingerprint)?;
    let context_record = journal_record(
        &request.run_id,
        &request.agent_id,
        None,
        None,
        runtime.next_journal_seq(),
        ids.record("context.projected"),
        JournalRecordKind::Context,
        "context",
        "projected",
        source.clone(),
        fingerprint.clone(),
        JournalRecordPayload::ContextProjection(ContextProjectionRecord {
            projection_id: projection.projection_id.clone(),
            selected_item_count: projection.items.len() as u32,
            provider_destination: projection.provider_destination.clone(),
        }),
    );
    let context_cursor = journal.append(context_record)?;
    events.publish(event_frame(
        &request.run_id,
        &request.agent_id,
        Some(ids.turn_id.clone()),
        None,
        event_ids.next(),
        EventFamily::Turn,
        EventKind::ContextAssembled,
        "context assembled",
        Some(context_cursor),
        fingerprint.clone(),
        &ids,
    ))?;

    if let Some(output_contract) = request.output_contract.as_ref() {
        append_structured_output_requested(
            runtime,
            &request,
            &ids,
            &mut event_ids,
            &fingerprint,
            &source,
            &ids.attempt_id,
            output_contract,
        )?;
    }

    let mut provider_request = provider.project_request(
        &projection,
        &ProviderProjectionPolicy::redacted("policy.provider.p0_redacted"),
    )?;
    if let Some(output_contract) = request.output_contract.as_ref() {
        provider_request = provider_request.with_structured_output_hint(output_contract);
    }
    let provider_effect_intent = journal_record(
        &request.run_id,
        &request.agent_id,
        Some(ids.turn_id.clone()),
        Some(ids.attempt_id.clone()),
        runtime.next_journal_seq(),
        ids.record("provider.effect.intent"),
        JournalRecordKind::EffectIntent,
        "effect",
        "provider_request_intent",
        source.clone(),
        fingerprint.clone(),
        JournalRecordPayload::EffectIntent(EffectIntent {
            effect_id: ids.provider_effect_id.clone(),
            kind: EffectKind::ProviderRequest,
            subject_ref: EntityRef::run(request.run_id.clone()),
            source: source.clone(),
            destination: Some(DestinationRef::with_kind(
                DestinationKind::Provider,
                "destination.provider.p0_text",
            )),
            policy_refs: vec![PolicyRef::with_kind(
                PolicyKind::RuntimePackage,
                "policy.p0.provider_request",
            )],
            idempotency_key: Some(IdempotencyKey::new(format!(
                "idempotency.p0.{}.provider_request",
                ids.fragment
            ))),
            dedupe_key: None,
            content_refs: Vec::new(),
            redacted_summary: "provider request intent".to_string(),
        }),
    );
    let provider_effect_cursor = journal.append(provider_effect_intent)?;
    let model_intent = journal_record(
        &request.run_id,
        &request.agent_id,
        Some(ids.turn_id.clone()),
        Some(ids.attempt_id.clone()),
        runtime.next_journal_seq(),
        ids.record("model.intent"),
        JournalRecordKind::ModelAttempt,
        "model",
        "provider_request_projected",
        source.clone(),
        fingerprint.clone(),
        JournalRecordPayload::ModelAttempt(ModelAttemptRecord {
            provider_route_id: snapshot.provider_route_id.clone(),
            provider_model_id: snapshot.provider_model_id.clone(),
            request_message_count: provider_request.messages.len() as u32,
            stop_reason: None,
            usage: None,
        }),
    );
    let model_intent_cursor = journal.append(model_intent)?;
    events.publish(event_frame(
        &request.run_id,
        &request.agent_id,
        Some(ids.turn_id.clone()),
        Some(ids.attempt_id.clone()),
        event_ids.next(),
        EventFamily::Turn,
        EventKind::ProviderRequestProjected,
        "provider request projected",
        Some(provider_effect_cursor),
        fingerprint.clone(),
        &ids,
    ))?;
    events.publish(event_frame(
        &request.run_id,
        &request.agent_id,
        Some(ids.turn_id.clone()),
        Some(ids.attempt_id.clone()),
        event_ids.next(),
        EventFamily::Model,
        EventKind::ModelAttemptStarted,
        "model attempt started",
        Some(model_intent_cursor),
        fingerprint.clone(),
        &ids,
    ))?;

    let response = provider.complete(&provider_request)?;
    let response = append_model_attempt_completion(
        runtime,
        &request,
        &ids,
        &mut event_ids,
        &fingerprint,
        &source,
        ids.attempt_id.clone(),
        &snapshot.provider_route_id,
        &snapshot.provider_model_id,
        provider_request.messages.len() as u32,
        ids.provider_effect_id.clone(),
        response,
    )?;

    if let Some(output_contract) = request.output_contract.as_ref() {
        match drive_p1_structured_output(
            runtime,
            &request,
            &ids,
            &mut event_ids,
            &fingerprint,
            &source,
            provider.as_ref(),
            &provider_request,
            &snapshot.provider_route_id,
            &snapshot.provider_model_id,
            output_contract,
            response,
        ) {
            Ok(success) => append_message_and_terminal(
                runtime,
                &handle,
                &request,
                &ids,
                &mut event_ids,
                &fingerprint,
                &source,
                RunStatus::Completed,
                success.final_output,
                Some(success.artifacts),
            ),
            Err(error) => {
                let summary = error.context().message;
                append_message_and_terminal(
                    runtime,
                    &handle,
                    &request,
                    &ids,
                    &mut event_ids,
                    &fingerprint,
                    &source,
                    RunStatus::Failed,
                    summary,
                    None,
                )?;
                Err(error)
            }
        }
    } else {
        append_message_and_terminal(
            runtime,
            &handle,
            &request,
            &ids,
            &mut event_ids,
            &fingerprint,
            &source,
            RunStatus::Completed,
            response.output_text,
            None,
        )
    }
}

fn append_model_attempt_completion(
    runtime: &AgentRuntime,
    request: &RunRequest,
    ids: &P0Ids,
    event_ids: &mut EventIdSequence,
    fingerprint: &str,
    source: &SourceRef,
    attempt_id: crate::ids::AttemptId,
    provider_route_id: &str,
    provider_model_id: &str,
    request_message_count: u32,
    provider_effect_id: EffectId,
    response: ProviderResponse,
) -> Result<ProviderResponse, AgentError> {
    let journal = runtime.journal_port(&request.run_id)?;
    let events = runtime.event_bus_port(&request.run_id)?;
    let terminal_status = terminal_status(&response.stop_reason);
    let effect_status = match &terminal_status {
        RunStatus::Completed => EffectTerminalStatus::Completed,
        RunStatus::Cancelled => EffectTerminalStatus::Cancelled,
        _ => EffectTerminalStatus::Failed,
    };

    let provider_result = journal_record(
        &request.run_id,
        &request.agent_id,
        Some(ids.turn_id.clone()),
        Some(attempt_id.clone()),
        runtime.next_journal_seq(),
        ids.record(&format!("provider.effect.result.{}", attempt_id.as_str())),
        JournalRecordKind::EffectResult,
        "effect",
        "provider_request_result",
        source.clone(),
        fingerprint.to_string(),
        JournalRecordPayload::EffectResult(EffectResult {
            effect_id: provider_effect_id,
            terminal_status: effect_status,
            external_operation_id: None,
            reconciliation_ref: None,
            error_ref: None,
            content_refs: Vec::new(),
            redacted_summary: "provider request completed".to_string(),
        }),
    );
    journal.append(provider_result)?;

    let model_complete = journal_record(
        &request.run_id,
        &request.agent_id,
        Some(ids.turn_id.clone()),
        Some(attempt_id.clone()),
        runtime.next_journal_seq(),
        ids.record(&format!("model.completed.{}", attempt_id.as_str())),
        JournalRecordKind::ModelAttempt,
        "model",
        "completed",
        source.clone(),
        fingerprint.to_string(),
        JournalRecordPayload::ModelAttempt(ModelAttemptRecord {
            provider_route_id: provider_route_id.to_string(),
            provider_model_id: provider_model_id.to_string(),
            request_message_count,
            stop_reason: Some(response.stop_reason.clone()),
            usage: response.usage.clone(),
        }),
    );
    let model_complete_cursor = journal.append(model_complete)?;
    events.publish(event_frame(
        &request.run_id,
        &request.agent_id,
        Some(ids.turn_id.clone()),
        Some(attempt_id),
        event_ids.next(),
        EventFamily::Model,
        EventKind::ModelMessageCompleted,
        "model message completed",
        Some(model_complete_cursor),
        fingerprint.to_string(),
        ids,
    ))?;

    Ok(response)
}

fn append_message_and_terminal(
    runtime: &AgentRuntime,
    handle: &RunHandle,
    request: &RunRequest,
    ids: &P0Ids,
    event_ids: &mut EventIdSequence,
    fingerprint: &str,
    source: &SourceRef,
    terminal_status: RunStatus,
    output: String,
    structured_output: Option<StructuredOutputArtifacts>,
) -> Result<RunResult, AgentError> {
    let journal = runtime.journal_port(&request.run_id)?;
    let events = runtime.event_bus_port(&request.run_id)?;

    let message_record = journal_record(
        &request.run_id,
        &request.agent_id,
        Some(ids.turn_id.clone()),
        Some(ids.attempt_id.clone()),
        runtime.next_journal_seq(),
        ids.record("message.completed"),
        JournalRecordKind::Message,
        "message",
        "completed",
        source.clone(),
        fingerprint.to_string(),
        JournalRecordPayload::Message(MessageRecord {
            message_id: ids.output_message_id.clone(),
            role: "assistant".to_string(),
            redacted_summary: if structured_output.is_some() {
                "assistant structured output response".to_string()
            } else {
                "assistant text response".to_string()
            },
        }),
    );
    journal.append(message_record)?;

    let terminal_record = journal_record(
        &request.run_id,
        &request.agent_id,
        Some(ids.turn_id.clone()),
        Some(ids.attempt_id.clone()),
        runtime.next_journal_seq(),
        ids.record("run.terminal"),
        JournalRecordKind::Run,
        "run",
        terminal_status.as_terminal_str().unwrap_or("failed"),
        source.clone(),
        fingerprint.to_string(),
        JournalRecordPayload::TerminalResult(TerminalResultMarker {
            effect_id: ids.terminal_effect_id.clone(),
            result_record_id: ids.record("run.terminal"),
            terminal_status: terminal_status
                .as_terminal_str()
                .unwrap_or("failed")
                .to_string(),
        }),
    );
    let terminal_cursor = journal.append(terminal_record.clone())?;
    let sealed = runtime
        .seal_terminal_result_from_journal(&terminal_record, output.clone())?
        .with_structured_output_if_present(structured_output);
    events.publish(event_frame(
        &request.run_id,
        &request.agent_id,
        Some(ids.turn_id.clone()),
        Some(ids.attempt_id.clone()),
        event_ids.next(),
        EventFamily::Run,
        terminal_event_kind(&terminal_status),
        "run terminal",
        Some(terminal_cursor),
        fingerprint.to_string(),
        ids,
    ))?;

    let _ = handle.wait()?;
    Ok(sealed)
}

struct P1StructuredOutputSuccess {
    final_output: String,
    artifacts: StructuredOutputArtifacts,
}

fn append_structured_output_requested(
    runtime: &AgentRuntime,
    request: &RunRequest,
    ids: &P0Ids,
    event_ids: &mut EventIdSequence,
    fingerprint: &str,
    source: &SourceRef,
    attempt_id: &crate::ids::AttemptId,
    contract: &OutputContract,
) -> Result<(), AgentError> {
    let journal = runtime.journal_port(&request.run_id)?;
    let events = runtime.event_bus_port(&request.run_id)?;
    let requested = StructuredOutputLifecycleRecord::requested(
        contract.schema_id.clone(),
        contract.schema_version,
        contract.schema_fingerprint(),
    );
    let requested_cursor = journal.append(structured_output_journal_record(
        runtime,
        request,
        ids,
        attempt_id,
        fingerprint,
        source,
        "structured_output.requested",
        "requested",
        StructuredOutputRecord::Lifecycle(requested),
    ))?;
    events.publish(event_frame(
        &request.run_id,
        &request.agent_id,
        Some(ids.turn_id.clone()),
        Some(attempt_id.clone()),
        event_ids.next(),
        EventFamily::StructuredOutput,
        EventKind::StructuredOutputRequested,
        "structured output requested",
        Some(requested_cursor),
        fingerprint.to_string(),
        ids,
    ))?;
    Ok(())
}

fn drive_p1_structured_output(
    runtime: &AgentRuntime,
    request: &RunRequest,
    ids: &P0Ids,
    event_ids: &mut EventIdSequence,
    fingerprint: &str,
    source: &SourceRef,
    provider: &dyn ProviderAdapter,
    base_provider_request: &ProviderRequest,
    provider_route_id: &str,
    provider_model_id: &str,
    contract: &OutputContract,
    first_response: ProviderResponse,
) -> Result<P1StructuredOutputSuccess, AgentError> {
    let journal = runtime.journal_port(&request.run_id)?;
    let events = runtime.event_bus_port(&request.run_id)?;
    let validator = JsonSchemaSubsetValidator::default();
    let repair_controller = RepairPolicyController;
    let mut repair_accounting = RepairAccounting::default();
    let mut reports = Vec::new();
    let mut response = first_response;
    let mut attempt_id = ids.attempt_id.clone();
    let mut attempt_index = 0_u8;

    loop {
        attempt_index += 1;
        let candidate_content_ref = candidate_content_ref(ids, attempt_index);
        let candidate = OutputCandidate::new(
            attempt_id.clone(),
            candidate_content_ref.clone(),
            response.output_text.clone(),
        );
        let validation_attempt_id =
            ValidationAttemptId::new(format!("validation.{}.{}", ids.fragment, attempt_index));

        let started = StructuredOutputLifecycleRecord::validation_started(
            contract.schema_id.clone(),
            contract.schema_version,
            contract.schema_fingerprint(),
            attempt_id.clone(),
            candidate_content_ref.clone(),
        );
        let started_cursor = journal.append(structured_output_journal_record(
            runtime,
            request,
            ids,
            &attempt_id,
            fingerprint,
            source,
            &format!("structured_output.validation_started.{attempt_index}"),
            "validation_started",
            StructuredOutputRecord::Lifecycle(started),
        ))?;
        events.publish(event_frame(
            &request.run_id,
            &request.agent_id,
            Some(ids.turn_id.clone()),
            Some(attempt_id.clone()),
            event_ids.next(),
            EventFamily::StructuredOutput,
            EventKind::StructuredOutputValidationStarted,
            "structured output validation started",
            Some(started_cursor),
            fingerprint.to_string(),
            ids,
        ))?;

        match validator.validate_candidate(contract, validation_attempt_id, &candidate) {
            Ok(success) => {
                journal.append(structured_output_journal_record(
                    runtime,
                    request,
                    ids,
                    &attempt_id,
                    fingerprint,
                    source,
                    &format!("structured_output.validation_succeeded.{attempt_index}"),
                    "validation_succeeded",
                    StructuredOutputRecord::Validation(success.record.clone()),
                ))?;

                let validation_report = ValidationReportRecord::passed(
                    success.validation_attempt_id.clone(),
                    success.schema_id.clone(),
                    success.schema_version,
                    success.source_attempt_id.clone(),
                    stored_content_ref(
                        &request.run_id,
                        ids,
                        &format!("candidate.{attempt_index}"),
                        ContentKind::OutputPayload,
                        "structured output candidate content ref",
                    ),
                    stored_content_ref(
                        &request.run_id,
                        ids,
                        &format!("validation.report.{attempt_index}"),
                        ContentKind::Document,
                        "structured output validation report ref",
                    ),
                    "local structured output validation passed",
                );
                journal.append(structured_output_journal_record(
                    runtime,
                    request,
                    ids,
                    &attempt_id,
                    fingerprint,
                    source,
                    &format!("structured_output.validation_report.{attempt_index}"),
                    "validation_report",
                    StructuredOutputRecord::ValidationReport(validation_report.clone()),
                ))?;

                let canonical_json = canonical_json_bytes(&success.canonical_value)?;
                let canonical_value_ref =
                    canonical_value_ref(&request.run_id, ids, attempt_index, &canonical_json);
                runtime
                    .content_port(&request.run_id)?
                    .store_resolved_content(&canonical_value_ref, canonical_json)
                    .map_err(|error| error.to_agent_error())?;
                let validated_output = ValidatedOutput::from_validation_report(
                    ValidatedOutputParams {
                        output_id: ValidatedOutputId::new(format!(
                            "validated.output.{}",
                            ids.fragment
                        )),
                        schema_id: success.schema_id.clone(),
                        schema_version: success.schema_version,
                        schema_fingerprint: success.schema_fingerprint.clone(),
                        canonical_value_ref,
                        repair_attempts: repair_accounting.repair_attempts.clone(),
                        source_attempt_ids: vec![success.source_attempt_id.clone()],
                        content_refs: Vec::new(),
                        lineage: OutputLineage {
                            lineage_ref: LineageRef {
                                lineage_id: LineageId::new(format!(
                                    "lineage.validated.output.{}",
                                    ids.fragment
                                )),
                                source: source.clone(),
                                destination: Some(DestinationRef::with_kind(
                                    DestinationKind::Host,
                                    "destination.typed_result.p1",
                                )),
                                policy_refs: vec![contract.validation.validator_ref_policy()],
                            },
                            produced_by: EntityRef::new(
                                EntityKind::Attempt,
                                success.source_attempt_id.clone(),
                            ),
                            derived_from: vec![EntityRef::new(
                                EntityKind::Content,
                                success.candidate_content_ref.as_str(),
                            )],
                        },
                        policy_refs: vec![
                            contract.validation.validator_ref_policy(),
                            contract.repair.repair_adapter_ref_policy(),
                        ],
                        privacy: PrivacyClass::ContentRefsOnly,
                        redacted_summary: "structured output validated with refs only".to_string(),
                    },
                    &validation_report,
                )?;
                journal.append(structured_output_journal_record(
                    runtime,
                    request,
                    ids,
                    &attempt_id,
                    fingerprint,
                    source,
                    &format!("structured_output.validated_output.{attempt_index}"),
                    "validated_output",
                    StructuredOutputRecord::ValidatedOutput(validated_output.clone()),
                ))?;

                let publication = TypedResultPublicationRecord::published(&validated_output)?;
                let publication_cursor = journal.append(structured_output_journal_record(
                    runtime,
                    request,
                    ids,
                    &attempt_id,
                    fingerprint,
                    source,
                    &format!("structured_output.typed_publication.{attempt_index}"),
                    "typed_result_publication",
                    StructuredOutputRecord::TypedResultPublication(publication.clone()),
                ))?;
                validate_typed_result_publication_order(&[
                    ValidatedOutputPublicationStep::ValidationReport(validation_report.clone()),
                    ValidatedOutputPublicationStep::ValidatedOutput(validated_output.clone()),
                    ValidatedOutputPublicationStep::TypedResultPublication(publication.clone()),
                ])?;
                events.publish(event_frame(
                    &request.run_id,
                    &request.agent_id,
                    Some(ids.turn_id.clone()),
                    Some(attempt_id.clone()),
                    event_ids.next(),
                    EventFamily::StructuredOutput,
                    EventKind::StructuredOutputValidated,
                    "structured output validated",
                    Some(publication_cursor),
                    fingerprint.to_string(),
                    ids,
                ))?;

                return Ok(P1StructuredOutputSuccess {
                    final_output: response.output_text,
                    artifacts: StructuredOutputArtifacts {
                        validation_reports: vec![validation_report],
                        validated_output,
                        typed_result_publication: publication,
                    },
                });
            }
            Err(report) => {
                let failed_cursor = journal.append(structured_output_journal_record(
                    runtime,
                    request,
                    ids,
                    &attempt_id,
                    fingerprint,
                    source,
                    &format!("structured_output.validation_failed.{attempt_index}"),
                    "validation_failed",
                    StructuredOutputRecord::Validation(report.record.clone()),
                ))?;
                events.publish(event_frame(
                    &request.run_id,
                    &request.agent_id,
                    Some(ids.turn_id.clone()),
                    Some(attempt_id.clone()),
                    event_ids.next(),
                    EventFamily::StructuredOutput,
                    EventKind::StructuredOutputValidationFailed,
                    "structured output validation failed",
                    Some(failed_cursor),
                    fingerprint.to_string(),
                    ids,
                ))?;
                reports.push(report.clone());

                match repair_controller.next_attempt(contract, &report, &repair_accounting) {
                    RepairDecision::Attempt { prompt, record } => {
                        repair_accounting.record_attempt(prompt.repair_attempt_id.clone());
                        let repair_cursor = journal.append(structured_output_journal_record(
                            runtime,
                            request,
                            ids,
                            &attempt_id,
                            fingerprint,
                            source,
                            &format!("structured_output.repair_requested.{attempt_index}"),
                            "repair_requested",
                            StructuredOutputRecord::Repair(record),
                        ))?;
                        events.publish(event_frame(
                            &request.run_id,
                            &request.agent_id,
                            Some(ids.turn_id.clone()),
                            Some(attempt_id.clone()),
                            event_ids.next(),
                            EventFamily::StructuredOutput,
                            EventKind::StructuredOutputRepairRequested,
                            "structured output repair requested",
                            Some(repair_cursor),
                            fingerprint.to_string(),
                            ids,
                        ))?;

                        let repair_attempt_id = crate::ids::AttemptId::new(format!(
                            "attempt.p1.{}.repair.{}",
                            ids.fragment, attempt_index
                        ));
                        let repair_effect_id = append_provider_repair_attempt_started(
                            runtime,
                            request,
                            ids,
                            event_ids,
                            fingerprint,
                            source,
                            repair_attempt_id.clone(),
                            provider_route_id,
                            provider_model_id,
                            base_provider_request.messages.len() as u32 + 1,
                        )?;
                        let repair_request =
                            repair_provider_request(base_provider_request, &prompt.prompt_summary);
                        let repair_response = provider.complete(&repair_request)?;
                        response = append_model_attempt_completion(
                            runtime,
                            request,
                            ids,
                            event_ids,
                            fingerprint,
                            source,
                            repair_attempt_id.clone(),
                            provider_route_id,
                            provider_model_id,
                            repair_request.messages.len() as u32,
                            repair_effect_id,
                            repair_response,
                        )?;
                        attempt_id = repair_attempt_id;
                    }
                    RepairDecision::Exhausted { failure: _, record } => {
                        let failure = crate::validation::TerminalValidationFailure::from_reports(
                            &reports,
                            repair_accounting.repair_attempts.clone(),
                            record.retry_exhausted,
                        );
                        let record =
                            repair_exhaustion_record_from_failure(&failure, record.reason.clone());
                        journal.append(structured_output_journal_record(
                            runtime,
                            request,
                            ids,
                            &attempt_id,
                            fingerprint,
                            source,
                            &format!("structured_output.repair_exhausted.{attempt_index}"),
                            "repair_exhausted",
                            StructuredOutputRecord::RepairExhaustion(record),
                        ))?;
                        let terminal_record_cursor =
                            journal.append(structured_output_journal_record(
                                runtime,
                                request,
                                ids,
                                &attempt_id,
                                fingerprint,
                                source,
                                &format!("structured_output.terminal_failure.{attempt_index}"),
                                "failed",
                                StructuredOutputRecord::Validation(failure.record.clone()),
                            ))?;
                        events.publish(event_frame(
                            &request.run_id,
                            &request.agent_id,
                            Some(ids.turn_id.clone()),
                            Some(attempt_id),
                            event_ids.next(),
                            EventFamily::StructuredOutput,
                            EventKind::StructuredOutputFailed,
                            "structured output failed",
                            Some(terminal_record_cursor),
                            fingerprint.to_string(),
                            ids,
                        ))?;
                        return Err(failure.as_agent_error());
                    }
                }
            }
        }
    }
}

fn append_provider_repair_attempt_started(
    runtime: &AgentRuntime,
    request: &RunRequest,
    ids: &P0Ids,
    event_ids: &mut EventIdSequence,
    fingerprint: &str,
    source: &SourceRef,
    attempt_id: crate::ids::AttemptId,
    provider_route_id: &str,
    provider_model_id: &str,
    request_message_count: u32,
) -> Result<EffectId, AgentError> {
    let journal = runtime.journal_port(&request.run_id)?;
    let events = runtime.event_bus_port(&request.run_id)?;
    let effect_id = EffectId::new(format!(
        "effect.p1.{}.provider_repair.{}",
        ids.fragment,
        attempt_id.as_str()
    ));
    let provider_effect_intent = journal_record(
        &request.run_id,
        &request.agent_id,
        Some(ids.turn_id.clone()),
        Some(attempt_id.clone()),
        runtime.next_journal_seq(),
        ids.record(&format!(
            "provider.repair.effect.intent.{}",
            attempt_id.as_str()
        )),
        JournalRecordKind::EffectIntent,
        "effect",
        "provider_repair_intent",
        source.clone(),
        fingerprint.to_string(),
        JournalRecordPayload::EffectIntent(EffectIntent {
            effect_id: effect_id.clone(),
            kind: EffectKind::ProviderRequest,
            subject_ref: EntityRef::run(request.run_id.clone()),
            source: source.clone(),
            destination: Some(DestinationRef::with_kind(
                DestinationKind::Provider,
                "destination.provider.p1_repair",
            )),
            policy_refs: vec![PolicyRef::with_kind(
                PolicyKind::RuntimePackage,
                "policy.p1.provider_repair_request",
            )],
            idempotency_key: Some(IdempotencyKey::new(format!(
                "idempotency.p1.{}.{}",
                ids.fragment,
                attempt_id.as_str()
            ))),
            dedupe_key: None,
            content_refs: Vec::new(),
            redacted_summary: "provider repair request intent".to_string(),
        }),
    );
    let provider_effect_cursor = journal.append(provider_effect_intent)?;
    let model_intent = journal_record(
        &request.run_id,
        &request.agent_id,
        Some(ids.turn_id.clone()),
        Some(attempt_id.clone()),
        runtime.next_journal_seq(),
        ids.record(&format!("model.repair.intent.{}", attempt_id.as_str())),
        JournalRecordKind::ModelAttempt,
        "model",
        "repair_request_projected",
        source.clone(),
        fingerprint.to_string(),
        JournalRecordPayload::ModelAttempt(ModelAttemptRecord {
            provider_route_id: provider_route_id.to_string(),
            provider_model_id: provider_model_id.to_string(),
            request_message_count,
            stop_reason: None,
            usage: None,
        }),
    );
    let model_intent_cursor = journal.append(model_intent)?;
    events.publish(event_frame(
        &request.run_id,
        &request.agent_id,
        Some(ids.turn_id.clone()),
        Some(attempt_id.clone()),
        event_ids.next(),
        EventFamily::Turn,
        EventKind::ProviderRequestProjected,
        "provider repair request projected",
        Some(provider_effect_cursor),
        fingerprint.to_string(),
        ids,
    ))?;
    events.publish(event_frame(
        &request.run_id,
        &request.agent_id,
        Some(ids.turn_id.clone()),
        Some(attempt_id),
        event_ids.next(),
        EventFamily::Model,
        EventKind::ModelAttemptStarted,
        "model repair attempt started",
        Some(model_intent_cursor),
        fingerprint.to_string(),
        ids,
    ))?;
    Ok(effect_id)
}

fn repair_provider_request(
    base_provider_request: &ProviderRequest,
    prompt_summary: &str,
) -> ProviderRequest {
    let mut request = base_provider_request.clone();
    request.messages.push(ProviderMessage {
        role: ProviderMessageRole::User,
        content: prompt_summary.to_string(),
        privacy: PrivacyClass::ContentRefsOnly,
        projected_metadata: None,
    });
    request.projection_item_count = request.messages.len();
    request
}

fn structured_output_journal_record(
    runtime: &AgentRuntime,
    request: &RunRequest,
    ids: &P0Ids,
    attempt_id: &crate::ids::AttemptId,
    fingerprint: &str,
    source: &SourceRef,
    record_label: &str,
    event_kind: &str,
    payload: StructuredOutputRecord,
) -> JournalRecord {
    journal_record(
        &request.run_id,
        &request.agent_id,
        Some(ids.turn_id.clone()),
        Some(attempt_id.clone()),
        runtime.next_journal_seq(),
        ids.record(record_label),
        JournalRecordKind::StructuredOutput,
        "structured_output",
        event_kind,
        source.clone(),
        fingerprint.to_string(),
        JournalRecordPayload::StructuredOutput(payload),
    )
}

fn candidate_content_ref(ids: &P0Ids, attempt_index: u8) -> ContentRefId {
    ContentRefId::new(format!(
        "content.ref.p1.{}.candidate.{}",
        ids.fragment, attempt_index
    ))
}

fn canonical_json_bytes(value: &Value) -> Result<Vec<u8>, AgentError> {
    serde_json::to_vec(value)
        .map_err(|error| AgentError::contract_violation(format!("canonical JSON failed: {error}")))
}

fn canonical_value_ref(
    run_id: &RunId,
    ids: &P0Ids,
    attempt_index: u8,
    canonical_json: &[u8],
) -> StoredContentRef {
    let content_hash = sha256_content_hash(canonical_json);
    let digest_fragment = content_hash
        .strip_prefix("sha256:")
        .expect("sha256 hash prefix")
        .chars()
        .take(12)
        .collect::<String>();
    let mut content_ref = StoredContentRef::new(
        ContentId::new(format!(
            "content.p1.{}.canonical.{}.{}",
            ids.fragment, attempt_index, digest_fragment
        )),
        ContentVersion::new("v1"),
        ContentKind::OutputPayload,
        ContentScope::Run,
        EntityRef::run(run_id.clone()),
        SourceRef::with_kind(SourceKind::Sdk, "source.sdk.p1_structured_output"),
        AdapterRef::new("resolver.content.p1"),
        "validated structured output canonical JSON",
    );
    content_ref.mime = Some("application/json".to_string());
    content_ref.size_bytes = Some(canonical_json.len() as u64);
    content_ref.content_hash = Some(content_hash);
    content_ref.privacy_class = PrivacyClass::ContentRefsOnly;
    content_ref.retention_class = RetentionClass::RunScoped;
    content_ref.trust_class = TrustClass::SdkGenerated;
    content_ref
}

fn stored_content_ref(
    run_id: &RunId,
    ids: &P0Ids,
    label: &str,
    kind: ContentKind,
    redacted_summary: &str,
) -> StoredContentRef {
    let mut content_ref = StoredContentRef::new(
        ContentId::new(format!("content.p1.{}.{}", ids.fragment, label)),
        ContentVersion::new("v1"),
        kind,
        ContentScope::Run,
        EntityRef::run(run_id.clone()),
        SourceRef::with_kind(SourceKind::Sdk, "source.sdk.p1_structured_output"),
        AdapterRef::new("resolver.content.p1"),
        redacted_summary,
    );
    content_ref.mime = Some("application/json".to_string());
    content_ref.size_bytes = Some(128);
    content_ref.content_hash =
        Some("sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd".to_string());
    content_ref.privacy_class = PrivacyClass::ContentRefsOnly;
    content_ref.retention_class = RetentionClass::RunScoped;
    content_ref.trust_class = TrustClass::SdkGenerated;
    content_ref
}

fn sha256_content_hash(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("sha256:{digest:x}")
}

fn build_text_projection(
    request: &RunRequest,
    ids: &P0Ids,
    runtime_package_fingerprint: &str,
) -> Result<ContextProjection, AgentError> {
    let policy_ref = sdk_context_policy_ref();
    let message = AgentMessage::user_text(
        ids.input_message_id.clone(),
        request.input.clone(),
        request.source.clone(),
        policy_ref.clone(),
    );
    let mut contribution = ContextContribution::new(
        ids.contribution_id.clone(),
        ContextContributionKind::UserInput,
        EntityRef::message(ids.input_message_id.clone()),
        request.source.clone(),
        policy_ref.clone(),
        "user text input",
    );
    contribution.inline_redacted_summary = Some(request.input.clone());
    contribution.privacy_class = PrivacyClass::ContentRefsOnly;

    let item = ContextItem::admit(
        contribution,
        ids.context_item_id.clone(),
        DestinationRef::with_kind(DestinationKind::Provider, "destination.provider.p0_text"),
        ProjectionRole::User,
    );

    ContextProjection::build(
        ids.projection_id.clone(),
        vec![message],
        vec![item],
        Vec::new(),
        DestinationRef::with_kind(DestinationKind::Provider, "destination.provider.p0_text"),
        ContextBudgetSummary::default(),
        PolicyRef::with_kind(PolicyKind::Redaction, "policy.redaction.default"),
        runtime_package_fingerprint,
    )
}

fn journal_record(
    run_id: &RunId,
    agent_id: &AgentId,
    turn_id: Option<TurnId>,
    attempt_id: Option<crate::ids::AttemptId>,
    journal_seq: u64,
    record_id: String,
    record_kind: JournalRecordKind,
    event_family: &str,
    event_kind: &str,
    source: SourceRef,
    runtime_package_fingerprint: String,
    payload: JournalRecordPayload,
) -> JournalRecord {
    let base = JournalRecordBase {
        journal_seq,
        record_id,
        run_id: run_id.clone(),
        agent_id: agent_id.clone(),
        turn_id,
        attempt_id,
        source: source.clone(),
        destination: Some(DestinationRef::with_kind(
            DestinationKind::Journal,
            "destination.journal.p0_text",
        )),
        causal_refs: Vec::new(),
        tags: vec!["p0_text_run".to_string()],
        timestamp_millis: 0,
        runtime_package_fingerprint: runtime_package_fingerprint.clone(),
        privacy: PrivacyClass::ContentRefsOnly,
        redaction_policy_id: "policy.redaction.default".to_string(),
        checkpoint_ref: None,
    };
    JournalRecord {
        journal_schema_version: JOURNAL_SCHEMA_VERSION,
        journal_seq: base.journal_seq,
        record_id: base.record_id,
        record_kind,
        run_id: base.run_id.clone(),
        agent_id: base.agent_id.clone(),
        turn_id: base.turn_id.clone(),
        attempt_id: base.attempt_id.clone(),
        subject_ref: EntityRef::run(base.run_id.clone()),
        related_refs: Vec::new(),
        causal_refs: base.causal_refs,
        source: base.source.clone(),
        destination: base.destination.clone(),
        correlation_keys: Vec::new(),
        tags: base.tags.clone(),
        delivery_semantics: "journal_backed".to_string(),
        event_index: EventIndexProjection {
            run_id: base.run_id.clone(),
            agent_id: base.agent_id.clone(),
            turn_id: base.turn_id.clone(),
            event_family: event_family.to_string(),
            event_kind: event_kind.to_string(),
            source: base.source,
            destination: base.destination,
            subject_ref: EntityRef::run(base.run_id.clone()),
            related_refs: Vec::new(),
            correlation_keys: Vec::new(),
            tags: base.tags,
            privacy_class: base.privacy,
            delivery_semantics: "journal_backed".to_string(),
        },
        timestamp_millis: base.timestamp_millis,
        runtime_package_fingerprint,
        privacy: base.privacy,
        content_refs: Vec::new(),
        redaction_policy_id: base.redaction_policy_id,
        idempotency_key: None,
        dedupe_key: None,
        checkpoint_ref: base.checkpoint_ref,
        payload,
    }
}

fn event_frame(
    run_id: &RunId,
    agent_id: &AgentId,
    turn_id: Option<TurnId>,
    attempt_id: Option<crate::ids::AttemptId>,
    event_seq: u64,
    event_family: EventFamily,
    event_kind: EventKind,
    redacted_summary: &str,
    journal_cursor: Option<crate::journal::JournalCursor>,
    runtime_package_fingerprint: String,
    ids: &P0Ids,
) -> EventFrame {
    let event = AgentEvent::with_redacted_summary(
        EventEnvelope {
            schema_version: EVENT_SCHEMA_VERSION,
            event_id: EventId::new(ids.event(event_seq)),
            event_seq,
            event_family,
            event_kind,
            payload_schema_version: 1,
            timestamp: "1970-01-01T00:00:00Z".to_string(),
            recorded_at: "1970-01-01T00:00:00Z".to_string(),
            run_id: run_id.clone(),
            agent_id: agent_id.clone(),
            turn_id,
            attempt_id,
            message_id: None,
            context_item_id: None,
            trace_id: ids.trace_id.clone(),
            span_id: SpanId::new(ids.span(event_seq)),
            parent_event_id: None,
            caused_by: None,
            subject_ref: EntityRef::run(run_id.clone()),
            related_refs: Vec::new(),
            causal_refs: Vec::new(),
            correlation: EventCorrelation::default(),
            tags: Vec::new(),
            source: SourceRef::with_kind(SourceKind::Sdk, "source.sdk.p0_text_run"),
            destination: Some(DestinationRef::with_kind(
                DestinationKind::EventStream,
                "destination.event_stream.p0_text",
            )),
            policy_refs: Vec::new(),
            journal_cursor,
            state_before: None,
            state_after: None,
            delivery_semantics: EventDeliverySemantics::JournalBacked,
            privacy: PrivacyClass::ContentRefsOnly,
            content_capture: ContentCaptureMode::Off,
            redaction_policy_id: "policy.redaction.default".to_string(),
            runtime_package_fingerprint,
        },
        redacted_summary,
    );
    EventFrame {
        cursor: event.envelope.cursor(EventStreamScope::Run(run_id.clone())),
        event,
        archive_cursor: None,
        overflow: None,
    }
}

fn terminal_status(stop_reason: &ProviderStopReason) -> RunStatus {
    match stop_reason {
        ProviderStopReason::EndTurn => RunStatus::Completed,
        ProviderStopReason::Cancelled => RunStatus::Cancelled,
        ProviderStopReason::MaxTokens
        | ProviderStopReason::ProviderError
        | ProviderStopReason::Unknown => RunStatus::Failed,
    }
}

fn terminal_event_kind(status: &RunStatus) -> EventKind {
    match status {
        RunStatus::Completed => EventKind::RunCompleted,
        RunStatus::Cancelled => EventKind::RunCancelled,
        _ => EventKind::RunFailed,
    }
}

struct P0Ids {
    fragment: String,
    turn_id: TurnId,
    attempt_id: crate::ids::AttemptId,
    input_message_id: MessageId,
    output_message_id: MessageId,
    contribution_id: ContextContributionId,
    context_item_id: ContextItemId,
    projection_id: ContextProjectionId,
    provider_effect_id: EffectId,
    terminal_effect_id: EffectId,
    trace_id: TraceId,
}

#[derive(Default)]
struct EventIdSequence {
    next: u64,
}

impl EventIdSequence {
    fn next(&mut self) -> u64 {
        self.next += 1;
        self.next
    }
}

impl P0Ids {
    fn new(run_id: &RunId) -> Self {
        let fragment = stable_fragment(run_id.as_str());
        Self {
            turn_id: TurnId::new(format!("turn.p0.{fragment}")),
            attempt_id: crate::ids::AttemptId::new(format!("attempt.p0.{fragment}")),
            input_message_id: MessageId::new(format!("message.p0.{fragment}.input")),
            output_message_id: MessageId::new(format!("message.p0.{fragment}.output")),
            contribution_id: ContextContributionId::new(format!(
                "context.contribution.p0.{fragment}"
            )),
            context_item_id: ContextItemId::new(format!("context.item.p0.{fragment}")),
            projection_id: ContextProjectionId::new(format!("context.projection.p0.{fragment}")),
            provider_effect_id: EffectId::new(format!("effect.p0.{fragment}.provider_request")),
            terminal_effect_id: EffectId::new(format!("effect.p0.{fragment}.terminal")),
            trace_id: TraceId::new(format!("trace.p0.{fragment}")),
            fragment,
        }
    }

    fn record(&self, label: &str) -> String {
        format!("journal.p0.{}.{}", self.fragment, label)
    }

    fn event(&self, event_seq: u64) -> String {
        format!("event.p0.{}.{}", self.fragment, event_seq)
    }

    fn span(&self, event_seq: u64) -> String {
        format!("span.p0.{}.{}", self.fragment, event_seq)
    }
}

fn stable_fragment(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    digest[..6]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}
