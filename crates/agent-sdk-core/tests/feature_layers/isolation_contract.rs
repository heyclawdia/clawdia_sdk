use std::sync::{Arc, Mutex};

use agent_sdk_core::{
    AgentError, AgentErrorKind, AgentId, CleanupStatus, DestinationKind, DestinationRef,
    ExecutionEnvironment, FilesystemIsolationPolicy, IsolationCapability,
    IsolationCapabilityReport, IsolationClass, IsolationDowngradeApproval,
    IsolationLifecycleContext, IsolationLifecycleCoordinator, IsolationMatchStatus,
    IsolationReadinessProfile, IsolationRequirement, IsolationRequirementSnapshot,
    IsolationRuntimeRegistry, IsolationTrustField, JournalCursor, JournalRecord,
    JournalRecordPayload, NetworkIsolationPolicy, PolicyKind, PolicyRef, RuntimePackage,
    RuntimePackageId, RuntimePackageSidecarId, SecretExposurePolicy, SourceKind, SourceRef,
    WorkspaceMountMode,
    testing::{FakeIsolationRuntime, FakeJournalStore, read_fixture},
};
use serde_json::{Value, json};

#[test]
fn isolated_helper_lowers_to_environment_spec_and_package_snapshot() {
    let environment = isolated_environment(NetworkIsolationPolicy::Allowlist {
        hosts: vec!["crates.io".to_string()],
    });
    let snapshot = requirement_snapshot(&environment);
    let package = package_with_isolation(snapshot.clone());

    assert_eq!(
        environment.spec.requirement.minimum_class,
        IsolationClass::Sandbox
    );
    assert_eq!(
        environment.spec.filesystem.workspace.mode,
        WorkspaceMountMode::Snapshot
    );
    assert_eq!(
        package
            .canonical_snapshot()
            .expect("canonical snapshot")
            .isolation_requirements[0],
        snapshot
    );
    assert_eq!(
        sidecar_summary(&environment, &package),
        read_fixture("tests/fixtures/isolation/environment-sidecar.json")
            .expect("environment fixture")
    );

    let changed_environment = isolated_environment(NetworkIsolationPolicy::Disabled);
    let changed_package = package_with_isolation(requirement_snapshot(&changed_environment));
    assert_ne!(
        package.fingerprint().expect("fingerprint"),
        changed_package.fingerprint().expect("changed fingerprint"),
        "network policy is an execution-affecting isolation fingerprint input"
    );
}

#[test]
fn unsupported_adapter_denies_without_allowed_fallback() {
    let environment = isolated_environment(NetworkIsolationPolicy::Disabled);
    let package = package_with_isolation(requirement_snapshot(&environment));
    let runtime = Arc::new(FakeIsolationRuntime::unsupported_host(
        "runtime.fake.unsupported",
        ["missing portable isolation runtime"],
    ));
    let mut registry = IsolationRuntimeRegistry::new();
    registry
        .register(runtime.clone())
        .expect("runtime registers");
    let journal = FakeJournalStore::default();
    let coordinator = IsolationLifecycleCoordinator::new(Arc::new(journal), registry);

    let outcome = coordinator
        .select_environment(
            &package,
            &environment,
            IsolationLifecycleContext::test(package.fingerprint().unwrap()),
            None,
        )
        .expect("unsupported host returns denied outcome");

    assert_eq!(outcome.status, IsolationMatchStatus::UnsupportedHost);
    assert_eq!(runtime.call_count(), 1, "capability report is allowed");
    assert_eq!(
        downgrade_summary(&outcome),
        read_fixture("tests/fixtures/isolation/unsupported-host-denied.json")
            .expect("unsupported fixture")
    );
}

#[test]
fn container_required_denies_hostprocess_fallback() {
    let environment = container_environment();
    let package = package_with_isolation(requirement_snapshot(&environment));
    let runtime = Arc::new(FakeIsolationRuntime::host_process_only("runtime.fake.host"));
    let mut registry = IsolationRuntimeRegistry::new();
    registry
        .register(runtime.clone())
        .expect("runtime registers");
    let journal = FakeJournalStore::default();
    let coordinator = IsolationLifecycleCoordinator::new(Arc::new(journal), registry);

    let outcome = coordinator
        .select_environment(
            &package,
            &environment,
            IsolationLifecycleContext::test(package.fingerprint().unwrap()),
            Some(IsolationDowngradeApproval::approved_for_tool(
                "decision.tool.approved",
            )),
        )
        .expect("host fallback is denied");

    assert_eq!(outcome.status, IsolationMatchStatus::DowngradeDenied);
    assert_eq!(runtime.start_process_call_count(), 0);

    let mut test_only_environment = environment.clone();
    test_only_environment.spec.requirement = test_only_environment
        .spec
        .requirement
        .fallback_test_only_host_process();
    let denied = coordinator
        .select_environment(
            &package,
            &test_only_environment,
            IsolationLifecycleContext {
                readiness_profile: IsolationReadinessProfile::Production,
                ..IsolationLifecycleContext::test(package.fingerprint().unwrap())
            },
            Some(IsolationDowngradeApproval::approved_for_isolation(
                "decision.isolation.host",
            )),
        )
        .expect("test-only host fallback is denied in production");
    assert_eq!(denied.status, IsolationMatchStatus::DowngradeDenied);
}

#[test]
fn capability_or_trust_gap_requires_package_and_policy_decision() {
    let mut environment = isolated_environment(NetworkIsolationPolicy::Disabled);
    environment.spec.requirement = environment
        .spec
        .requirement
        .require_capabilities([
            IsolationCapability::NoNetworkGuarantee,
            IsolationCapability::ProcessStats,
            IsolationCapability::SecretIsolation,
        ])
        .require_locality()
        .require_secret_isolation()
        .allow_downgrade(
            [IsolationClass::Sandbox],
            [
                IsolationCapability::ProcessStats,
                IsolationCapability::SecretIsolation,
            ],
            [IsolationTrustField::SecretIsolation],
            [policy("policy.isolation.downgrade")],
        );
    let package = package_with_isolation(requirement_snapshot(&environment));
    let mut report = IsolationCapabilityReport::sandbox("runtime.fake.sandbox");
    report.capabilities = report
        .capabilities
        .without(IsolationCapability::ProcessStats)
        .without(IsolationCapability::SecretIsolation);
    report.trust = report.trust.best_effort_secret_isolation();
    let runtime = Arc::new(FakeIsolationRuntime::with_report(report));
    let mut registry = IsolationRuntimeRegistry::new();
    registry.register(runtime).expect("runtime registers");
    let journal = FakeJournalStore::default();
    let coordinator = IsolationLifecycleCoordinator::new(Arc::new(journal), registry);

    let missing_approval = coordinator
        .select_environment(
            &package,
            &environment,
            IsolationLifecycleContext::test(package.fingerprint().unwrap()),
            None,
        )
        .expect("missing approval returns denied outcome");
    assert_eq!(
        missing_approval.status,
        IsolationMatchStatus::DowngradeDenied
    );

    let tool_approval = coordinator
        .select_environment(
            &package,
            &environment,
            IsolationLifecycleContext::test(package.fingerprint().unwrap()),
            Some(IsolationDowngradeApproval::approved_for_tool(
                "decision.tool.run",
            )),
        )
        .expect("tool approval cannot approve isolation downgrade");
    assert_eq!(tool_approval.status, IsolationMatchStatus::DowngradeDenied);

    let approved = coordinator
        .select_environment(
            &package,
            &environment,
            IsolationLifecycleContext::test(package.fingerprint().unwrap()),
            Some(
                IsolationDowngradeApproval::approved_for_isolation("decision.isolation.downgrade")
                    .approve_capability(IsolationCapability::ProcessStats)
                    .approve_capability(IsolationCapability::SecretIsolation)
                    .approve_trust(IsolationTrustField::SecretIsolation),
            ),
        )
        .expect("explicit isolation downgrade approval is accepted");
    assert_eq!(approved.status, IsolationMatchStatus::DowngradeApproved);
    assert_eq!(
        downgrade_summary(&approved),
        read_fixture("tests/fixtures/isolation/downgrade-approved.json")
            .expect("downgrade fixture")
    );
}

#[test]
fn process_start_intent_is_journaled_before_adapter_call_and_io_is_redacted() {
    let environment = isolated_environment(NetworkIsolationPolicy::Disabled);
    let package = package_with_isolation(requirement_snapshot(&environment));
    let runtime = Arc::new(FakeIsolationRuntime::with_report(
        IsolationCapabilityReport::sandbox("runtime.fake.sandbox"),
    ));
    let mut registry = IsolationRuntimeRegistry::new();
    registry
        .register(runtime.clone())
        .expect("runtime registers");
    let journal = FakeJournalStore::default();
    let coordinator = IsolationLifecycleCoordinator::new(Arc::new(journal.clone()), registry);
    let process = environment
        .process(["cargo", "test"])
        .env_secret("API_TOKEN", "secret.ref.api_token")
        .build()
        .expect("process builds");

    let outcome = coordinator
        .start_process(
            &package,
            environment.clone(),
            process,
            IsolationLifecycleContext::test(package.fingerprint().unwrap()),
            None,
        )
        .expect("process starts");

    assert_eq!(outcome.status, IsolationMatchStatus::Matched);
    assert_eq!(runtime.start_process_call_count(), 1);
    assert_eq!(
        journal.records()[0].record_kind,
        agent_sdk_core::JournalRecordKind::EffectIntent,
        "intent append must happen before adapter start"
    );
    assert!(matches!(
        journal.records()[0].payload,
        JournalRecordPayload::EffectIntent(_)
    ));
    assert!(
        journal
            .records()
            .iter()
            .any(|record| matches!(record.payload, JournalRecordPayload::Isolation(_))),
        "isolation lifecycle records must be durable canonical payloads"
    );
    assert_eq!(
        lifecycle_summary(&journal.records()),
        read_fixture("tests/fixtures/isolation/lifecycle-intent-result.json")
            .expect("lifecycle fixture")
    );
    assert_eq!(
        process_io_summary(outcome.io_frames.as_slice()),
        read_fixture("tests/fixtures/isolation/process-io-redaction.json").expect("io fixture")
    );
}

#[test]
fn intent_append_failure_prevents_adapter_process_start() {
    let environment = isolated_environment(NetworkIsolationPolicy::Disabled);
    let package = package_with_isolation(requirement_snapshot(&environment));
    let runtime = Arc::new(FakeIsolationRuntime::with_report(
        IsolationCapabilityReport::sandbox("runtime.fake.sandbox"),
    ));
    let mut registry = IsolationRuntimeRegistry::new();
    registry
        .register(runtime.clone())
        .expect("runtime registers");
    let journal = FakeJournalStore::default();
    journal.fail_next_append("journal unavailable before process start");
    let coordinator = IsolationLifecycleCoordinator::new(Arc::new(journal), registry);
    let process = environment.process(["cargo", "test"]).build().unwrap();

    let error = coordinator
        .start_process(
            &package,
            environment,
            process,
            IsolationLifecycleContext::test(package.fingerprint().unwrap()),
            None,
        )
        .expect_err("journal failure fails closed");

    assert_eq!(error.kind(), AgentErrorKind::JournalFailure);
    assert_eq!(runtime.start_process_call_count(), 0);
}

#[test]
fn cleanup_failure_creates_repair_needed_record() {
    let environment = isolated_environment(NetworkIsolationPolicy::Disabled);
    let package = package_with_isolation(requirement_snapshot(&environment));
    let runtime = Arc::new(
        FakeIsolationRuntime::with_report(IsolationCapabilityReport::sandbox(
            "runtime.fake.cleanup",
        ))
        .with_cleanup_status(CleanupStatus::RepairNeeded),
    );
    let mut registry = IsolationRuntimeRegistry::new();
    registry.register(runtime).expect("runtime registers");
    let journal = FakeJournalStore::default();
    let coordinator = IsolationLifecycleCoordinator::new(Arc::new(journal.clone()), registry);

    let outcome = coordinator
        .cleanup_environment(
            environment.clone(),
            IsolationLifecycleContext::test(package.fingerprint().unwrap()),
        )
        .expect("cleanup failure is journaled as repair-needed");

    assert_eq!(outcome.status, CleanupStatus::RepairNeeded);
    assert_eq!(
        cleanup_summary(&journal.records()),
        read_fixture("tests/fixtures/isolation/cleanup-recovery.json").expect("cleanup fixture")
    );
}

#[derive(Clone, Debug)]
struct FailResultJournal {
    records: Arc<Mutex<Vec<JournalRecord>>>,
}

impl FailResultJournal {
    fn new() -> Self {
        Self {
            records: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl agent_sdk_core::RunJournal for FailResultJournal {
    fn append(&self, record: JournalRecord) -> Result<JournalCursor, AgentError> {
        if record.journal_seq == 4 {
            return Err(AgentError::new(
                AgentErrorKind::JournalFailure,
                agent_sdk_core::RetryClassification::RepairNeeded,
                "injected process result append failure",
            ));
        }
        let mut records = self.records.lock().expect("journal lock");
        records.push(record);
        Ok(JournalCursor::new(format!("journal.{}", records.len())))
    }
}

#[test]
fn result_append_failure_after_process_start_enters_recovery() {
    let environment = isolated_environment(NetworkIsolationPolicy::Disabled);
    let package = package_with_isolation(requirement_snapshot(&environment));
    let runtime = Arc::new(FakeIsolationRuntime::with_report(
        IsolationCapabilityReport::sandbox("runtime.fake.sandbox"),
    ));
    let mut registry = IsolationRuntimeRegistry::new();
    registry.register(runtime).expect("runtime registers");
    let journal = Arc::new(FailResultJournal::new());
    let coordinator = IsolationLifecycleCoordinator::new(journal.clone(), registry);
    let process = environment.process(["cargo", "test"]).build().unwrap();

    let outcome = coordinator
        .start_process(
            &package,
            environment,
            process,
            IsolationLifecycleContext::test(package.fingerprint().unwrap()),
            None,
        )
        .expect("recovery record is appended");

    assert!(outcome.recovery_required);
    assert!(matches!(
        journal
            .records
            .lock()
            .unwrap()
            .iter()
            .last()
            .expect("recovery record")
            .payload,
        JournalRecordPayload::Recovery(_)
    ));
}

fn isolated_environment(network: NetworkIsolationPolicy) -> ExecutionEnvironment {
    ExecutionEnvironment::require(
        IsolationRequirement::at_least(IsolationClass::Sandbox)
            .prefer("runtime.fake.sandbox")
            .require_capabilities([
                IsolationCapability::NoNetworkGuarantee,
                IsolationCapability::ReadOnlyRoot,
                IsolationCapability::Cleanup,
                IsolationCapability::ProcessTimeout,
                IsolationCapability::IoRedaction,
            ]),
    )
    .environment_id("env.isolation.contract")
    .workspace("workspace.primary", WorkspaceMountMode::Snapshot)
    .network(network)
    .secrets(SecretExposurePolicy::no_ambient())
    .ephemeral()
    .source(source("source.sdk.isolation"))
    .destination(DestinationRef::with_kind(
        DestinationKind::ExternalRuntime,
        "destination.isolation.runtime",
    ))
    .build()
    .expect("environment builds")
}

fn container_environment() -> ExecutionEnvironment {
    ExecutionEnvironment::require(
        IsolationRequirement::at_least(IsolationClass::Container)
            .prefer("runtime.fake.host")
            .require_capabilities([IsolationCapability::Cleanup]),
    )
    .environment_id("env.container.required")
    .filesystem(FilesystemIsolationPolicy::no_workspace())
    .network(NetworkIsolationPolicy::Disabled)
    .secrets(SecretExposurePolicy::no_ambient())
    .build()
    .expect("environment builds")
}

fn requirement_snapshot(environment: &ExecutionEnvironment) -> IsolationRequirementSnapshot {
    IsolationRequirementSnapshot::from_environment(
        RuntimePackageSidecarId::new("sidecar.isolation.contract"),
        environment,
        policy("policy.redaction.isolation"),
        policy("policy.cleanup.isolation"),
        policy("policy.child.isolation"),
    )
}

fn package_with_isolation(snapshot: IsolationRequirementSnapshot) -> RuntimePackage {
    RuntimePackage::builder(RuntimePackageId::new("package.isolation.contract"))
        .agent(agent_sdk_core::AgentSnapshot {
            agent_id: AgentId::new("agent.isolation.contract"),
            name: "isolation contract".to_string(),
            default_behavior_refs: Vec::new(),
        })
        .provider_route(agent_sdk_core::ProviderRouteSnapshot::new(
            "provider.fake",
            "model.fake",
        ))
        .isolation_requirement(snapshot)
        .build()
        .expect("package builds")
}

fn policy(id: &str) -> PolicyRef {
    PolicyRef::with_kind(PolicyKind::RuntimePackage, id)
}

fn source(id: &str) -> SourceRef {
    SourceRef::with_kind(SourceKind::Sdk, id)
}

fn sidecar_summary(environment: &ExecutionEnvironment, package: &RuntimePackage) -> Value {
    json!({
        "environment_id": environment.environment_id.as_str(),
        "minimum_class": format!("{:?}", environment.spec.requirement.minimum_class),
        "network": environment.spec.network.summary_key(),
        "workspace_mode": format!("{:?}", environment.spec.filesystem.workspace.mode),
        "sidecar_count": package.isolation_requirements.len(),
        "fingerprint_has_isolation": package.canonical_snapshot().unwrap().fingerprint_manifest.included_groups.iter().any(|group| format!("{:?}", group) == "IsolationRequirements"),
    })
}

fn downgrade_summary(outcome: &agent_sdk_core::IsolationSelectionOutcome) -> Value {
    let decision = outcome.downgrade_record.as_ref().expect("downgrade record");
    json!({
        "status": format!("{:?}", outcome.status),
        "requested_class": format!("{:?}", decision.requested_class),
        "selected_class": format!("{:?}", decision.selected_class),
        "capability_gaps": decision.capability_gaps.iter().map(|gap| format!("{:?}", gap)).collect::<Vec<_>>(),
        "trust_gaps": decision.trust_gaps.iter().map(|gap| format!("{:?}", gap)).collect::<Vec<_>>(),
        "policy_decision_scope": decision.policy_decision_scope.as_ref().map(|scope| format!("{:?}", scope)),
    })
}

fn lifecycle_summary(records: &[JournalRecord]) -> Value {
    let records = records
        .iter()
        .filter_map(|record| match &record.payload {
            JournalRecordPayload::EffectIntent(intent) => Some(json!({
                "journal_seq": record.journal_seq,
                "record_kind": "effect_intent",
                "effect_kind": format!("{:?}", intent.kind),
                "subject_kind": format!("{:?}", intent.subject_ref.kind),
                "redacted_summary": intent.redacted_summary,
            })),
            JournalRecordPayload::EffectResult(result) => Some(json!({
                "journal_seq": record.journal_seq,
                "record_kind": "effect_result",
                "terminal_status": format!("{:?}", result.terminal_status),
                "redacted_summary": result.redacted_summary,
            })),
            _ => None,
        })
        .collect::<Vec<_>>();
    json!({ "records": records })
}

fn process_io_summary(frames: &[agent_sdk_core::ProcessIoFrame]) -> Value {
    let frames = frames
        .iter()
        .map(|frame| {
            json!({
                "stream": format!("{:?}", frame.stream),
                "byte_count": frame.byte_count,
                "content_hash": frame.content_hash,
                "content_refs": frame.content_refs.iter().map(|content| content.as_str()).collect::<Vec<_>>(),
                "raw_content_present": frame.raw_content_present,
                "redacted_summary": frame.redacted_summary,
            })
        })
        .collect::<Vec<_>>();
    json!({ "frames": frames })
}

fn cleanup_summary(records: &[JournalRecord]) -> Value {
    let records = records
        .iter()
        .filter_map(|record| match &record.payload {
            JournalRecordPayload::EffectIntent(intent) => Some(json!({
                "journal_seq": record.journal_seq,
                "record_kind": "cleanup_intent",
                "effect_kind": format!("{:?}", intent.kind),
                "redacted_summary": intent.redacted_summary,
            })),
            JournalRecordPayload::EffectResult(result) => Some(json!({
                "journal_seq": record.journal_seq,
                "record_kind": "cleanup_result",
                "terminal_status": format!("{:?}", result.terminal_status),
                "redacted_summary": result.redacted_summary,
            })),
            _ => None,
        })
        .collect::<Vec<_>>();
    json!({ "records": records })
}
