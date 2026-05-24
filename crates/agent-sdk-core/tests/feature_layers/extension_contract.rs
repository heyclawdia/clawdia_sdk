use std::sync::Arc;

use agent_sdk_core::{
    AgentId, ApprovalBroker, ApprovalDecision, ApprovalDispatchResponse, CapabilityId,
    DestinationKind, DestinationRef, EffectKind, EntityKind, ExtensionActionCapability,
    ExtensionActionContext, ExtensionActionEventKind, ExtensionActionExecutorRegistry,
    ExtensionActionId, ExtensionActionKind, ExtensionActionOutcomeStatus,
    ExtensionActionRegistrySnapshot, ExtensionActionRequest, ExtensionActionRequestId,
    ExtensionBridgeRef, ExtensionId, ExtensionPackageResolution, ExtensionProtocolErrorKind,
    ExtensionProtocolRequestId, ExtensionProtocolVersion, ExtensionVersion, JournalRecordPayload,
    PolicyKind, PolicyRef, ResolvedExtensionPackage, RunId, SourceKind, SourceRef,
    testing::{
        FakeJournalStore, ScriptedApprovalDispatcher, ScriptedExtensionActionExecutor,
        normalize_json_value, read_fixture,
    },
};
use serde_json::{Value, json};

#[test]
fn core_extension_capability_helper_lowers_to_explicit_capability_fields() {
    let helper = CoreCapabilitiesFixture::helper();
    let explicit = CoreCapabilitiesFixture::explicit();

    assert_eq!(helper, explicit);

    let encoded = serde_json::to_value(&helper).expect("capabilities serialize");
    let audit = agent_sdk_core::audit_core_extension_capabilities(&encoded);
    assert!(audit.forbidden_fields.is_empty());
    assert_eq!(
        normalize_json_value(encoded),
        read_fixture("tests/fixtures/extensions/core-capabilities-helper.json")
            .expect("core capabilities fixture")
    );
}

#[test]
fn host_manifest_fields_are_reported_but_never_package_authority() {
    let host_manifest_like = json!({
        "extension_id": "extension.reviewer",
        "version": "1.2.0",
        "runtime": "subprocess_json_rpc_ndjson",
        "app_event_subscriptions": ["agent.host.*"],
        "browser_safe_exports": ["@agent-sdk/extension-sdk/browser-safe"],
        "trust_state": "host_trusted",
        "marketplace": {"listing": "host-owned"},
        "core_capabilities": CoreCapabilitiesFixture::helper(),
    });

    let audit = agent_sdk_core::audit_core_extension_capabilities(&host_manifest_like);

    assert!(audit.has_forbidden_field("runtime"));
    assert!(audit.has_forbidden_field("app_event_subscriptions"));
    assert!(audit.has_forbidden_field("browser_safe_exports"));
    assert!(audit.has_forbidden_field("trust_state"));
    assert!(audit.has_forbidden_field("marketplace"));
}

#[test]
fn host_policy_resolves_extension_actions_into_catalog_and_sidecars() {
    let resolved = resolved_package();

    assert_eq!(
        format!("{:?}", resolved.catalog_snapshot.source_kind),
        "Extension"
    );
    assert_eq!(resolved.action_capabilities.len(), 1);
    assert_eq!(
        resolved.action_capabilities[0].capability_id,
        CapabilityId::new("cap.extension.reviewer.submit_ui_effect")
    );
    assert_eq!(resolved.action_sidecars.len(), 1);
    assert_eq!(
        resolved.action_sidecars[0].bridge_ref,
        ExtensionBridgeRef::new("bridge.extension.reviewer")
    );
    assert_eq!(
        resolved.action_sidecars[0].policy_refs,
        vec![approval_policy("policy.extension.action")]
    );

    assert_eq!(
        extension_package_summary(&resolved),
        read_fixture("tests/fixtures/extensions/extension-package-snapshot.json")
            .expect("package snapshot fixture")
    );
}

#[test]
fn extension_action_records_effect_intent_before_host_action_and_terminal_result() {
    let resolved = resolved_package();
    let snapshot =
        ExtensionActionRegistrySnapshot::from_resolved_package(&resolved).expect("snapshot");
    let executor = Arc::new(ScriptedExtensionActionExecutor::new(
        ExtensionBridgeRef::new("bridge.extension.reviewer"),
        agent_sdk_core::ExtensionActionExecutionOutput::completed("host action accepted"),
    ));
    let mut executors = ExtensionActionExecutorRegistry::new();
    executors
        .register(executor.clone())
        .expect("register executor");
    let coordinator = agent_sdk_core::ExtensionActionCoordinator::new(snapshot, executors)
        .with_approval_broker(ApprovalBroker::default());
    let journal = FakeJournalStore::default();
    let dispatcher = ScriptedApprovalDispatcher::new(ApprovalDispatchResponse::decision(
        ApprovalDecision::approved("actor.host.user"),
    ));

    let outcome = coordinator
        .execute(
            &journal,
            action_request(),
            action_context(&resolved),
            Some(&dispatcher),
        )
        .expect("action executes");

    assert_eq!(executor.call_count(), 1);
    assert_eq!(outcome.status, ExtensionActionOutcomeStatus::Completed);
    assert_eq!(
        outcome
            .events
            .iter()
            .map(|event| &event.kind)
            .collect::<Vec<_>>(),
        vec![
            &ExtensionActionEventKind::Submitted,
            &ExtensionActionEventKind::Started,
            &ExtensionActionEventKind::Completed,
        ]
    );
    assert_eq!(
        executor.calls()[0].effect_intent.kind,
        EffectKind::ExtensionAction
    );
    assert_eq!(
        executor.calls()[0].effect_intent.subject_ref.kind,
        EntityKind::ExtensionAction
    );
    assert_eq!(
        extension_action_journal_summary(&journal.records()),
        read_fixture("tests/fixtures/extensions/action-intent-result.json")
            .expect("action journal fixture")
    );
    let records = journal.records();
    let feature_statuses = records
        .iter()
        .filter_map(|record| match &record.payload {
            JournalRecordPayload::ExtensionAction(action) => Some(&action.status),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        feature_statuses,
        vec![
            &agent_sdk_core::ExtensionActionRecordStatus::Submitted,
            &agent_sdk_core::ExtensionActionRecordStatus::Started,
            &agent_sdk_core::ExtensionActionRecordStatus::Completed,
        ],
        "successful extension action lifecycle must be durable canonical ExtensionAction payloads"
    );
}

#[test]
fn extension_action_missing_dispatcher_denies_before_host_action() {
    let resolved = resolved_package();
    let snapshot =
        ExtensionActionRegistrySnapshot::from_resolved_package(&resolved).expect("snapshot");
    let executor = Arc::new(ScriptedExtensionActionExecutor::new(
        ExtensionBridgeRef::new("bridge.extension.reviewer"),
        agent_sdk_core::ExtensionActionExecutionOutput::completed("must not execute"),
    ));
    let mut executors = ExtensionActionExecutorRegistry::new();
    executors
        .register(executor.clone())
        .expect("register executor");
    let coordinator = agent_sdk_core::ExtensionActionCoordinator::new(snapshot, executors)
        .with_approval_broker(ApprovalBroker::default());
    let journal = FakeJournalStore::default();

    let outcome = coordinator
        .execute(&journal, action_request(), action_context(&resolved), None)
        .expect("missing dispatcher records denial");

    assert_eq!(outcome.status, ExtensionActionOutcomeStatus::Denied);
    assert_eq!(executor.call_count(), 0);
    assert!(outcome.intent_cursor.is_none());
    assert!(outcome.terminal_cursor.is_none());
    assert!(
        outcome
            .events
            .iter()
            .any(|event| event.kind == ExtensionActionEventKind::Denied)
    );
    assert!(
        journal.records().iter().any(|record| matches!(
            record.payload,
            JournalRecordPayload::ExtensionAction(agent_sdk_core::ExtensionActionRecord {
                status: agent_sdk_core::ExtensionActionRecordStatus::Denied,
                ..
            })
        )),
        "denied-before-execution extension outcomes must be durable"
    );
}

#[test]
fn extension_cannot_self_approve_action() {
    let resolved = resolved_package();
    let snapshot =
        ExtensionActionRegistrySnapshot::from_resolved_package(&resolved).expect("snapshot");
    let executor = Arc::new(ScriptedExtensionActionExecutor::new(
        ExtensionBridgeRef::new("bridge.extension.reviewer"),
        agent_sdk_core::ExtensionActionExecutionOutput::completed("must not execute"),
    ));
    let mut executors = ExtensionActionExecutorRegistry::new();
    executors
        .register(executor.clone())
        .expect("register executor");
    let coordinator = agent_sdk_core::ExtensionActionCoordinator::new(snapshot, executors)
        .with_approval_broker(ApprovalBroker::default());
    let journal = FakeJournalStore::default();
    let request = action_request();
    let dispatcher = ScriptedApprovalDispatcher::new(ApprovalDispatchResponse::decision(
        ApprovalDecision::Approved {
            actor_ref: request.source.clone(),
        },
    ));

    let outcome = coordinator
        .execute(
            &journal,
            request,
            action_context(&resolved),
            Some(&dispatcher),
        )
        .expect("self approval records denial");

    assert_eq!(outcome.status, ExtensionActionOutcomeStatus::Denied);
    assert_eq!(executor.call_count(), 0);
    assert_eq!(
        outcome.approval_outcome.unwrap().reason_code,
        "approval.extension_self_response"
    );
}

#[test]
fn extension_action_route_missing_policy_fails_closed_before_execution() {
    let mut resolved = resolved_package();
    resolved.action_sidecars[0].policy_refs.clear();

    let error = ExtensionActionRegistrySnapshot::from_resolved_package(&resolved)
        .expect_err("missing action policy must fail closed");

    assert_eq!(error.kind(), agent_sdk_core::AgentErrorKind::InvalidPackage);
    assert!(error.context().message.contains("policy_refs"));
}

#[test]
fn protocol_response_id_mismatch_records_recovery_marker() {
    let expected = ExtensionProtocolRequestId::new("protocol.request.init.1");
    let actual = ExtensionProtocolRequestId::new("protocol.response.other");
    let error = agent_sdk_core::validate_extension_protocol_response_id(&expected, &actual)
        .expect_err("response id mismatch fails");
    assert_eq!(error.kind, ExtensionProtocolErrorKind::ResponseIdMismatch);

    let journal = FakeJournalStore::default();
    let recovery = agent_sdk_core::recover_extension_protocol_error(
        &journal,
        error,
        protocol_context(),
        vec![approval_policy("policy.extension.protocol")],
    )
    .expect("protocol recovery journaled");

    assert_eq!(recovery.cursor.as_str(), "journal.1");
    assert_eq!(
        protocol_recovery_summary(&journal.records()),
        read_fixture("tests/fixtures/extensions/protocol-error-recovery.json")
            .expect("protocol recovery fixture")
    );
}

#[test]
fn unsupported_protocol_version_fails_before_capabilities_enter_core() {
    let error = ExtensionProtocolVersion::negotiate(99)
        .expect_err("unsupported version rejected before core capabilities");

    assert_eq!(
        error.kind,
        ExtensionProtocolErrorKind::UnsupportedProtocolVersion
    );
    assert!(error.redacted_summary.contains("unsupported"));
}

#[test]
fn core_extension_surface_has_no_runtime_or_package_export_imports() {
    let package_source = include_str!("../../src/package/extension.rs");
    let ports_source = include_str!("../../src/ports/extension.rs");
    let application_source = include_str!("../../src/application/extension.rs");

    for forbidden in [
        "std::process",
        "Command::new",
        "node:",
        "child_process",
        "node_modules",
        "bun",
    ] {
        assert!(!package_source.contains(forbidden));
        assert!(!ports_source.contains(forbidden));
        assert!(!application_source.contains(forbidden));
    }
}

struct CoreCapabilitiesFixture;

impl CoreCapabilitiesFixture {
    fn helper() -> agent_sdk_core::CoreExtensionCapabilities {
        agent_sdk_core::CoreExtensionCapabilities::builder(ExtensionId::new("extension.reviewer"))
            .version(ExtensionVersion::new("1.2.0"))
            .tool("review_notes")
            .action(
                ExtensionActionId::new("submit_ui_effect"),
                ExtensionActionKind::HostAction,
                DestinationRef::with_kind(DestinationKind::Host, "destination.host.ui"),
            )
            .build()
            .expect("helper builds")
    }

    fn explicit() -> agent_sdk_core::CoreExtensionCapabilities {
        agent_sdk_core::CoreExtensionCapabilities {
            extension_id: ExtensionId::new("extension.reviewer"),
            version: ExtensionVersion::new("1.2.0"),
            tools: vec![agent_sdk_core::ExtensionToolCapability::new("review_notes")],
            hooks: Vec::new(),
            providers: Vec::new(),
            subagents: Vec::new(),
            actions: vec![ExtensionActionCapability::new(
                ExtensionActionId::new("submit_ui_effect"),
                ExtensionActionKind::HostAction,
                DestinationRef::with_kind(DestinationKind::Host, "destination.host.ui"),
            )],
        }
    }
}

fn resolved_package() -> ResolvedExtensionPackage {
    ResolvedExtensionPackage::from_core_capabilities(
        CoreCapabilitiesFixture::helper(),
        ExtensionPackageResolution {
            source_ref: SourceRef::with_kind(SourceKind::Extension, "extension.reviewer"),
            catalog_id: "catalog.extension.reviewer".to_string(),
            activation_policy_ref: approval_policy("policy.extension.activation"),
            bridge_ref: ExtensionBridgeRef::new("bridge.extension.reviewer"),
            action_policy_ref: approval_policy("policy.extension.action"),
            approval_policy_ref: approval_policy("policy.extension.approval"),
            redaction_policy_id: "redaction.extension.default".to_string(),
            runtime_package_fingerprint: "runtime.package.extension.1".to_string(),
        },
    )
    .expect("extension package resolves")
}

fn action_request() -> ExtensionActionRequest {
    ExtensionActionRequest {
        request_id: ExtensionActionRequestId::new("extension.action.request.1"),
        extension_id: ExtensionId::new("extension.reviewer"),
        action_id: ExtensionActionId::new("submit_ui_effect"),
        source: SourceRef::with_kind(SourceKind::Extension, "extension.reviewer"),
        input_refs: vec![agent_sdk_core::domain::ContentRef::new(
            "content.extension.action.args.1",
        )],
        redacted_input_summary: "apply bounded UI effect".to_string(),
        idempotency_key: Some(agent_sdk_core::IdempotencyKey::new(
            "idem.extension.action.1",
        )),
        dedupe_key: None,
        runtime_package_fingerprint: "runtime.package.extension.1".to_string(),
    }
}

fn action_context(package: &ResolvedExtensionPackage) -> ExtensionActionContext {
    ExtensionActionContext {
        run_id: RunId::new("run.extension.action"),
        agent_id: AgentId::new("agent.extension.action"),
        turn_id: Some(agent_sdk_core::TurnId::new("turn.extension.action")),
        runtime_package_fingerprint: package.runtime_package_fingerprint.clone(),
        next_journal_seq: 1,
        timestamp_millis: 500,
        record_id_prefix: "journal.record.extension.action".to_string(),
        redaction_policy_id: "redaction.extension.default".to_string(),
    }
}

fn protocol_context() -> agent_sdk_core::ExtensionProtocolRecoveryContext {
    agent_sdk_core::ExtensionProtocolRecoveryContext {
        run_id: RunId::new("run.extension.protocol"),
        agent_id: AgentId::new("agent.extension.protocol"),
        source: SourceRef::with_kind(SourceKind::Extension, "extension.reviewer"),
        runtime_package_fingerprint: "runtime.package.extension.1".to_string(),
        next_journal_seq: 1,
        timestamp_millis: 900,
        record_id: "journal.record.extension.protocol.recovery".to_string(),
        redaction_policy_id: "redaction.extension.default".to_string(),
    }
}

fn approval_policy(id: &str) -> PolicyRef {
    PolicyRef::with_kind(PolicyKind::Approval, id)
}

fn extension_package_summary(package: &ResolvedExtensionPackage) -> Value {
    normalize_json_value(json!({
        "catalog": {
            "catalog_id": package.catalog_snapshot.catalog_id,
            "source_kind": format!("{:?}", package.catalog_snapshot.source_kind),
            "source_ref": package.catalog_snapshot.source_ref.as_str(),
            "activation_policy": package.catalog_snapshot.activation_policy_ref.as_str(),
            "candidates": package.catalog_snapshot.candidates.iter().map(|candidate| candidate.as_str()).collect::<Vec<_>>(),
        },
        "actions": package.action_sidecars.iter().map(|sidecar| json!({
            "sidecar_id": sidecar.sidecar_id,
            "action_id": sidecar.action_ref.action_id.as_str(),
            "capability_id": sidecar.action_ref.capability_id.as_str(),
            "kind": sidecar.action_kind,
            "bridge_ref": sidecar.bridge_ref.as_str(),
            "destination_kind": format!("{:?}", sidecar.destination.kind),
            "policy_refs": sidecar.policy_refs.iter().map(|policy| policy.as_str()).collect::<Vec<_>>(),
            "approval_policy_ref": sidecar.approval_policy_ref.as_str(),
        })).collect::<Vec<_>>(),
        "sidecars": package.package_sidecars.iter().map(|sidecar| json!({
            "sidecar_id": sidecar.sidecar_id,
            "kind": sidecar.kind,
            "version": sidecar.version,
            "policy_refs": sidecar.policy_refs.iter().map(|policy| policy.as_str()).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    }))
}

fn extension_action_journal_summary(records: &[agent_sdk_core::JournalRecord]) -> Value {
    normalize_json_value(json!({
        "records": records.iter().map(|record| {
            match &record.payload {
                agent_sdk_core::JournalRecordPayload::Approval(approval) => json!({
                    "kind": "approval",
                    "event_kind": record.event_index.event_kind,
                    "payload": match approval {
                        agent_sdk_core::ApprovalRecord::DispatchIntent { effect_intent, .. } => json!({
                            "effect_kind": effect_intent.kind,
                            "source_kind": effect_intent.source.kind,
                            "destination_kind": effect_intent.destination.as_ref().map(|destination| &destination.kind),
                            "policy_refs": effect_intent.policy_refs.iter().map(|policy| policy.as_str()).collect::<Vec<_>>(),
                        }),
                        agent_sdk_core::ApprovalRecord::DispatchResult { lifecycle_status, effect_result, .. } => json!({
                            "lifecycle_status": lifecycle_status,
                            "terminal_status": effect_result.terminal_status,
                            "summary": effect_result.redacted_summary,
                        }),
                        other => json!({"other": format!("{other:?}")}),
                    },
                }),
                agent_sdk_core::JournalRecordPayload::EffectIntent(intent) => json!({
                    "kind": "effect_intent",
                    "effect_kind": intent.kind,
                    "subject_kind": intent.subject_ref.kind,
                    "source_kind": intent.source.kind,
                    "destination_kind": intent.destination.as_ref().map(|destination| &destination.kind),
                    "policy_refs": intent.policy_refs.iter().map(|policy| policy.as_str()).collect::<Vec<_>>(),
                    "content_refs": intent.content_refs.iter().map(|content| content.as_str()).collect::<Vec<_>>(),
                }),
                agent_sdk_core::JournalRecordPayload::EffectResult(result) => json!({
                    "kind": "effect_result",
                    "terminal_status": result.terminal_status,
                    "summary": result.redacted_summary,
                }),
                agent_sdk_core::JournalRecordPayload::ExtensionAction(action) => json!({
                    "kind": "extension_action",
                    "event_kind": record.event_index.event_kind,
                    "status": action.status,
                    "action_kind": action.action_kind,
                    "policy_refs": action.policy_refs.iter().map(|policy| policy.as_str()).collect::<Vec<_>>(),
                    "has_effect_intent": action.effect_intent.is_some(),
                    "has_effect_result": action.effect_result.is_some(),
                }),
                payload => json!({
                    "kind": format!("{payload:?}"),
                }),
            }
        }).collect::<Vec<_>>(),
    }))
}

fn protocol_recovery_summary(records: &[agent_sdk_core::JournalRecord]) -> Value {
    normalize_json_value(json!({
        "records": records.iter().map(|record| {
            match &record.payload {
                agent_sdk_core::JournalRecordPayload::Recovery(recovery) => json!({
                    "record_kind": "recovery",
                    "reason": recovery.recovery_reason,
                    "policy_refs": recovery.policy_refs.iter().map(|policy| policy.as_str()).collect::<Vec<_>>(),
                    "unsafe_pending_count": recovery.unsafe_pending.len(),
                }),
                payload => json!({ "record_kind": format!("{payload:?}") }),
            }
        }).collect::<Vec<_>>(),
    }))
}
