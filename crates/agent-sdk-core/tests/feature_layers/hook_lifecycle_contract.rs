use std::sync::Mutex;

use agent_sdk_core::{
    AgentError, AgentErrorKind, AgentId, JournalCursor, JournalRecord, PolicyRef,
    ProviderRouteSnapshot, RetryClassification, RunId, RunJournal, RuntimePackage,
    RuntimePackageId, SourceKind, SourceRef,
    effect::EffectKind,
    hook_ports::InMemoryHookExecutorRegistry,
    hook_records,
    hooks::{HookInvocationStatus, HookLifecycleContext, HookLifecycleCoordinator},
    package_hooks::{
        self, DenyReason, HookConfig, HookExecutionMode, HookFailurePolicy, HookId,
        HookMutationRight, HookMutationRights, HookOrdering, HookOverflowPolicy, HookPoint,
        HookPrivacyPolicy, HookQueueConfig, HookResponse, HookSource, HookSpec, HookTimeoutPolicy,
        lower_code_hook, ordered_hooks_for_point,
    },
    testing::ScriptedHookExecutor,
    testing::{FakeJournalStore, normalize_json_value, read_fixture},
};

fn hook_policy(id: &str) -> PolicyRef {
    package_hooks::hook_policy_ref(id)
}

fn source(id: &str) -> SourceRef {
    SourceRef::with_kind(SourceKind::Host, id)
}

fn fixture(path: &str) -> serde_json::Value {
    read_fixture(path).expect("fixture")
}

#[derive(Default)]
struct FailHookResultJournal {
    records: Mutex<Vec<JournalRecord>>,
}

impl FailHookResultJournal {
    fn records(&self) -> Vec<JournalRecord> {
        self.records.lock().expect("hook journal lock").clone()
    }
}

impl RunJournal for FailHookResultJournal {
    fn append(&self, record: JournalRecord) -> Result<JournalCursor, AgentError> {
        if record.record_id.ends_with(".result") {
            return Err(AgentError::new(
                AgentErrorKind::JournalFailure,
                RetryClassification::RepairNeeded,
                "injected hook result append failure",
            ));
        }
        let mut records = self.records.lock().expect("hook journal lock");
        records.push(record);
        Ok(JournalCursor::new(format!("journal.{}", records.len())))
    }
}

fn baseline_agent_id() -> AgentId {
    AgentId::new("agent.hook.contract")
}

fn package_with_hook(spec: &HookSpec) -> RuntimePackage {
    RuntimePackage::builder(RuntimePackageId::new("package.hook.contract"))
        .agent(agent_sdk_core::AgentSnapshot {
            agent_id: baseline_agent_id(),
            name: "hook contract".to_string(),
            default_behavior_refs: Vec::new(),
        })
        .provider_route(ProviderRouteSnapshot::new("provider.fake", "model.fake"))
        .sidecar(spec.sidecar_snapshot().expect("hook sidecar"))
        .build()
        .expect("package builds")
}

fn observe_spec(hook_id: &str) -> HookSpec {
    lower_code_hook(
        hook_id,
        HookPoint::BeforeContextAssembly,
        format!("executor.{hook_id}"),
        hook_policy("policy.hooks.observe"),
    )
    .expect("observe hook lowers")
}

fn before_tool_deny_spec() -> HookSpec {
    let mut spec = HookSpec::blocking(
        "audit.before_tool",
        HookPoint::BeforeToolCall,
        HookSource::InProcess,
        "executor.audit.before_tool",
        hook_policy("policy.hooks.before_tool"),
        HookMutationRights::from_rights([HookMutationRight::Deny]),
    );
    spec.failure = HookFailurePolicy::Deny;
    spec
}

fn lifecycle_context(package: &RuntimePackage) -> HookLifecycleContext {
    HookLifecycleContext::new(
        RunId::new("run.hook.contract"),
        baseline_agent_id(),
        source("source.hook.contract"),
        package.fingerprint().expect("package fingerprint"),
    )
}

#[test]
fn agent_on_hook_lowers_to_hook_spec_sidecar() {
    let spec = observe_spec("audit.before_context");
    let sidecar = spec.sidecar_snapshot().expect("sidecar");

    assert_eq!(sidecar.kind, "hook_spec");
    assert_eq!(sidecar.refs.len(), 1);
    assert_eq!(sidecar.refs[0].sidecar_id, "hook.audit.before_context");
    assert!(sidecar.content_hash.starts_with("sha256:"));
    assert_eq!(
        normalize_json_value(serde_json::to_value(sidecar).expect("sidecar JSON")),
        fixture("tests/fixtures/hooks/hook-sidecar.json")
    );
}

#[test]
fn config_hook_and_code_hook_share_runtime_package_shape() {
    let code = observe_spec("audit.before_context");
    let config = HookConfig {
        hook_id: HookId::new("audit.before_context"),
        point: HookPoint::BeforeContextAssembly,
        source: HookSource::InProcess,
        ordering: HookOrdering::normal(100),
        execution: HookExecutionMode::nonblocking_observe_default(),
        timeout: HookTimeoutPolicy::bounded_ms(250),
        failure: HookFailurePolicy::FailOpenObserveOnly,
        mutation_rights: HookMutationRights::observe_only(),
        privacy: HookPrivacyPolicy::EnvelopeAndRedactedSummary,
        policy_ref: hook_policy("policy.hooks.observe"),
        executor_ref: package_hooks::HookExecutorRef::new("executor.audit.before_context"),
    }
    .lower()
    .expect("config lowers");

    assert_eq!(code, config);
    assert_eq!(
        package_with_hook(&code).canonical_snapshot().unwrap(),
        package_with_hook(&config).canonical_snapshot().unwrap()
    );
}

#[test]
fn hook_helper_and_explicit_hook_spec_emit_equivalent_package_fingerprint() {
    let helper = observe_spec("audit.before_context");
    let explicit = HookSpec::observe(
        "audit.before_context",
        HookPoint::BeforeContextAssembly,
        HookSource::InProcess,
        "executor.audit.before_context",
        hook_policy("policy.hooks.observe"),
    );

    assert_eq!(
        package_with_hook(&helper).fingerprint().unwrap(),
        package_with_hook(&explicit).fingerprint().unwrap()
    );
}

#[test]
fn hook_execution_mode_and_queue_are_fingerprinted() {
    let baseline = observe_spec("audit.before_context");
    let mut changed = baseline.clone();
    changed.execution = HookExecutionMode::NonBlocking {
        queue: HookQueueConfig::new(128, 4),
        overflow: HookOverflowPolicy::DropObserveOnly,
    };

    assert_ne!(
        package_with_hook(&baseline).fingerprint().unwrap(),
        package_with_hook(&changed).fingerprint().unwrap()
    );
}

#[test]
fn hook_ordering_is_deterministic_by_point_phase_order_and_id() {
    let mut early_b = observe_spec("b");
    early_b.ordering = HookOrdering::early(0);
    let mut early_a = observe_spec("a");
    early_a.ordering = HookOrdering::early(0);
    let mut normal = observe_spec("normal");
    normal.ordering = HookOrdering::normal(0);
    let mut late = observe_spec("late");
    late.ordering = HookOrdering::late(-100);
    let mut other_point = observe_spec("other");
    other_point.point = HookPoint::AfterContextAssembly;

    let ordered = ordered_hooks_for_point(
        &[normal, late, early_b, other_point, early_a],
        HookPoint::BeforeContextAssembly,
    );

    assert_eq!(
        ordered
            .into_iter()
            .map(|spec| spec.hook_id.as_str().to_string())
            .collect::<Vec<_>>(),
        vec!["a", "b", "normal", "late"]
    );
}

#[test]
fn nonblocking_observe_hook_timeout_fails_open_with_record() {
    let spec = observe_spec("audit.before_context");
    let package = package_with_hook(&spec);
    let registry = InMemoryHookExecutorRegistry::new();
    registry
        .register(ScriptedHookExecutor::once(
            "executor.audit.before_context",
            HookResponse::ObserveOnly,
            spec.timeout.timeout_ms + 1,
        ))
        .expect("register executor");
    let journal = FakeJournalStore::default();
    let mut coordinator = HookLifecycleCoordinator::new(&registry, &journal, 1);

    let outcomes = coordinator
        .invoke_point(
            &[spec],
            HookPoint::BeforeContextAssembly,
            lifecycle_context(&package),
            package_hooks::HookView::redacted("context assembly envelope"),
        )
        .expect("observe timeout fails open");

    assert_eq!(outcomes[0].status, HookInvocationStatus::TimedOutFailOpen);
    assert!(journal.records().is_empty());
}

#[test]
fn security_hook_timeout_denies_or_interrupts_not_fail_open() {
    let spec = before_tool_deny_spec();
    let package = package_with_hook(&spec);
    let registry = InMemoryHookExecutorRegistry::new();
    registry
        .register(ScriptedHookExecutor::once(
            "executor.audit.before_tool",
            HookResponse::ObserveOnly,
            spec.timeout.timeout_ms + 1,
        ))
        .expect("register executor");
    let journal = FakeJournalStore::default();
    let mut coordinator = HookLifecycleCoordinator::new(&registry, &journal, 1);

    let error = coordinator
        .invoke_point(
            &[spec],
            HookPoint::BeforeToolCall,
            lifecycle_context(&package),
            package_hooks::HookView::redacted("before tool envelope"),
        )
        .expect_err("security timeout fails closed");

    assert_eq!(error.kind(), AgentErrorKind::PolicyDenial);
    assert!(journal.records().is_empty());
}

#[test]
fn nonblocking_security_relevant_hook_is_rejected_by_package_validation() {
    let mut spec = HookSpec::observe(
        "audit.bad",
        HookPoint::BeforeToolCall,
        HookSource::InProcess,
        "executor.audit.bad",
        hook_policy("policy.hooks.bad"),
    );
    spec.mutation_rights = HookMutationRights::from_rights([HookMutationRight::Deny]);
    spec.failure = HookFailurePolicy::Deny;

    let error = spec.validate().expect_err("invalid package hook");
    assert_eq!(error.kind(), AgentErrorKind::InvalidPackage);
}

#[test]
fn hook_helper_resolves_executor_ref_before_start_run() {
    let spec = observe_spec("audit.before_context");
    let registry = InMemoryHookExecutorRegistry::new();
    let journal = FakeJournalStore::default();
    let coordinator = HookLifecycleCoordinator::new(&registry, &journal, 1);

    let error = coordinator
        .validate_package_hooks(&[spec])
        .expect_err("missing executor fails package validation");

    assert_eq!(error.kind(), AgentErrorKind::InvalidPackage);
}

#[test]
fn hook_response_class_outside_mutation_rights_is_rejected() {
    let mut spec = before_tool_deny_spec();
    spec.mutation_rights = HookMutationRights::observe_only();
    let package = package_with_hook(&spec);
    let registry = InMemoryHookExecutorRegistry::new();
    registry
        .register(ScriptedHookExecutor::once(
            "executor.audit.before_tool",
            HookResponse::Deny(DenyReason {
                code: "tool.denied".to_string(),
                redacted_summary: "deny before tool".to_string(),
            }),
            1,
        ))
        .expect("register executor");
    let journal = FakeJournalStore::default();
    let mut coordinator = HookLifecycleCoordinator::new(&registry, &journal, 1);

    let outcomes = coordinator
        .invoke_point(
            &[spec],
            HookPoint::BeforeToolCall,
            lifecycle_context(&package),
            package_hooks::HookView::redacted("before tool envelope"),
        )
        .expect("response rejected, transition can decide next step");

    assert_eq!(
        outcomes[0].status,
        HookInvocationStatus::RejectedMutationRight
    );
    assert!(journal.records().is_empty());
}

#[test]
fn hook_mutation_rights_matrix_matches_allowed_response_table() {
    assert!(!HookMutationRights::deny_or_request_approval().is_observe_only());

    let before_tool = HookPoint::BeforeToolCall.allowed_response_classes();
    assert!(before_tool.contains(&package_hooks::HookResponseClass::Deny));
    assert!(before_tool.contains(&package_hooks::HookResponseClass::ModifyToolRequest));
    assert!(!before_tool.contains(&package_hooks::HookResponseClass::RewriteToolResult));

    let after_terminal = HookPoint::AfterRunTerminal.allowed_response_classes();
    assert_eq!(
        after_terminal,
        [package_hooks::HookResponseClass::Observe]
            .into_iter()
            .collect()
    );
}

#[test]
fn before_tool_hook_can_deny_before_executor_start_with_journal_backing() {
    let spec = before_tool_deny_spec();
    let package = package_with_hook(&spec);
    let registry = InMemoryHookExecutorRegistry::new();
    registry
        .register(ScriptedHookExecutor::once(
            "executor.audit.before_tool",
            HookResponse::Deny(DenyReason {
                code: "tool.denied".to_string(),
                redacted_summary: "deny before tool".to_string(),
            }),
            1,
        ))
        .expect("register executor");
    let journal = FakeJournalStore::default();
    let mut coordinator = HookLifecycleCoordinator::new(&registry, &journal, 1);

    let outcomes = coordinator
        .invoke_point(
            &[spec],
            HookPoint::BeforeToolCall,
            lifecycle_context(&package),
            package_hooks::HookView::redacted("before tool envelope"),
        )
        .expect("deny applies");

    assert_eq!(
        outcomes[0].status,
        HookInvocationStatus::AppliedJournaledMutation
    );
    assert!(outcomes[0].journaled_before_apply);
    let records = journal.records();
    assert_eq!(records.len(), 3);
    assert!(matches!(
        records[0].payload,
        agent_sdk_core::JournalRecordPayload::Hook(_)
    ));
    match &records[1].payload {
        agent_sdk_core::JournalRecordPayload::EffectIntent(intent) => {
            assert_eq!(intent.kind, EffectKind::HookMutation);
            assert_eq!(
                intent.policy_refs,
                vec![hook_policy("policy.hooks.before_tool")]
            );
        }
        other => panic!("expected hook mutation effect intent, got {other:?}"),
    }
    assert!(matches!(
        records[2].payload,
        agent_sdk_core::JournalRecordPayload::EffectResult(_)
    ));
    assert_eq!(
        normalize_json_value(serde_json::to_value(&records[1].payload).expect("payload JSON")),
        fixture("tests/fixtures/hooks/hook-mutation-effect-intent.json")
    );
}

#[test]
fn hook_response_apply_record_is_journal_backed_after_records() {
    let spec = before_tool_deny_spec();
    let registered = hook_records::HookRecord::registered(&spec).expect("registered record");
    let started = hook_records::HookRecord::invocation_started(&spec, "hook.invocation.1");
    assert_eq!(registered.hook_id, spec.hook_id);
    assert_eq!(started.point, HookPoint::BeforeToolCall);

    let record = hook_records::HookRecord::response_decision(
        &spec,
        "hook.invocation.1",
        hook_records::HookResponseDecision::AcceptedJournaledBeforeApply,
        package_hooks::HookResponseClass::Deny,
        vec![hook_records::hook_entity_ref(&spec.hook_id)],
    );

    assert_eq!(
        normalize_json_value(serde_json::to_value(record).expect("hook record JSON")),
        fixture("tests/fixtures/hooks/hook-response-accepted-record.json")
    );
}

#[test]
fn hook_response_mutation_append_failure_prevents_apply() {
    let spec = before_tool_deny_spec();
    let package = package_with_hook(&spec);
    let registry = InMemoryHookExecutorRegistry::new();
    registry
        .register(ScriptedHookExecutor::once(
            "executor.audit.before_tool",
            HookResponse::Deny(DenyReason {
                code: "tool.denied".to_string(),
                redacted_summary: "deny before tool".to_string(),
            }),
            1,
        ))
        .expect("register executor");
    let journal = FakeJournalStore::default();
    journal.fail_next_append("disk unavailable");
    let mut coordinator = HookLifecycleCoordinator::new(&registry, &journal, 1);

    let error = coordinator
        .invoke_point(
            &[spec],
            HookPoint::BeforeToolCall,
            lifecycle_context(&package),
            package_hooks::HookView::redacted("before tool envelope"),
        )
        .expect_err("append failure prevents apply");

    assert_eq!(error.kind(), AgentErrorKind::PolicyDenial);
    assert!(journal.records().is_empty());
}

#[test]
fn hook_response_terminal_append_failure_prevents_apply() {
    let spec = before_tool_deny_spec();
    let package = package_with_hook(&spec);
    let registry = InMemoryHookExecutorRegistry::new();
    registry
        .register(ScriptedHookExecutor::once(
            "executor.audit.before_tool",
            HookResponse::Deny(DenyReason {
                code: "tool.denied".to_string(),
                redacted_summary: "deny before tool".to_string(),
            }),
            1,
        ))
        .expect("register executor");
    let journal = FailHookResultJournal::default();
    let mut coordinator = HookLifecycleCoordinator::new(&registry, &journal, 1);

    let error = coordinator
        .invoke_point(
            &[spec],
            HookPoint::BeforeToolCall,
            lifecycle_context(&package),
            package_hooks::HookView::redacted("before tool envelope"),
        )
        .expect_err("terminal append failure prevents apply");

    assert_eq!(error.kind(), AgentErrorKind::PolicyDenial);
    assert_eq!(journal.records().len(), 2);
    assert!(matches!(
        journal.records()[0].payload,
        agent_sdk_core::JournalRecordPayload::Hook(_)
    ));
    assert!(matches!(
        journal.records()[1].payload,
        agent_sdk_core::JournalRecordPayload::EffectIntent(_)
    ));
}

#[test]
fn cancel_interrupts_inflight_hooks_and_continues_child_shutdown() {
    let spec = observe_spec("audit.before_context");
    let package = package_with_hook(&spec);
    let executor = ScriptedHookExecutor::once(
        "executor.audit.before_context",
        HookResponse::ObserveOnly,
        1,
    );
    let registry = InMemoryHookExecutorRegistry::new();
    registry
        .register(executor.clone())
        .expect("register executor");
    let journal = FakeJournalStore::default();
    let mut context = lifecycle_context(&package);
    context.cancellation = package_hooks::HookCancellationToken::cancelled();
    let mut coordinator = HookLifecycleCoordinator::new(&registry, &journal, 1);

    let outcomes = coordinator
        .invoke_point(
            &[spec],
            HookPoint::BeforeContextAssembly,
            context,
            package_hooks::HookView::redacted("context assembly envelope"),
        )
        .expect("cancel records hook cancellation");

    assert_eq!(outcomes[0].status, HookInvocationStatus::Cancelled);
    assert!(executor.invocations().is_empty());
}

#[test]
fn hook_response_lowering_matrix_has_no_generic_effect_hatch() {
    let application_source = include_str!("../../src/application/hooks.rs");
    let record_source = include_str!("../../src/records/hooks.rs");

    assert!(record_source.contains("EffectKind::HookMutation"));
    assert!(!application_source.contains("GenericSideEffect"));
    assert!(!application_source.contains("active_run_callback"));
    assert!(!application_source.contains("SideEffectQueue"));
}
