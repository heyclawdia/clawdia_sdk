use agent_sdk_core::{
    AdapterRef, AgentError, AgentErrorKind, ApprovalPolicy, CapabilityPermission, CausalIds,
    ContentCapturePolicy, DestinationRef, EscalationPolicy, MissingDependency, PermissionPolicy,
    PolicyDecision, PolicyRef, PolicyStage, PrivacyClass, RetryClassification, RunId,
    SandboxPolicy, SourceRef, ToolCallId,
};

#[test]
fn agent_error_preserves_typed_context_and_causal_ids() {
    let error = AgentError::new(
        AgentErrorKind::PolicyDenial,
        RetryClassification::UserActionNeeded,
        "approval denied before tool execution",
    )
    .with_policy_ref(PolicyRef::new("policy.approval.write"))
    .with_source(SourceRef::new("source.remote.channel"))
    .with_destination(DestinationRef::new("destination.tool.workspace_write"))
    .with_causal_ids(CausalIds {
        run_id: Some(RunId::new("run.policy.1")),
        tool_call_id: Some(ToolCallId::new("tool.call.1")),
        ..CausalIds::default()
    });

    assert_eq!(error.kind(), AgentErrorKind::PolicyDenial);
    assert_eq!(error.retry(), RetryClassification::UserActionNeeded);
    let context = error.context();
    assert_eq!(context.policy_refs[0].as_str(), "policy.approval.write");
    let causal_ids = error.causal_ids();
    assert_eq!(causal_ids.run_id.unwrap().as_str(), "run.policy.1");
    assert_eq!(causal_ids.tool_call_id.unwrap().as_str(), "tool.call.1");
}

#[test]
fn policy_decision_is_finite_and_serializes_stably() {
    let decisions = [
        PolicyDecision::allow("matrix.allow"),
        PolicyDecision::deny("matrix.deny"),
        PolicyDecision::Ask {
            approval: fixture_approval_request(),
        },
        PolicyDecision::Modify {
            modification: agent_sdk_core::policy::ToolRequestModification {
                redacted_summary: "drop unsafe argument".to_string(),
                policy_refs: vec![PolicyRef::new("policy.modify")],
            },
        },
        PolicyDecision::Defer {
            resume_policy: agent_sdk_core::policy::ResumePolicy {
                resume_after_ms: Some(100),
                reason: agent_sdk_core::policy::DecisionReason::new("matrix.defer"),
            },
        },
        PolicyDecision::interrupt("matrix.interrupt"),
    ];

    for decision in decisions {
        let encoded = serde_json::to_string(&decision).expect("decision serializes");
        let decoded: PolicyDecision = serde_json::from_str(&encoded).expect("decision decodes");
        assert_eq!(decoded, decision);
    }
}

#[test]
fn safe_policy_decision_fixture_matches_contract_shape() {
    let decision = PolicyDecision::deny("missing.approval_dispatcher");
    let encoded = serde_json::to_string_pretty(&decision).expect("decision fixture serializes");
    let expected = include_str!("../fixtures/policy/missing_dispatcher_decision.json").trim();

    assert_eq!(encoded, expected);
}

#[test]
fn missing_dependencies_fail_closed_before_dispatch_or_execution() {
    let cases = [
        MissingDependency::PolicySnapshot,
        MissingDependency::PolicyRef,
        MissingDependency::PermissionEvaluator,
        MissingDependency::SandboxEvaluator,
        MissingDependency::ApprovalDispatcher,
        MissingDependency::Adapter,
        MissingDependency::Sink,
        MissingDependency::Store,
        MissingDependency::JournalAppend,
        MissingDependency::ExecutorRef,
        MissingDependency::IsolationRuntime,
    ];

    for dependency in cases {
        let outcome = agent_sdk_core::PolicyOutcome::fail_closed(PolicyStage::PreTool, dependency);

        assert!(
            !outcome.is_allowed(),
            "{dependency:?} must not default to allow"
        );

        let error = dependency.to_error(CausalIds::default());
        assert_ne!(error.retry(), RetryClassification::Retryable);
    }
}

#[test]
fn permission_policy_denies_unknown_capability_by_default() {
    let policy = PermissionPolicy::deny_all(PolicyRef::new("policy.permission.default"));
    let request = agent_sdk_core::policy::PermissionRequest {
        stage: PolicyStage::PreTool,
        source: SourceRef::new("source.extension.example"),
        destination: DestinationRef::new("destination.tool.shell"),
        capability: CapabilityPermission::Shell,
        subject: None,
        privacy: PrivacyClass::Internal,
        retention: agent_sdk_core::domain::RetentionClass::RunScoped,
    };

    let outcome = policy.check(&request);

    assert!(!outcome.is_allowed());
    assert_eq!(outcome.policy_refs[0].as_str(), "policy.permission.default");
}

#[test]
fn sandbox_policy_requires_adapter_without_host_downgrade() {
    let policy = SandboxPolicy {
        policy_ref: PolicyRef::new("policy.sandbox.container"),
        mode: agent_sdk_core::policy::SandboxMode::RequireIsolation {
            adapter_ref: AdapterRef::new("adapter.container"),
        },
        network: agent_sdk_core::policy::NetworkPolicy::Disabled,
        missing_adapter_decision: PolicyDecision::deny("sandbox.adapter_missing"),
    };

    assert!(!policy.evaluate(false).is_allowed());
    assert!(policy.evaluate(true).is_allowed());
}

#[test]
fn approval_escalation_denies_when_dispatcher_is_required_but_missing() {
    let approval = ApprovalPolicy::ask_by_default(
        PolicyRef::new("policy.approval.write"),
        fixture_approval_request(),
    );
    let escalation = EscalationPolicy {
        policy_ref: PolicyRef::new("policy.escalation.source_scoped"),
        dispatcher_required: true,
        timeout_ms: 120_000,
        allowed_decisions: vec![agent_sdk_core::policy::ApprovalDecisionKind::Approved],
    };

    assert!(matches!(
        approval.classify().decision,
        PolicyDecision::Ask { .. }
    ));
    assert!(!escalation.evaluate_dispatcher(false).is_allowed());
    assert!(escalation.evaluate_dispatcher(true).is_allowed());
}

#[test]
fn content_capture_safe_defaults_do_not_capture_raw_content() {
    let mut policy = ContentCapturePolicy::safe_defaults(PolicyRef::new("policy.telemetry.safe"));

    assert!(!policy.allows_raw_content());

    for mode in [
        agent_sdk_core::policy::ContentCaptureMode::Off,
        agent_sdk_core::policy::ContentCaptureMode::MetadataOnly,
        agent_sdk_core::policy::ContentCaptureMode::RedactedSummary,
    ] {
        policy.mode = mode.clone();
        policy.source_permits_content = true;
        policy.sink_permits_content = true;
        policy.byte_limit = 1024;
        assert!(
            !policy.allows_raw_content(),
            "{mode:?} must not open the raw content gate"
        );
    }

    policy.mode = agent_sdk_core::policy::ContentCaptureMode::RawContent;
    policy.source_permits_content = true;
    policy.sink_permits_content = true;
    policy.byte_limit = 1024;

    assert!(
        policy.allows_raw_content(),
        "raw capture only opens after every explicit policy gate passes"
    );
}

#[test]
fn policy_surface_supports_deterministic_consumer_conformance_cases() {
    let cases = [
        ("deny", PolicyDecision::deny("conformance.denied")),
        ("timeout", PolicyDecision::deny("approval.timeout")),
        ("missing_sink", PolicyDecision::deny("missing.sink")),
        (
            "missing_journal_append",
            PolicyDecision::deny("missing.journal_append"),
        ),
    ];

    for (name, decision) in cases {
        let encoded = serde_json::to_string(&decision).expect("conformance case serializes");
        let decoded: PolicyDecision =
            serde_json::from_str(&encoded).expect("conformance case decodes");

        assert_eq!(decoded, decision, "{name} case must be deterministic");
        assert!(!decoded.is_allow(), "{name} case must fail closed");
    }

    let capture_denial =
        ContentCapturePolicy::safe_defaults(PolicyRef::new("policy.conformance.content_capture"));
    assert!(!capture_denial.allows_raw_content());
}

fn fixture_approval_request() -> agent_sdk_core::policy::ApprovalRequestSpec {
    agent_sdk_core::policy::ApprovalRequestSpec {
        source: SourceRef::new("source.remote.channel"),
        destination: DestinationRef::new("destination.tool.workspace_write"),
        effect_class: agent_sdk_core::policy::EffectClass::Write,
        risk_class: agent_sdk_core::policy::RiskClass::High,
        dispatcher_scope: agent_sdk_core::policy::DispatcherScope::SourceScoped,
        timeout_ms: 120_000,
        allowed_decisions: vec![
            agent_sdk_core::policy::ApprovalDecisionKind::Approved,
            agent_sdk_core::policy::ApprovalDecisionKind::Denied,
        ],
        policy_refs: vec![PolicyRef::new("policy.approval.write")],
    }
}
