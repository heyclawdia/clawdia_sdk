use crate::domain::{
    AdapterRef, DestinationRef, EntityRef, PolicyRef, PrivacyClass, RetentionClass, SourceRef,
    TrustClass,
};
use crate::error::{AgentError, AgentErrorKind, CausalIds, RetryClassification};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PolicyDecision {
    Allow {
        reason: DecisionReason,
    },
    Deny {
        reason: DecisionReason,
    },
    Ask {
        approval: ApprovalRequestSpec,
    },
    Modify {
        modification: ToolRequestModification,
    },
    Defer {
        resume_policy: ResumePolicy,
    },
    Interrupt {
        reason: DecisionReason,
    },
}

impl PolicyDecision {
    pub fn allow(code: impl Into<String>) -> Self {
        Self::Allow {
            reason: DecisionReason::new(code),
        }
    }

    pub fn deny(code: impl Into<String>) -> Self {
        Self::Deny {
            reason: DecisionReason::new(code),
        }
    }

    pub fn interrupt(code: impl Into<String>) -> Self {
        Self::Interrupt {
            reason: DecisionReason::new(code),
        }
    }

    pub fn is_allow(&self) -> bool {
        matches!(self, Self::Allow { .. })
    }
}

impl Default for PolicyDecision {
    fn default() -> Self {
        Self::deny("policy.default_deny")
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PolicyOutcome {
    pub stage: PolicyStage,
    pub decision: PolicyDecision,
    pub subject: Option<EntityRef>,
    pub source: Option<SourceRef>,
    pub destination: Option<DestinationRef>,
    pub policy_refs: Vec<PolicyRef>,
    pub privacy: PrivacyClass,
    pub retention: RetentionClass,
}

impl PolicyOutcome {
    pub fn denied(stage: PolicyStage, code: impl Into<String>) -> Self {
        Self {
            stage,
            decision: PolicyDecision::deny(code),
            subject: None,
            source: None,
            destination: None,
            policy_refs: Vec::new(),
            privacy: PrivacyClass::Internal,
            retention: RetentionClass::RunScoped,
        }
    }

    pub fn fail_closed(stage: PolicyStage, dependency: MissingDependency) -> Self {
        Self::denied(stage, dependency.reason_code())
    }

    pub fn is_allowed(&self) -> bool {
        self.decision.is_allow()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DecisionReason {
    pub code: String,
    pub redacted_summary: Option<String>,
    pub policy_refs: Vec<PolicyRef>,
}

impl DecisionReason {
    pub fn new(code: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            redacted_summary: None,
            policy_refs: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ApprovalRequestSpec {
    pub source: SourceRef,
    pub destination: DestinationRef,
    pub effect_class: EffectClass,
    pub risk_class: RiskClass,
    pub dispatcher_scope: DispatcherScope,
    pub timeout_ms: u64,
    pub allowed_decisions: Vec<ApprovalDecisionKind>,
    pub policy_refs: Vec<PolicyRef>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ToolRequestModification {
    pub redacted_summary: String,
    pub policy_refs: Vec<PolicyRef>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResumePolicy {
    pub resume_after_ms: Option<u64>,
    pub reason: DecisionReason,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PolicyStage {
    Input,
    ModelInputProjection,
    PreTool,
    PostTool,
    Output,
    Handoff,
    Stream,
    Delivery,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum MissingDependency {
    PolicySnapshot,
    PolicyRef,
    PermissionEvaluator,
    SandboxEvaluator,
    ApprovalDispatcher,
    Adapter,
    Sink,
    Store,
    JournalAppend,
    ExecutorRef,
    IsolationRuntime,
}

impl MissingDependency {
    pub fn reason_code(self) -> &'static str {
        match self {
            Self::PolicySnapshot => "missing.policy_snapshot",
            Self::PolicyRef => "missing.policy_ref",
            Self::PermissionEvaluator => "missing.permission_evaluator",
            Self::SandboxEvaluator => "missing.sandbox_evaluator",
            Self::ApprovalDispatcher => "missing.approval_dispatcher",
            Self::Adapter => "missing.adapter",
            Self::Sink => "missing.sink",
            Self::Store => "missing.store",
            Self::JournalAppend => "missing.journal_append",
            Self::ExecutorRef => "missing.executor_ref",
            Self::IsolationRuntime => "missing.isolation_runtime",
        }
    }

    pub fn to_error(self, causal_ids: CausalIds) -> AgentError {
        let kind = match self {
            Self::JournalAppend => AgentErrorKind::JournalFailure,
            Self::Adapter | Self::IsolationRuntime => AgentErrorKind::IsolationFailure,
            Self::ApprovalDispatcher => AgentErrorKind::ApprovalFailure,
            _ => AgentErrorKind::PolicyDenial,
        };

        AgentError::new(
            kind,
            RetryClassification::HostConfigurationNeeded,
            self.reason_code(),
        )
        .with_causal_ids(causal_ids)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PermissionPolicy {
    pub policy_ref: PolicyRef,
    pub grants: Vec<PermissionGrant>,
    pub default_decision: PolicyDecision,
}

impl PermissionPolicy {
    pub fn deny_all(policy_ref: PolicyRef) -> Self {
        Self {
            policy_ref,
            grants: Vec::new(),
            default_decision: PolicyDecision::deny("permission.default_deny"),
        }
    }

    pub fn check(&self, request: &PermissionRequest) -> PolicyOutcome {
        let grant = self.grants.iter().find(|grant| grant.matches(request));
        let decision = grant
            .map(|grant| grant.decision.clone())
            .unwrap_or_else(|| self.default_decision.clone());

        PolicyOutcome {
            stage: request.stage,
            decision,
            subject: request.subject.clone(),
            source: Some(request.source.clone()),
            destination: Some(request.destination.clone()),
            policy_refs: vec![self.policy_ref.clone()],
            privacy: request.privacy.clone(),
            retention: request.retention.clone(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PermissionGrant {
    pub source: SourceRef,
    pub capability: CapabilityPermission,
    pub decision: PolicyDecision,
}

impl PermissionGrant {
    fn matches(&self, request: &PermissionRequest) -> bool {
        self.source == request.source && self.capability == request.capability
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PermissionRequest {
    pub stage: PolicyStage,
    pub source: SourceRef,
    pub destination: DestinationRef,
    pub capability: CapabilityPermission,
    pub subject: Option<EntityRef>,
    pub privacy: PrivacyClass,
    pub retention: RetentionClass,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CapabilityPermission {
    FilesystemRead,
    FilesystemWrite,
    Network,
    Shell,
    Mcp,
    Media,
    Contacts,
    MemoryRead,
    MemoryWrite,
    OutputDelivery,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SandboxPolicy {
    pub policy_ref: PolicyRef,
    pub mode: SandboxMode,
    pub network: NetworkPolicy,
    pub missing_adapter_decision: PolicyDecision,
}

impl SandboxPolicy {
    pub fn host_denied(policy_ref: PolicyRef) -> Self {
        Self {
            policy_ref,
            mode: SandboxMode::DenyHostExecution,
            network: NetworkPolicy::Disabled,
            missing_adapter_decision: PolicyDecision::deny("sandbox.missing_adapter"),
        }
    }

    pub fn evaluate(&self, adapter_available: bool) -> PolicyOutcome {
        let decision = match (&self.mode, adapter_available) {
            (SandboxMode::DenyHostExecution, _) => PolicyDecision::deny("sandbox.host_denied"),
            (SandboxMode::RequireIsolation { .. }, false) => self.missing_adapter_decision.clone(),
            (SandboxMode::RequireIsolation { .. }, true) | (SandboxMode::AllowHostExecution, _) => {
                PolicyDecision::allow("sandbox.allowed")
            }
        };

        PolicyOutcome {
            stage: PolicyStage::PreTool,
            decision,
            subject: None,
            source: None,
            destination: None,
            policy_refs: vec![self.policy_ref.clone()],
            privacy: PrivacyClass::Internal,
            retention: RetentionClass::RunScoped,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SandboxMode {
    DenyHostExecution,
    RequireIsolation { adapter_ref: AdapterRef },
    AllowHostExecution,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum NetworkPolicy {
    Disabled,
    EgressAllowlist(Vec<String>),
    Enabled,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ApprovalPolicy {
    pub policy_ref: PolicyRef,
    pub default_decision: PolicyDecision,
}

impl ApprovalPolicy {
    pub fn ask_by_default(policy_ref: PolicyRef, approval: ApprovalRequestSpec) -> Self {
        Self {
            policy_ref,
            default_decision: PolicyDecision::Ask { approval },
        }
    }

    pub fn classify(&self) -> PolicyOutcome {
        PolicyOutcome {
            stage: PolicyStage::PreTool,
            decision: self.default_decision.clone(),
            subject: None,
            source: None,
            destination: None,
            policy_refs: vec![self.policy_ref.clone()],
            privacy: PrivacyClass::Internal,
            retention: RetentionClass::RunScoped,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EscalationPolicy {
    pub policy_ref: PolicyRef,
    pub dispatcher_required: bool,
    pub timeout_ms: u64,
    pub allowed_decisions: Vec<ApprovalDecisionKind>,
}

impl EscalationPolicy {
    pub fn evaluate_dispatcher(&self, dispatcher_available: bool) -> PolicyOutcome {
        let decision = if self.dispatcher_required && !dispatcher_available {
            PolicyDecision::deny("escalation.missing_dispatcher")
        } else {
            PolicyDecision::allow("escalation.dispatcher_ready")
        };

        PolicyOutcome {
            stage: PolicyStage::PreTool,
            decision,
            subject: None,
            source: None,
            destination: None,
            policy_refs: vec![self.policy_ref.clone()],
            privacy: PrivacyClass::Internal,
            retention: RetentionClass::RunScoped,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PrivacyPolicy {
    pub policy_ref: PolicyRef,
    pub minimum_privacy: PrivacyClass,
    pub retention: RetentionClass,
    pub trust_required: TrustClass,
}

impl PrivacyPolicy {
    pub fn safe_defaults(policy_ref: PolicyRef) -> Self {
        Self {
            policy_ref,
            minimum_privacy: PrivacyClass::Internal,
            retention: RetentionClass::RunScoped,
            trust_required: TrustClass::Trusted,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ContentCapturePolicy {
    pub policy_ref: PolicyRef,
    pub mode: ContentCaptureMode,
    pub source_permits_content: bool,
    pub sink_permits_content: bool,
    pub redaction_required: bool,
    pub retention_required: bool,
    pub sampling_required: bool,
    pub byte_limit: u64,
}

impl ContentCapturePolicy {
    pub fn safe_defaults(policy_ref: PolicyRef) -> Self {
        Self {
            policy_ref,
            mode: ContentCaptureMode::Off,
            source_permits_content: false,
            sink_permits_content: false,
            redaction_required: true,
            retention_required: true,
            sampling_required: true,
            byte_limit: 0,
        }
    }

    pub fn allows_raw_content(&self) -> bool {
        matches!(self.mode, ContentCaptureMode::RawContent)
            && self.source_permits_content
            && self.sink_permits_content
            && self.redaction_required
            && self.retention_required
            && self.sampling_required
            && self.byte_limit > 0
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ContentCaptureMode {
    Off,
    MetadataOnly,
    RedactedSummary,
    RawContent,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ApprovalDecisionKind {
    Approved,
    ApprovedForSession,
    Denied,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DispatcherScope {
    Host,
    SourceScoped,
    Headless,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum EffectClass {
    Read,
    Write,
    Process,
    Network,
    ApprovalDispatch,
    OutputDelivery,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RiskClass {
    Low,
    Medium,
    High,
    Critical,
}
