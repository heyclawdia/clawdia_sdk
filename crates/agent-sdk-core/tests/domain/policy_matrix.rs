use agent_sdk_core::{
    AdapterRef, ApprovalDecisionKind, ApprovalPolicy, CapabilityPermission, ContentCaptureMode,
    ContentCapturePolicy, DestinationRef, DispatcherScope, EffectClass, EscalationPolicy,
    MissingDependency, PermissionPolicy, PolicyDecision, PolicyRef, PolicyStage, PrivacyClass,
    ResumePolicy, RetentionClass, RiskClass, SandboxMode, SandboxPolicy, SourceRef,
    policy::{PermissionGrant, PermissionRequest},
};

#[test]
fn finite_policy_matrix_preserves_fail_closed_defaults() {
    let source = SourceRef::new("source.policy.matrix");
    let destination = DestinationRef::new("destination.policy.matrix.tool");
    let permission = PermissionPolicy {
        policy_ref: PolicyRef::new("policy.matrix.permission"),
        grants: vec![PermissionGrant {
            source: source.clone(),
            capability: CapabilityPermission::FilesystemRead,
            decision: PolicyDecision::allow("permission.read.allowed"),
        }],
        default_decision: PolicyDecision::deny("permission.default_deny"),
    };

    let cases = [
        MatrixCase {
            name: "granted_read",
            stage: PolicyStage::PreTool,
            decision: permission
                .check(&permission_request(
                    source.clone(),
                    destination.clone(),
                    CapabilityPermission::FilesystemRead,
                ))
                .decision,
            allowed: true,
        },
        MatrixCase {
            name: "unknown_shell",
            stage: PolicyStage::PreTool,
            decision: permission
                .check(&permission_request(
                    source.clone(),
                    destination.clone(),
                    CapabilityPermission::Shell,
                ))
                .decision,
            allowed: false,
        },
        MatrixCase {
            name: "approval_required",
            stage: PolicyStage::PreTool,
            decision: ApprovalPolicy::ask_by_default(
                PolicyRef::new("policy.matrix.approval"),
                approval_request(source.clone(), destination.clone()),
            )
            .classify()
            .decision,
            allowed: false,
        },
        MatrixCase {
            name: "missing_dispatcher",
            stage: PolicyStage::PreTool,
            decision: EscalationPolicy {
                policy_ref: PolicyRef::new("policy.matrix.escalation"),
                dispatcher_required: true,
                timeout_ms: 30_000,
                allowed_decisions: vec![ApprovalDecisionKind::Approved],
            }
            .evaluate_dispatcher(false)
            .decision,
            allowed: false,
        },
        MatrixCase {
            name: "sandbox_adapter_missing",
            stage: PolicyStage::PreTool,
            decision: SandboxPolicy {
                policy_ref: PolicyRef::new("policy.matrix.sandbox"),
                mode: SandboxMode::RequireIsolation {
                    adapter_ref: AdapterRef::new("adapter.matrix.isolation"),
                },
                network: agent_sdk_core::policy::NetworkPolicy::Disabled,
                missing_adapter_decision: PolicyDecision::deny("sandbox.adapter_missing"),
            }
            .evaluate(false)
            .decision,
            allowed: false,
        },
        MatrixCase {
            name: "explicit_defer",
            stage: PolicyStage::Stream,
            decision: PolicyDecision::Defer {
                resume_policy: ResumePolicy {
                    resume_after_ms: Some(100),
                    reason: agent_sdk_core::DecisionReason::new("stream.backpressure.defer"),
                },
            },
            allowed: false,
        },
        MatrixCase {
            name: "explicit_interrupt",
            stage: PolicyStage::Stream,
            decision: PolicyDecision::interrupt("stream.rule.interrupt"),
            allowed: false,
        },
    ];

    for case in cases {
        assert_eq!(
            case.decision.is_allow(),
            case.allowed,
            "{} at {:?} had an unexpected allow/deny shape",
            case.name,
            case.stage
        );
        let encoded = serde_json::to_string(&case.decision).expect("policy matrix serializes");
        let decoded: PolicyDecision =
            serde_json::from_str(&encoded).expect("policy matrix decodes");
        assert_eq!(decoded, case.decision, "{} must round-trip", case.name);
    }
}

#[test]
fn missing_dependencies_and_content_capture_matrix_are_consumer_testable() {
    for dependency in [
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
    ] {
        let outcome = agent_sdk_core::PolicyOutcome::fail_closed(PolicyStage::PreTool, dependency);
        assert!(
            !outcome.is_allowed(),
            "{dependency:?} must be a deterministic denial"
        );
        assert!(
            dependency.reason_code().starts_with("missing."),
            "{dependency:?} must expose a stable conformance reason"
        );
    }

    let mut capture = ContentCapturePolicy::safe_defaults(PolicyRef::new("policy.matrix.capture"));
    for mode in [
        ContentCaptureMode::Off,
        ContentCaptureMode::MetadataOnly,
        ContentCaptureMode::RedactedSummary,
    ] {
        capture.mode = mode;
        capture.source_permits_content = true;
        capture.sink_permits_content = true;
        capture.byte_limit = 1024;
        assert!(
            !capture.allows_raw_content(),
            "non-raw capture mode must stay raw-content-free"
        );
    }

    capture.mode = ContentCaptureMode::RawContent;
    capture.source_permits_content = true;
    capture.sink_permits_content = true;
    capture.redaction_required = true;
    capture.retention_required = true;
    capture.sampling_required = true;
    capture.byte_limit = 1024;
    assert!(capture.allows_raw_content());
}

struct MatrixCase {
    name: &'static str,
    stage: PolicyStage,
    decision: PolicyDecision,
    allowed: bool,
}

fn permission_request(
    source: SourceRef,
    destination: DestinationRef,
    capability: CapabilityPermission,
) -> PermissionRequest {
    PermissionRequest {
        stage: PolicyStage::PreTool,
        source,
        destination,
        capability,
        subject: None,
        privacy: PrivacyClass::Internal,
        retention: RetentionClass::RunScoped,
    }
}

fn approval_request(
    source: SourceRef,
    destination: DestinationRef,
) -> agent_sdk_core::ApprovalRequestSpec {
    agent_sdk_core::ApprovalRequestSpec {
        source,
        destination,
        effect_class: EffectClass::Write,
        risk_class: RiskClass::High,
        dispatcher_scope: DispatcherScope::SourceScoped,
        timeout_ms: 30_000,
        allowed_decisions: vec![ApprovalDecisionKind::Approved, ApprovalDecisionKind::Denied],
        policy_refs: vec![PolicyRef::new("policy.matrix.approval")],
    }
}
