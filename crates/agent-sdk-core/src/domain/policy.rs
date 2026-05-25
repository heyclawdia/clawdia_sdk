//! Domain primitives for stable SDK vocabulary. Use these items for IDs, refs,
//! policy, privacy, trust, and errors that cross crate or host boundaries. They are
//! data-only and must not perform provider, filesystem, network, or UI side effects.
//! This file contains the policy portion of that contract.
//!
use crate::domain::{
    AdapterRef, DestinationRef, EntityRef, PolicyRef, PrivacyClass, RetentionClass, SourceRef,
    TrustClass,
};
use crate::error::{AgentError, AgentErrorKind, CausalIds, RetryClassification};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Enumerates the finite policy decision cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum PolicyDecision {
    /// Use this variant when the contract needs to represent allow; selecting it has no side effect by itself.
    Allow {
        /// Redacted explanation for a denial, failure, status, or package
        /// delta.
        reason: DecisionReason,
    },
    /// Use this variant when the contract needs to represent deny; selecting it has no side effect by itself.
    Deny {
        /// Redacted explanation for a denial, failure, status, or package
        /// delta.
        reason: DecisionReason,
    },
    /// Use this variant when the contract needs to represent ask; selecting it has no side effect by itself.
    Ask {
        /// Approval used by this record or request.
        approval: ApprovalRequestSpec,
    },
    /// Use this variant when the contract needs to represent modify; selecting it has no side effect by itself.
    Modify {
        /// Modification used by this record or request.
        modification: ToolRequestModification,
    },
    /// Use this variant when the contract needs to represent defer; selecting it has no side effect by itself.
    Defer {
        /// Resume policy used by this record or request.
        resume_policy: ResumePolicy,
    },
    /// Use this variant when the contract needs to represent interrupt; selecting it has no side effect by itself.
    Interrupt {
        /// Redacted explanation for a denial, failure, status, or package
        /// delta.
        reason: DecisionReason,
    },
}

impl PolicyDecision {
    /// Builds the allow value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn allow(code: impl Into<String>) -> Self {
        Self::Allow {
            reason: DecisionReason::new(code),
        }
    }

    /// Returns an updated domain::policy value with deny applied. This is
    /// data construction only and does not execute the configured behavior.
    pub fn deny(code: impl Into<String>) -> Self {
        Self::Deny {
            reason: DecisionReason::new(code),
        }
    }

    /// Interrupt.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn interrupt(code: impl Into<String>) -> Self {
        Self::Interrupt {
            reason: DecisionReason::new(code),
        }
    }

    /// Reports whether this value is allow. The check is pure and does
    /// not mutate SDK or host state.
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
/// Defines the policy outcome SDK value.
/// Construction records local state only; documented runtimes, executors, or ports own side effects.
pub struct PolicyOutcome {
    /// Stage used by this record or request.
    pub stage: PolicyStage,
    /// Decision used by this record or request.
    pub decision: PolicyDecision,
    /// Optional subject value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub subject: Option<EntityRef>,
    /// Source label or ref for this item; it is metadata and does not fetch
    /// content by itself.
    pub source: Option<SourceRef>,
    /// Destination label or ref for this item; it is metadata and does not
    /// deliver content by itself.
    pub destination: Option<DestinationRef>,
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy: PrivacyClass,
    /// Retention class used by hosts and sinks when storing or exporting this
    /// item.
    pub retention: RetentionClass,
}

impl PolicyOutcome {
    /// Builds the denied record or result value.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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

    /// Builds the fail closed value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn fail_closed(stage: PolicyStage, dependency: MissingDependency) -> Self {
        Self::denied(stage, dependency.reason_code())
    }

    /// Reports whether this value is allowed. The check is pure and
    /// does not mutate SDK or host state.
    pub fn is_allowed(&self) -> bool {
        self.decision.is_allow()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Defines the decision reason SDK value.
/// Construction records local state only; documented runtimes, executors, or ports own side effects.
pub struct DecisionReason {
    /// Code used by this record or request.
    pub code: String,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: Option<String>,
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
}

impl DecisionReason {
    /// Creates a new domain::policy value with explicit caller-provided
    /// inputs. This constructor is data-only and performs no I/O or
    /// external side effects.
    pub fn new(code: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            redacted_summary: None,
            policy_refs: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Defines the approval request spec SDK value.
/// Construction records local state only; documented runtimes, executors, or ports own side effects.
pub struct ApprovalRequestSpec {
    /// Source label or ref for this item; it is metadata and does not fetch
    /// content by itself.
    pub source: SourceRef,
    /// Destination label or ref for this item; it is metadata and does not
    /// deliver content by itself.
    pub destination: DestinationRef,
    /// Classification value for effect class.
    /// Policy and projection paths use it for finite routing decisions.
    pub effect_class: EffectClass,
    /// Risk classification for the operation or capability.
    /// Policy uses it to decide whether approval, sandboxing, or denial is required.
    pub risk_class: RiskClass,
    /// Dispatcher scope used by this record or request.
    pub dispatcher_scope: DispatcherScope,
    /// Timeout budget in milliseconds for the requested operation.
    pub timeout_ms: u64,
    /// Allowlist for this policy or contract.
    /// Validation uses it to reject undeclared or policy-denied values.
    pub allowed_decisions: Vec<ApprovalDecisionKind>,
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Defines the tool request modification SDK value.
/// Construction records local state only; documented runtimes, executors, or ports own side effects.
pub struct ToolRequestModification {
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Defines the resume policy SDK value.
/// Construction records local state only; documented runtimes, executors, or ports own side effects.
pub struct ResumePolicy {
    /// resume after ms duration in milliseconds.
    pub resume_after_ms: Option<u64>,
    /// Redacted explanation for a denial, failure, status, or package delta.
    pub reason: DecisionReason,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Enumerates the finite policy stage cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum PolicyStage {
    /// Use this variant when the contract needs to represent input; selecting it has no side effect by itself.
    Input,
    /// Use this variant when the contract needs to represent model input projection; selecting it has no side effect by itself.
    ModelInputProjection,
    /// Use this variant when the contract needs to represent pre tool; selecting it has no side effect by itself.
    PreTool,
    /// Use this variant when the contract needs to represent post tool; selecting it has no side effect by itself.
    PostTool,
    /// Use this variant when the contract needs to represent output; selecting it has no side effect by itself.
    Output,
    /// Use this variant when the contract needs to represent handoff; selecting it has no side effect by itself.
    Handoff,
    /// Use this variant when the contract needs to represent stream; selecting it has no side effect by itself.
    Stream,
    /// Use this variant when the contract needs to represent delivery; selecting it has no side effect by itself.
    Delivery,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Enumerates the finite missing dependency cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum MissingDependency {
    /// Use this variant when the contract needs to represent policy snapshot; selecting it has no side effect by itself.
    PolicySnapshot,
    /// Use this variant when the contract needs to represent policy ref; selecting it has no side effect by itself.
    PolicyRef,
    /// Use this variant when the contract needs to represent permission evaluator; selecting it has no side effect by itself.
    PermissionEvaluator,
    /// Use this variant when the contract needs to represent sandbox evaluator; selecting it has no side effect by itself.
    SandboxEvaluator,
    /// Use this variant when the contract needs to represent approval dispatcher; selecting it has no side effect by itself.
    ApprovalDispatcher,
    /// Use this variant when the contract needs to represent adapter; selecting it has no side effect by itself.
    Adapter,
    /// Use this variant when the contract needs to represent sink; selecting it has no side effect by itself.
    Sink,
    /// Use this variant when the contract needs to represent store; selecting it has no side effect by itself.
    Store,
    /// Use this variant when the contract needs to represent journal append; selecting it has no side effect by itself.
    JournalAppend,
    /// Use this variant when the contract needs to represent executor ref; selecting it has no side effect by itself.
    ExecutorRef,
    /// Use this variant when the contract needs to represent isolation runtime; selecting it has no side effect by itself.
    IsolationRuntime,
}

impl MissingDependency {
    /// Returns the reason code currently held by this value.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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

    /// Converts this value into error data.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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
/// Defines the permission policy SDK value.
/// Construction records local state only; documented runtimes, executors, or ports own side effects.
pub struct PermissionPolicy {
    /// Policy reference that must be resolved by the host or runtime before
    /// execution.
    pub policy_ref: PolicyRef,
    /// Collection of grants values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub grants: Vec<PermissionGrant>,
    /// Default decision used by this record or request.
    pub default_decision: PolicyDecision,
}

impl PermissionPolicy {
    /// Returns an updated value with deny all configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn deny_all(policy_ref: PolicyRef) -> Self {
        Self {
            policy_ref,
            grants: Vec::new(),
            default_decision: PolicyDecision::deny("permission.default_deny"),
        }
    }

    /// Builds the check value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
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
/// Defines the permission grant SDK value.
/// Construction records local state only; documented runtimes, executors, or ports own side effects.
pub struct PermissionGrant {
    /// Source label or ref for this item; it is metadata and does not fetch
    /// content by itself.
    pub source: SourceRef,
    /// Capability used by this record or request.
    pub capability: CapabilityPermission,
    /// Decision used by this record or request.
    pub decision: PolicyDecision,
}

impl PermissionGrant {
    fn matches(&self, request: &PermissionRequest) -> bool {
        self.source == request.source && self.capability == request.capability
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Defines the permission request SDK value.
/// Construction records local state only; documented runtimes, executors, or ports own side effects.
pub struct PermissionRequest {
    /// Stage used by this record or request.
    pub stage: PolicyStage,
    /// Source label or ref for this item; it is metadata and does not fetch
    /// content by itself.
    pub source: SourceRef,
    /// Destination label or ref for this item; it is metadata and does not
    /// deliver content by itself.
    pub destination: DestinationRef,
    /// Capability used by this record or request.
    pub capability: CapabilityPermission,
    /// Optional subject value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub subject: Option<EntityRef>,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy: PrivacyClass,
    /// Retention class used by hosts and sinks when storing or exporting this
    /// item.
    pub retention: RetentionClass,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Enumerates the finite capability permission cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum CapabilityPermission {
    /// Use this variant when the contract needs to represent filesystem read; selecting it has no side effect by itself.
    FilesystemRead,
    /// Use this variant when the contract needs to represent filesystem write; selecting it has no side effect by itself.
    FilesystemWrite,
    /// Use this variant when the contract needs to represent network; selecting it has no side effect by itself.
    Network,
    /// Use this variant when the contract needs to represent shell; selecting it has no side effect by itself.
    Shell,
    /// Use this variant when the contract needs to represent mcp; selecting it has no side effect by itself.
    Mcp,
    /// Use this variant when the contract needs to represent media; selecting it has no side effect by itself.
    Media,
    /// Use this variant when the contract needs to represent contacts; selecting it has no side effect by itself.
    Contacts,
    /// Use this variant when the contract needs to represent memory read; selecting it has no side effect by itself.
    MemoryRead,
    /// Use this variant when the contract needs to represent memory write; selecting it has no side effect by itself.
    MemoryWrite,
    /// Use this variant when the contract needs to represent output delivery; selecting it has no side effect by itself.
    OutputDelivery,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Defines the sandbox policy SDK value.
/// Construction records local state only; documented runtimes, executors, or ports own side effects.
pub struct SandboxPolicy {
    /// Policy reference that must be resolved by the host or runtime before
    /// execution.
    pub policy_ref: PolicyRef,
    /// Mode that selects how this operation or contract should behave.
    /// Callers use it to choose the explicit execution path instead of relying on hidden
    /// defaults.
    pub mode: SandboxMode,
    /// Whether the request asks for network access. Host sandbox policy is
    /// still authoritative.
    pub network: NetworkPolicy,
    /// Missing adapter decision used by this record or request.
    pub missing_adapter_decision: PolicyDecision,
}

impl SandboxPolicy {
    /// Builds the host denied value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn host_denied(policy_ref: PolicyRef) -> Self {
        Self {
            policy_ref,
            mode: SandboxMode::DenyHostExecution,
            network: NetworkPolicy::Disabled,
            missing_adapter_decision: PolicyDecision::deny("sandbox.missing_adapter"),
        }
    }

    /// Builds the evaluate value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
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
/// Enumerates the finite sandbox mode cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum SandboxMode {
    /// Use this variant when the contract needs to represent deny host execution; selecting it has no side effect by itself.
    DenyHostExecution,
    /// Use this variant when the contract needs to represent require isolation; selecting it has no side effect by itself.
    RequireIsolation {
        /// Typed adapter ref reference. Resolving or executing it is a
        /// separate policy-gated step.
        adapter_ref: AdapterRef,
    },
    /// Use this variant when the contract needs to represent allow host execution; selecting it has no side effect by itself.
    AllowHostExecution,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Enumerates the finite network policy cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum NetworkPolicy {
    /// Use this variant when the contract needs to represent disabled; selecting it has no side effect by itself.
    Disabled,
    /// Use this variant when the contract needs to represent egress allowlist; selecting it has no side effect by itself.
    EgressAllowlist(Vec<String>),
    /// Use this variant when the contract needs to represent enabled; selecting it has no side effect by itself.
    Enabled,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Defines the approval policy SDK value.
/// Construction records local state only; documented runtimes, executors, or ports own side effects.
pub struct ApprovalPolicy {
    /// Policy reference that must be resolved by the host or runtime before
    /// execution.
    pub policy_ref: PolicyRef,
    /// Default decision used by this record or request.
    pub default_decision: PolicyDecision,
}

impl ApprovalPolicy {
    /// Returns an updated value with ask by default configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn ask_by_default(policy_ref: PolicyRef, approval: ApprovalRequestSpec) -> Self {
        Self {
            policy_ref,
            default_decision: PolicyDecision::Ask { approval },
        }
    }

    /// Builds the classify value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
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
/// Defines the escalation policy SDK value.
/// Construction records local state only; documented runtimes, executors, or ports own side effects.
pub struct EscalationPolicy {
    /// Policy reference that must be resolved by the host or runtime before
    /// execution.
    pub policy_ref: PolicyRef,
    /// Whether dispatcher required is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub dispatcher_required: bool,
    /// Timeout budget in milliseconds for the requested operation.
    pub timeout_ms: u64,
    /// Allowlist for this policy or contract.
    /// Validation uses it to reject undeclared or policy-denied values.
    pub allowed_decisions: Vec<ApprovalDecisionKind>,
}

impl EscalationPolicy {
    /// Evaluate dispatcher.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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
/// Defines the privacy policy SDK value.
/// Construction records local state only; documented runtimes, executors, or ports own side effects.
pub struct PrivacyPolicy {
    /// Policy reference that must be resolved by the host or runtime before
    /// execution.
    pub policy_ref: PolicyRef,
    /// Privacy classification for the value.
    /// Projection, telemetry, and delivery paths use it to enforce redaction and retention.
    pub minimum_privacy: PrivacyClass,
    /// Retention class used by hosts and sinks when storing or exporting this
    /// item.
    pub retention: RetentionClass,
    /// Whether trust required is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub trust_required: TrustClass,
}

impl PrivacyPolicy {
    /// Returns an updated value with safe defaults configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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
/// Defines the content capture policy SDK value.
/// Construction records local state only; documented runtimes, executors, or ports own side effects.
pub struct ContentCapturePolicy {
    /// Policy reference that must be resolved by the host or runtime before
    /// execution.
    pub policy_ref: PolicyRef,
    /// Mode that selects how this operation or contract should behave.
    /// Callers use it to choose the explicit execution path instead of relying on hidden
    /// defaults.
    pub mode: ContentCaptureMode,
    /// Whether source permits content is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub source_permits_content: bool,
    /// Whether sink permits content is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub sink_permits_content: bool,
    /// Whether redaction required is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub redaction_required: bool,
    /// Retention class for referenced content or records.
    /// Stores and telemetry sinks use it to decide how long evidence may be kept.
    pub retention_required: bool,
    /// Whether sampling required is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub sampling_required: bool,
    /// Byte size or byte limit for byte limit.
    /// Use it to enforce bounded reads, writes, summaries, or parser output.
    pub byte_limit: u64,
}

impl ContentCapturePolicy {
    /// Returns an updated value with safe defaults configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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

    /// Returns whether allows raw content applies for this state.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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
/// Enumerates the finite content capture mode cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ContentCaptureMode {
    /// Use this variant when the contract needs to represent off; selecting it has no side effect by itself.
    Off,
    /// Use this variant when the contract needs to represent metadata only; selecting it has no side effect by itself.
    MetadataOnly,
    /// Use this variant when the contract needs to represent redacted summary; selecting it has no side effect by itself.
    RedactedSummary,
    /// Use this variant when the contract needs to represent raw content; selecting it has no side effect by itself.
    RawContent,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Enumerates the finite approval decision kind cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ApprovalDecisionKind {
    /// Use this variant when the contract needs to represent approved; selecting it has no side effect by itself.
    Approved,
    /// Use this variant when the contract needs to represent approved for session; selecting it has no side effect by itself.
    ApprovedForSession,
    /// Use this variant when the contract needs to represent denied; selecting it has no side effect by itself.
    Denied,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Enumerates the finite dispatcher scope cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum DispatcherScope {
    /// Use this variant when the contract needs to represent host; selecting it has no side effect by itself.
    Host,
    /// Use this variant when the contract needs to represent source scoped; selecting it has no side effect by itself.
    SourceScoped,
    /// Use this variant when the contract needs to represent headless; selecting it has no side effect by itself.
    Headless,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Enumerates the finite effect class cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum EffectClass {
    /// Use this variant when the contract needs to represent read; selecting it has no side effect by itself.
    Read,
    /// Use this variant when the contract needs to represent write; selecting it has no side effect by itself.
    Write,
    /// Use this variant when the contract needs to represent process; selecting it has no side effect by itself.
    Process,
    /// Use this variant when the contract needs to represent network; selecting it has no side effect by itself.
    Network,
    /// Use this variant when the contract needs to represent approval dispatch; selecting it has no side effect by itself.
    ApprovalDispatch,
    /// Use this variant when the contract needs to represent output delivery; selecting it has no side effect by itself.
    OutputDelivery,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Enumerates the finite risk class cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum RiskClass {
    /// Use this variant when the contract needs to represent low; selecting it has no side effect by itself.
    Low,
    /// Use this variant when the contract needs to represent medium; selecting it has no side effect by itself.
    Medium,
    /// Use this variant when the contract needs to represent high; selecting it has no side effect by itself.
    High,
    /// Use this variant when the contract needs to represent critical; selecting it has no side effect by itself.
    Critical,
}
