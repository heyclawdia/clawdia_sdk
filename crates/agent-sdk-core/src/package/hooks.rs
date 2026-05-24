use std::{collections::BTreeSet, num::NonZeroUsize};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    capability::PackageSidecarRef,
    domain::{
        AgentError, AgentErrorKind, AgentId, AttemptId, ContextItemId, DestinationRef, EntityRef,
        PolicyKind, PolicyRef, PrivacyClass, RunId, SourceRef, TurnId,
    },
    error::RetryClassification,
    package::{PackageSidecarSnapshot, RuntimePackageFingerprint},
};

pub const HOOK_SIDECAR_KIND: &str = "hook_spec";
pub const HOOK_SIDECAR_VERSION: &str = "v1";

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct HookId(String);

impl HookId {
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        assert!(!value.is_empty(), "HookId must not be empty");
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct HookExecutorRef(String);

impl HookExecutorRef {
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        assert!(!value.is_empty(), "HookExecutorRef must not be empty");
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HookPoint {
    RunStarting,
    BeforeContextAssembly,
    AfterContextAssembly,
    BeforeProviderProjection,
    BeforeModelCall,
    OnModelDelta,
    AfterModelCall,
    BeforeStructuredOutputValidation,
    AfterStructuredOutputValidation,
    BeforeToolCall,
    AfterToolCall,
    BeforeApprovalRequest,
    AfterApprovalDecision,
    BeforeSubagentStart,
    AfterSubagentTerminal,
    BeforeIsolationProcessStart,
    AfterIsolationProcessExit,
    OnRunCancelRequested,
    BeforeRunComplete,
    AfterRunTerminal,
    BeforeCompaction,
    AfterCompaction,
}

impl HookPoint {
    pub fn allowed_response_classes(&self) -> BTreeSet<HookResponseClass> {
        use HookResponseClass as Class;
        match self {
            Self::RunStarting => set([Class::Observe, Class::InjectContext, Class::StopRun]),
            Self::BeforeContextAssembly => set([Class::Observe, Class::InjectContext]),
            Self::AfterContextAssembly => {
                set([Class::Observe, Class::RequestCompaction, Class::StopRun])
            }
            Self::BeforeProviderProjection => {
                set([Class::Observe, Class::ModifyProjection, Class::StopRun])
            }
            Self::BeforeModelCall => set([
                Class::Observe,
                Class::ModifyProjection,
                Class::RequestApproval,
                Class::StopRun,
            ]),
            Self::OnModelDelta => set([Class::Observe]),
            Self::AfterModelCall => set([Class::Observe, Class::RequestRetry, Class::StopRun]),
            Self::BeforeStructuredOutputValidation => {
                set([Class::Observe, Class::ModifyValidationHints])
            }
            Self::AfterStructuredOutputValidation => set([Class::Observe, Class::RequestRetry]),
            Self::BeforeToolCall => set([
                Class::Observe,
                Class::Deny,
                Class::ModifyToolRequest,
                Class::RequestApproval,
            ]),
            Self::AfterToolCall => set([
                Class::Observe,
                Class::RequestRetry,
                Class::RewriteToolResult,
            ]),
            Self::BeforeApprovalRequest => {
                set([Class::Observe, Class::ModifyApprovalRequest, Class::Deny])
            }
            Self::AfterApprovalDecision => set([Class::Observe]),
            Self::BeforeSubagentStart => {
                set([Class::Observe, Class::Deny, Class::ModifySubagentRequest])
            }
            Self::AfterSubagentTerminal => set([Class::Observe, Class::RequestUsageRollupRepair]),
            Self::BeforeIsolationProcessStart => {
                set([Class::Observe, Class::Deny, Class::ModifyProcessRequest])
            }
            Self::AfterIsolationProcessExit => set([Class::Observe, Class::RequestCleanupRepair]),
            Self::OnRunCancelRequested => set([Class::Observe, Class::RequestCleanupRepair]),
            Self::BeforeRunComplete => set([
                Class::Observe,
                Class::ValidateDetach,
                Class::StopCompletionWithRepairNeeded,
            ]),
            Self::AfterRunTerminal => set([Class::Observe]),
            Self::BeforeCompaction => set([Class::Observe, Class::MarkProtectedContext]),
            Self::AfterCompaction => set([Class::Observe, Class::RequestProjectionAuditRepair]),
        }
    }

    pub fn is_security_critical(&self) -> bool {
        matches!(
            self,
            Self::BeforeToolCall
                | Self::BeforeApprovalRequest
                | Self::BeforeSubagentStart
                | Self::BeforeIsolationProcessStart
                | Self::BeforeRunComplete
        )
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HookSource {
    HostConfig,
    InProcess,
    Extension,
    SdkDefault,
    TestOnlyFake,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HookOrdering {
    pub phase: HookOrderingPhase,
    pub order: i32,
}

impl HookOrdering {
    pub fn early(order: i32) -> Self {
        Self {
            phase: HookOrderingPhase::Early,
            order,
        }
    }

    pub fn normal(order: i32) -> Self {
        Self {
            phase: HookOrderingPhase::Normal,
            order,
        }
    }

    pub fn late(order: i32) -> Self {
        Self {
            phase: HookOrderingPhase::Late,
            order,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HookOrderingPhase {
    Early,
    Normal,
    Late,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum HookExecutionMode {
    Blocking,
    NonBlocking {
        queue: HookQueueConfig,
        overflow: HookOverflowPolicy,
    },
}

impl HookExecutionMode {
    pub fn nonblocking_observe_default() -> Self {
        Self::NonBlocking {
            queue: HookQueueConfig::new(64, 4),
            overflow: HookOverflowPolicy::DropObserveOnly,
        }
    }

    pub fn is_blocking(&self) -> bool {
        matches!(self, Self::Blocking)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HookQueueConfig {
    pub capacity: NonZeroUsize,
    pub terminal_reserve: NonZeroUsize,
}

impl HookQueueConfig {
    pub fn new(capacity: usize, terminal_reserve: usize) -> Self {
        Self {
            capacity: NonZeroUsize::new(capacity).expect("hook queue capacity must be nonzero"),
            terminal_reserve: NonZeroUsize::new(terminal_reserve)
                .expect("hook terminal reserve must be nonzero"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HookOverflowPolicy {
    DropObserveOnly,
    SummarizeAndContinue,
    FailHookInvocation,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HookTimeoutPolicy {
    pub timeout_ms: u64,
}

impl HookTimeoutPolicy {
    pub fn bounded_ms(timeout_ms: u64) -> Self {
        Self { timeout_ms }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HookFailurePolicy {
    FailOpenObserveOnly,
    Deny,
    InterruptRun,
    FailRun,
}

impl HookFailurePolicy {
    pub fn fails_closed(&self) -> bool {
        !matches!(self, Self::FailOpenObserveOnly)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HookMutationRights {
    pub rights: BTreeSet<HookMutationRight>,
}

impl HookMutationRights {
    pub fn observe_only() -> Self {
        Self {
            rights: set([HookMutationRight::Observe]),
        }
    }

    pub fn deny_or_request_approval() -> Self {
        Self {
            rights: set([
                HookMutationRight::Observe,
                HookMutationRight::Deny,
                HookMutationRight::RequestApproval,
            ]),
        }
    }

    pub fn from_rights(rights: impl IntoIterator<Item = HookMutationRight>) -> Self {
        let mut rights = rights.into_iter().collect::<BTreeSet<_>>();
        rights.insert(HookMutationRight::Observe);
        Self { rights }
    }

    pub fn allows_response_class(&self, response_class: HookResponseClass) -> bool {
        self.rights
            .contains(&HookMutationRight::from_response_class(response_class))
    }

    pub fn validate_for_point(&self, point: &HookPoint) -> Result<(), AgentError> {
        let allowed = point.allowed_response_classes();
        for right in &self.rights {
            let response_class = right.response_class();
            if !allowed.contains(&response_class) {
                return Err(invalid_package(format!(
                    "hook mutation right {:?} is not allowed at {:?}",
                    right, point
                )));
            }
        }
        Ok(())
    }

    pub fn is_observe_only(&self) -> bool {
        self.rights == set([HookMutationRight::Observe])
    }

    pub fn can_change_behavior(&self) -> bool {
        self.rights
            .iter()
            .any(|right| right.response_class().changes_behavior())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HookMutationRight {
    Observe,
    InjectContext,
    ModifyProjection,
    RequestCompaction,
    ModifyValidationHints,
    ModifyToolRequest,
    ModifyApprovalRequest,
    Deny,
    RequestApproval,
    RequestRetry,
    RewriteToolResult,
    ModifySubagentRequest,
    ModifyProcessRequest,
    ValidateDetach,
    RequestUsageRollupRepair,
    RequestCleanupRepair,
    MarkProtectedContext,
    RequestProjectionAuditRepair,
    StopCompletionWithRepairNeeded,
    StopRun,
}

impl HookMutationRight {
    pub fn response_class(&self) -> HookResponseClass {
        match self {
            Self::Observe => HookResponseClass::Observe,
            Self::InjectContext => HookResponseClass::InjectContext,
            Self::ModifyProjection => HookResponseClass::ModifyProjection,
            Self::RequestCompaction => HookResponseClass::RequestCompaction,
            Self::ModifyValidationHints => HookResponseClass::ModifyValidationHints,
            Self::ModifyToolRequest => HookResponseClass::ModifyToolRequest,
            Self::ModifyApprovalRequest => HookResponseClass::ModifyApprovalRequest,
            Self::Deny => HookResponseClass::Deny,
            Self::RequestApproval => HookResponseClass::RequestApproval,
            Self::RequestRetry => HookResponseClass::RequestRetry,
            Self::RewriteToolResult => HookResponseClass::RewriteToolResult,
            Self::ModifySubagentRequest => HookResponseClass::ModifySubagentRequest,
            Self::ModifyProcessRequest => HookResponseClass::ModifyProcessRequest,
            Self::ValidateDetach => HookResponseClass::ValidateDetach,
            Self::RequestUsageRollupRepair => HookResponseClass::RequestUsageRollupRepair,
            Self::RequestCleanupRepair => HookResponseClass::RequestCleanupRepair,
            Self::MarkProtectedContext => HookResponseClass::MarkProtectedContext,
            Self::RequestProjectionAuditRepair => HookResponseClass::RequestProjectionAuditRepair,
            Self::StopCompletionWithRepairNeeded => {
                HookResponseClass::StopCompletionWithRepairNeeded
            }
            Self::StopRun => HookResponseClass::StopRun,
        }
    }

    pub fn from_response_class(response_class: HookResponseClass) -> Self {
        match response_class {
            HookResponseClass::Observe => Self::Observe,
            HookResponseClass::InjectContext => Self::InjectContext,
            HookResponseClass::ModifyProjection => Self::ModifyProjection,
            HookResponseClass::RequestCompaction => Self::RequestCompaction,
            HookResponseClass::ModifyValidationHints => Self::ModifyValidationHints,
            HookResponseClass::ModifyToolRequest => Self::ModifyToolRequest,
            HookResponseClass::ModifyApprovalRequest => Self::ModifyApprovalRequest,
            HookResponseClass::Deny => Self::Deny,
            HookResponseClass::RequestApproval => Self::RequestApproval,
            HookResponseClass::RequestRetry => Self::RequestRetry,
            HookResponseClass::RewriteToolResult => Self::RewriteToolResult,
            HookResponseClass::ModifySubagentRequest => Self::ModifySubagentRequest,
            HookResponseClass::ModifyProcessRequest => Self::ModifyProcessRequest,
            HookResponseClass::ValidateDetach => Self::ValidateDetach,
            HookResponseClass::RequestUsageRollupRepair => Self::RequestUsageRollupRepair,
            HookResponseClass::RequestCleanupRepair => Self::RequestCleanupRepair,
            HookResponseClass::MarkProtectedContext => Self::MarkProtectedContext,
            HookResponseClass::RequestProjectionAuditRepair => Self::RequestProjectionAuditRepair,
            HookResponseClass::StopCompletionWithRepairNeeded => {
                Self::StopCompletionWithRepairNeeded
            }
            HookResponseClass::StopRun => Self::StopRun,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HookResponseClass {
    Observe,
    InjectContext,
    ModifyProjection,
    RequestCompaction,
    ModifyValidationHints,
    ModifyToolRequest,
    ModifyApprovalRequest,
    Deny,
    RequestApproval,
    RequestRetry,
    RewriteToolResult,
    ModifySubagentRequest,
    ModifyProcessRequest,
    ValidateDetach,
    RequestUsageRollupRepair,
    RequestCleanupRepair,
    MarkProtectedContext,
    RequestProjectionAuditRepair,
    StopCompletionWithRepairNeeded,
    StopRun,
}

impl HookResponseClass {
    pub fn changes_behavior(&self) -> bool {
        !matches!(self, Self::Observe)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HookPrivacyPolicy {
    EnvelopeAndRedactedSummary,
    ContentRefsOnly,
    ContentCaptureAllowed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HookSpec {
    pub hook_id: HookId,
    pub point: HookPoint,
    pub source: HookSource,
    pub ordering: HookOrdering,
    pub execution: HookExecutionMode,
    pub timeout: HookTimeoutPolicy,
    pub failure: HookFailurePolicy,
    pub mutation_rights: HookMutationRights,
    pub privacy: HookPrivacyPolicy,
    pub policy_ref: PolicyRef,
    pub executor_ref: HookExecutorRef,
}

impl HookSpec {
    pub fn observe(
        hook_id: impl Into<String>,
        point: HookPoint,
        source: HookSource,
        executor_ref: impl Into<String>,
        policy_ref: PolicyRef,
    ) -> Self {
        Self {
            hook_id: HookId::new(hook_id),
            point,
            source,
            ordering: HookOrdering::normal(100),
            execution: HookExecutionMode::nonblocking_observe_default(),
            timeout: HookTimeoutPolicy::bounded_ms(250),
            failure: HookFailurePolicy::FailOpenObserveOnly,
            mutation_rights: HookMutationRights::observe_only(),
            privacy: HookPrivacyPolicy::EnvelopeAndRedactedSummary,
            policy_ref,
            executor_ref: HookExecutorRef::new(executor_ref),
        }
    }

    pub fn blocking(
        hook_id: impl Into<String>,
        point: HookPoint,
        source: HookSource,
        executor_ref: impl Into<String>,
        policy_ref: PolicyRef,
        mutation_rights: HookMutationRights,
    ) -> Self {
        Self {
            hook_id: HookId::new(hook_id),
            point,
            source,
            ordering: HookOrdering::normal(100),
            execution: HookExecutionMode::Blocking,
            timeout: HookTimeoutPolicy::bounded_ms(250),
            failure: HookFailurePolicy::InterruptRun,
            mutation_rights,
            privacy: HookPrivacyPolicy::EnvelopeAndRedactedSummary,
            policy_ref,
            executor_ref: HookExecutorRef::new(executor_ref),
        }
    }

    pub fn validate(&self) -> Result<(), AgentError> {
        if self.timeout.timeout_ms == 0 {
            return Err(invalid_package("hook timeout_ms must be greater than zero"));
        }
        if self.policy_ref.as_str().is_empty() {
            return Err(invalid_package("hook policy_ref is required"));
        }
        self.mutation_rights.validate_for_point(&self.point)?;
        if !self.execution.is_blocking() && self.mutation_rights.can_change_behavior() {
            return Err(invalid_package(
                "nonblocking hooks must be observe-only in the first Rust slice",
            ));
        }
        if self.is_security_relevant()
            && matches!(self.failure, HookFailurePolicy::FailOpenObserveOnly)
        {
            return Err(invalid_package(
                "security-relevant hooks cannot use fail-open failure policy",
            ));
        }
        Ok(())
    }

    pub fn is_security_relevant(&self) -> bool {
        self.point.is_security_critical() || self.mutation_rights.can_change_behavior()
    }

    pub fn sort_key(&self) -> (HookPoint, HookOrderingPhase, i32, String) {
        (
            self.point.clone(),
            self.ordering.phase.clone(),
            self.ordering.order,
            self.hook_id.as_str().to_string(),
        )
    }

    pub fn sidecar_id(&self) -> String {
        format!("hook.{}", self.hook_id.as_str())
    }

    pub fn spec_hash(&self) -> Result<String, AgentError> {
        self.validate()?;
        let bytes = serde_json::to_vec(self).map_err(|error| {
            AgentError::contract_violation(format!("hook spec serialization failed: {error}"))
        })?;
        let digest = Sha256::digest(bytes);
        Ok(format!("sha256:{}", hex_lower(&digest)))
    }

    pub fn sidecar_snapshot(&self) -> Result<PackageSidecarSnapshot, AgentError> {
        let content_hash = self.spec_hash()?;
        let mut sidecar_ref =
            PackageSidecarRef::new(self.sidecar_id(), HOOK_SIDECAR_KIND, HOOK_SIDECAR_VERSION);
        sidecar_ref.content_hash = Some(content_hash.clone());
        Ok(PackageSidecarSnapshot {
            sidecar_id: self.sidecar_id(),
            kind: HOOK_SIDECAR_KIND.to_string(),
            version: HOOK_SIDECAR_VERSION.to_string(),
            refs: vec![sidecar_ref],
            policy_refs: vec![self.policy_ref.clone()],
            content_hash,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HookConfig {
    pub hook_id: HookId,
    pub point: HookPoint,
    pub source: HookSource,
    pub ordering: HookOrdering,
    pub execution: HookExecutionMode,
    pub timeout: HookTimeoutPolicy,
    pub failure: HookFailurePolicy,
    pub mutation_rights: HookMutationRights,
    pub privacy: HookPrivacyPolicy,
    pub policy_ref: PolicyRef,
    pub executor_ref: HookExecutorRef,
}

impl HookConfig {
    pub fn lower(self) -> Result<HookSpec, AgentError> {
        let spec = HookSpec {
            hook_id: self.hook_id,
            point: self.point,
            source: self.source,
            ordering: self.ordering,
            execution: self.execution,
            timeout: self.timeout,
            failure: self.failure,
            mutation_rights: self.mutation_rights,
            privacy: self.privacy,
            policy_ref: self.policy_ref,
            executor_ref: self.executor_ref,
        };
        spec.validate()?;
        Ok(spec)
    }
}

pub fn lower_code_hook(
    hook_id: impl Into<String>,
    point: HookPoint,
    executor_ref: impl Into<String>,
    policy_ref: PolicyRef,
) -> Result<HookSpec, AgentError> {
    let spec = HookSpec::observe(
        hook_id,
        point,
        HookSource::InProcess,
        executor_ref,
        policy_ref,
    );
    spec.validate()?;
    Ok(spec)
}

pub fn validate_hook_specs(specs: &[HookSpec]) -> Result<(), AgentError> {
    for spec in specs {
        spec.validate()?;
    }
    Ok(())
}

pub fn ordered_hooks_for_point(specs: &[HookSpec], point: HookPoint) -> Vec<HookSpec> {
    let mut hooks = specs
        .iter()
        .filter(|spec| spec.point == point)
        .cloned()
        .collect::<Vec<_>>();
    hooks.sort_by_key(HookSpec::sort_key);
    hooks
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HookInput {
    pub hook_id: HookId,
    pub point: HookPoint,
    pub run_id: RunId,
    pub agent_id: AgentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<TurnId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt_id: Option<AttemptId>,
    pub source: SourceRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<DestinationRef>,
    pub package_fingerprint: RuntimePackageFingerprint,
    pub view: HookView,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policy_refs: Vec<PolicyRef>,
    pub cancellation: HookCancellationToken,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct HookCancellationToken {
    pub cancelled: bool,
}

impl HookCancellationToken {
    pub fn cancelled() -> Self {
        Self { cancelled: true }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HookView {
    pub privacy: PrivacyClass,
    pub redacted_summary: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subject_refs: Vec<EntityRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content_refs: Vec<crate::domain::ContentRef>,
}

impl HookView {
    pub fn redacted(redacted_summary: impl Into<String>) -> Self {
        Self {
            privacy: PrivacyClass::ContentRefsOnly,
            redacted_summary: redacted_summary.into(),
            subject_refs: Vec::new(),
            content_refs: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum HookResponse {
    ObserveOnly,
    InjectContext(Vec<ContextInjectionRequest>),
    ModifyProjection(ProjectionPatch),
    RequestCompaction(CompactionRequest),
    ModifyValidationHints(ValidationHintPatch),
    ModifyToolRequest(ToolRequestPatch),
    ModifyApprovalRequest(ApprovalRequestPatch),
    Deny(DenyReason),
    RequestApproval(ApprovalRequestPatch),
    RequestRetry(RetryRequest),
    RewriteToolResult(ToolResultPatch),
    ModifySubagentRequest(SubagentRequestPatch),
    ModifyProcessRequest(ProcessRequestPatch),
    ValidateDetach(DetachValidationRequest),
    RequestUsageRollupRepair(UsageRollupRepairRequest),
    RequestCleanupRepair(CleanupRepairRequest),
    MarkProtectedContext(Vec<ContextItemId>),
    RequestProjectionAuditRepair(ProjectionAuditRepairRequest),
    StopCompletionWithRepairNeeded(RepairNeededReason),
    StopRun(StopReason),
}

impl HookResponse {
    pub fn response_class(&self) -> HookResponseClass {
        match self {
            Self::ObserveOnly => HookResponseClass::Observe,
            Self::InjectContext(_) => HookResponseClass::InjectContext,
            Self::ModifyProjection(_) => HookResponseClass::ModifyProjection,
            Self::RequestCompaction(_) => HookResponseClass::RequestCompaction,
            Self::ModifyValidationHints(_) => HookResponseClass::ModifyValidationHints,
            Self::ModifyToolRequest(_) => HookResponseClass::ModifyToolRequest,
            Self::ModifyApprovalRequest(_) => HookResponseClass::ModifyApprovalRequest,
            Self::Deny(_) => HookResponseClass::Deny,
            Self::RequestApproval(_) => HookResponseClass::RequestApproval,
            Self::RequestRetry(_) => HookResponseClass::RequestRetry,
            Self::RewriteToolResult(_) => HookResponseClass::RewriteToolResult,
            Self::ModifySubagentRequest(_) => HookResponseClass::ModifySubagentRequest,
            Self::ModifyProcessRequest(_) => HookResponseClass::ModifyProcessRequest,
            Self::ValidateDetach(_) => HookResponseClass::ValidateDetach,
            Self::RequestUsageRollupRepair(_) => HookResponseClass::RequestUsageRollupRepair,
            Self::RequestCleanupRepair(_) => HookResponseClass::RequestCleanupRepair,
            Self::MarkProtectedContext(_) => HookResponseClass::MarkProtectedContext,
            Self::RequestProjectionAuditRepair(_) => {
                HookResponseClass::RequestProjectionAuditRepair
            }
            Self::StopCompletionWithRepairNeeded(_) => {
                HookResponseClass::StopCompletionWithRepairNeeded
            }
            Self::StopRun(_) => HookResponseClass::StopRun,
        }
    }

    pub fn changes_behavior(&self) -> bool {
        self.response_class().changes_behavior()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ContextInjectionRequest {
    pub redacted_summary: String,
    pub policy_refs: Vec<PolicyRef>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProjectionPatch {
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CompactionRequest {
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ValidationHintPatch {
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ToolRequestPatch {
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ApprovalRequestPatch {
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DenyReason {
    pub code: String,
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RetryRequest {
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ToolResultPatch {
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubagentRequestPatch {
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProcessRequestPatch {
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DetachValidationRequest {
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct UsageRollupRepairRequest {
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CleanupRepairRequest {
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProjectionAuditRepairRequest {
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RepairNeededReason {
    pub code: String,
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StopReason {
    pub code: String,
    pub redacted_summary: String,
}

pub fn hook_policy_ref(id: impl Into<String>) -> PolicyRef {
    PolicyRef::with_kind(PolicyKind::RuntimePackage, id)
}

fn set<T, const N: usize>(items: [T; N]) -> BTreeSet<T>
where
    T: Ord,
{
    items.into_iter().collect()
}

fn invalid_package(message: impl Into<String>) -> AgentError {
    AgentError::new(
        AgentErrorKind::InvalidPackage,
        RetryClassification::HostConfigurationNeeded,
        message,
    )
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}
