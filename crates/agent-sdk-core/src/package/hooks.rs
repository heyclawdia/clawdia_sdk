//! Runtime-package records and builders. Use these items to describe the immutable
//! per-run package that freezes provider route, capabilities, policies, sidecars,
//! catalogs, and fingerprints. Builders are data-only and must not perform discovery
//! or execution side effects. This file contains the hooks portion of that contract.
//!
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

/// Constant value for the package::hooks contract. Use it to keep SDK
/// records and tests aligned on the same stable value.
pub const HOOK_SIDECAR_KIND: &str = "hook_spec";
/// Constant value for the package::hooks contract. Use it to keep SDK
/// records and tests aligned on the same stable value.
pub const HOOK_SIDECAR_VERSION: &str = "v1";

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
/// Describes the hook id portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct HookId(String);

impl HookId {
    /// Creates a new package::hooks value with explicit caller-provided
    /// inputs. This constructor is data-only and performs no I/O or
    /// external side effects.
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        assert!(!value.is_empty(), "HookId must not be empty");
        Self(value)
    }

    /// Returns this value as str. The accessor is side-effect free and
    /// keeps ownership with the caller.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
/// Describes the hook executor ref portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct HookExecutorRef(String);

impl HookExecutorRef {
    /// Creates a new package::hooks value with explicit caller-provided
    /// inputs. This constructor is data-only and performs no I/O or
    /// external side effects.
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        assert!(!value.is_empty(), "HookExecutorRef must not be empty");
        Self(value)
    }

    /// Returns this value as str. The accessor is side-effect free and
    /// keeps ownership with the caller.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite hook point cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum HookPoint {
    /// Use this variant when the contract needs to represent run starting; selecting it has no side effect by itself.
    RunStarting,
    /// Use this variant when the contract needs to represent before context assembly; selecting it has no side effect by itself.
    BeforeContextAssembly,
    /// Use this variant when the contract needs to represent after context assembly; selecting it has no side effect by itself.
    AfterContextAssembly,
    /// Use this variant when the contract needs to represent before provider projection; selecting it has no side effect by itself.
    BeforeProviderProjection,
    /// Use this variant when the contract needs to represent before model call; selecting it has no side effect by itself.
    BeforeModelCall,
    /// Use this variant when the contract needs to represent on model delta; selecting it has no side effect by itself.
    OnModelDelta,
    /// Use this variant when the contract needs to represent after model call; selecting it has no side effect by itself.
    AfterModelCall,
    /// Use this variant when the contract needs to represent before structured output validation; selecting it has no side effect by itself.
    BeforeStructuredOutputValidation,
    /// Use this variant when the contract needs to represent after structured output validation; selecting it has no side effect by itself.
    AfterStructuredOutputValidation,
    /// Use this variant when the contract needs to represent before tool call; selecting it has no side effect by itself.
    BeforeToolCall,
    /// Use this variant when the contract needs to represent after tool call; selecting it has no side effect by itself.
    AfterToolCall,
    /// Use this variant when the contract needs to represent before approval request; selecting it has no side effect by itself.
    BeforeApprovalRequest,
    /// Use this variant when the contract needs to represent after approval decision; selecting it has no side effect by itself.
    AfterApprovalDecision,
    /// Use this variant when the contract needs to represent before subagent start; selecting it has no side effect by itself.
    BeforeSubagentStart,
    /// Use this variant when the contract needs to represent after subagent terminal; selecting it has no side effect by itself.
    AfterSubagentTerminal,
    /// Use this variant when the contract needs to represent before isolation process start; selecting it has no side effect by itself.
    BeforeIsolationProcessStart,
    /// Use this variant when the contract needs to represent after isolation process exit; selecting it has no side effect by itself.
    AfterIsolationProcessExit,
    /// Use this variant when the contract needs to represent on run cancel requested; selecting it has no side effect by itself.
    OnRunCancelRequested,
    /// Use this variant when the contract needs to represent before run complete; selecting it has no side effect by itself.
    BeforeRunComplete,
    /// Use this variant when the contract needs to represent after run terminal; selecting it has no side effect by itself.
    AfterRunTerminal,
    /// Use this variant when the contract needs to represent before compaction; selecting it has no side effect by itself.
    BeforeCompaction,
    /// Use this variant when the contract needs to represent after compaction; selecting it has no side effect by itself.
    AfterCompaction,
}

impl HookPoint {
    /// Computes or returns allowed response classes for the package::hooks
    /// contract without external I/O or side effects.
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
                Class::RequestRetry,
                Class::ValidateDetach,
                Class::StopCompletionWithRepairNeeded,
            ]),
            Self::AfterRunTerminal => set([Class::Observe]),
            Self::BeforeCompaction => set([Class::Observe, Class::MarkProtectedContext]),
            Self::AfterCompaction => set([Class::Observe, Class::RequestProjectionAuditRepair]),
        }
    }

    /// Reports whether this value is security critical. The check is
    /// pure and does not mutate SDK or host state.
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
/// Enumerates the finite hook source cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum HookSource {
    /// Use this variant when the contract needs to represent host config; selecting it has no side effect by itself.
    HostConfig,
    /// Use this variant when the contract needs to represent in process; selecting it has no side effect by itself.
    InProcess,
    /// Use this variant when the contract needs to represent extension; selecting it has no side effect by itself.
    Extension,
    /// Use this variant when the contract needs to represent sdk default; selecting it has no side effect by itself.
    SdkDefault,
    /// Use this variant when the contract needs to represent test only fake; selecting it has no side effect by itself.
    TestOnlyFake,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the hook ordering portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct HookOrdering {
    /// Phase used by this record or request.
    pub phase: HookOrderingPhase,
    /// Order used by this record or request.
    pub order: i32,
}

impl HookOrdering {
    /// Returns an updated package::hooks value with early applied. This is
    /// data construction only and does not execute the configured behavior.
    pub fn early(order: i32) -> Self {
        Self {
            phase: HookOrderingPhase::Early,
            order,
        }
    }

    /// Returns an updated package::hooks value with normal applied. This is
    /// data construction only and does not execute the configured behavior.
    pub fn normal(order: i32) -> Self {
        Self {
            phase: HookOrderingPhase::Normal,
            order,
        }
    }

    /// Returns an updated package::hooks value with late applied. This is
    /// data construction only and does not execute the configured behavior.
    pub fn late(order: i32) -> Self {
        Self {
            phase: HookOrderingPhase::Late,
            order,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite hook ordering phase cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum HookOrderingPhase {
    /// Use this variant when the contract needs to represent early; selecting it has no side effect by itself.
    Early,
    /// Use this variant when the contract needs to represent normal; selecting it has no side effect by itself.
    Normal,
    /// Use this variant when the contract needs to represent late; selecting it has no side effect by itself.
    Late,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
/// Enumerates the finite hook execution mode cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum HookExecutionMode {
    /// Use this variant when the contract needs to represent blocking; selecting it has no side effect by itself.
    Blocking,
    /// Use this variant when the contract needs to represent non blocking; selecting it has no side effect by itself.
    NonBlocking {
        /// Subscriber queue settings used for streams created with this filter.
        /// It controls capacity, terminal reserve, and overflow behavior for the subscriber.
        queue: HookQueueConfig,
        /// Overflow policy applied when a subscriber queue reaches capacity.
        /// It decides whether to drop, summarize, backpressure, or fail the subscriber.
        overflow: HookOverflowPolicy,
    },
}

impl HookExecutionMode {
    /// Builds the nonblocking observe default value with the documented defaults.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn nonblocking_observe_default() -> Self {
        Self::NonBlocking {
            queue: HookQueueConfig::new(64, 4),
            overflow: HookOverflowPolicy::DropObserveOnly,
        }
    }

    /// Reports whether this value is blocking. The check is pure and
    /// does not mutate SDK or host state.
    pub fn is_blocking(&self) -> bool {
        matches!(self, Self::Blocking)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the hook queue config portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct HookQueueConfig {
    /// Total subscriber queue capacity.
    /// This bounds buffered event frames for a live subscriber.
    pub capacity: NonZeroUsize,
    /// Queue slots reserved for terminal frames.
    /// This keeps important terminal events available even when non-terminal frames overflow.
    pub terminal_reserve: NonZeroUsize,
}

impl HookQueueConfig {
    /// Creates a new package::hooks value with explicit caller-provided
    /// inputs. This constructor is data-only and performs no I/O or
    /// external side effects.
    ///
    /// # Panics
    ///
    /// Panics if constructor invariants fail, such as invalid identifier
    /// text or constructor-specific bounds. Use a fallible constructor such as
    /// `try_new` when one is available for untrusted input.
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
/// Enumerates the finite hook overflow policy cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum HookOverflowPolicy {
    /// Use this variant when the contract needs to represent drop observe only; selecting it has no side effect by itself.
    DropObserveOnly,
    /// Use this variant when the contract needs to represent summarize and continue; selecting it has no side effect by itself.
    SummarizeAndContinue,
    /// Use this variant when the contract needs to represent fail hook invocation; selecting it has no side effect by itself.
    FailHookInvocation,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the hook timeout policy portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct HookTimeoutPolicy {
    /// Timeout budget in milliseconds for the requested operation.
    pub timeout_ms: u64,
}

impl HookTimeoutPolicy {
    /// Returns an updated package::hooks value with bounded ms applied. This
    /// is data construction only and does not execute the configured
    /// behavior.
    pub fn bounded_ms(timeout_ms: u64) -> Self {
        Self { timeout_ms }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite hook failure policy cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum HookFailurePolicy {
    /// Use this variant when the contract needs to represent fail open observe only; selecting it has no side effect by itself.
    FailOpenObserveOnly,
    /// Use this variant when the contract needs to represent deny; selecting it has no side effect by itself.
    Deny,
    /// Use this variant when the contract needs to represent interrupt run; selecting it has no side effect by itself.
    InterruptRun,
    /// Use this variant when the contract needs to represent fail run; selecting it has no side effect by itself.
    FailRun,
}

impl HookFailurePolicy {
    /// Returns an updated package::hooks value with fails closed applied.
    /// This is data construction only and does not execute the configured
    /// behavior.
    pub fn fails_closed(&self) -> bool {
        !matches!(self, Self::FailOpenObserveOnly)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the hook mutation rights portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct HookMutationRights {
    /// Collection of rights values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub rights: BTreeSet<HookMutationRight>,
}

impl HookMutationRights {
    /// Observe only.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn observe_only() -> Self {
        Self {
            rights: set([HookMutationRight::Observe]),
        }
    }

    /// Builds the deny or request approval value with the documented defaults.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn deny_or_request_approval() -> Self {
        Self {
            rights: set([
                HookMutationRight::Observe,
                HookMutationRight::Deny,
                HookMutationRight::RequestApproval,
            ]),
        }
    }

    /// Constructs this value from rights. Use it when adapting
    /// canonical SDK records without introducing a second behavior
    /// path.
    pub fn from_rights(rights: impl IntoIterator<Item = HookMutationRight>) -> Self {
        let mut rights = rights.into_iter().collect::<BTreeSet<_>>();
        rights.insert(HookMutationRight::Observe);
        Self { rights }
    }

    /// Returns whether allows response class applies for this contract.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn allows_response_class(&self, response_class: HookResponseClass) -> bool {
        self.rights
            .contains(&HookMutationRight::from_response_class(response_class))
    }

    /// Validates the package::hooks invariants and returns a typed
    /// error on failure. Validation is pure and does not perform I/O,
    /// dispatch, journal appends, or adapter calls.
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

    /// Reports whether this value is observe only. The check is pure
    /// and does not mutate SDK or host state.
    pub fn is_observe_only(&self) -> bool {
        self.rights == set([HookMutationRight::Observe])
    }

    /// Returns whether can change behavior applies for this state.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn can_change_behavior(&self) -> bool {
        self.rights
            .iter()
            .any(|right| right.response_class().changes_behavior())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite hook mutation right cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum HookMutationRight {
    /// Use this variant when the contract needs to represent observe; selecting it has no side effect by itself.
    Observe,
    /// Use this variant when the contract needs to represent inject context; selecting it has no side effect by itself.
    InjectContext,
    /// Use this variant when the contract needs to represent modify projection; selecting it has no side effect by itself.
    ModifyProjection,
    /// Use this variant when the contract needs to represent request compaction; selecting it has no side effect by itself.
    RequestCompaction,
    /// Use this variant when the contract needs to represent modify validation hints; selecting it has no side effect by itself.
    ModifyValidationHints,
    /// Use this variant when the contract needs to represent modify tool request; selecting it has no side effect by itself.
    ModifyToolRequest,
    /// Use this variant when the contract needs to represent modify approval request; selecting it has no side effect by itself.
    ModifyApprovalRequest,
    /// Use this variant when the contract needs to represent deny; selecting it has no side effect by itself.
    Deny,
    /// Use this variant when the contract needs to represent request approval; selecting it has no side effect by itself.
    RequestApproval,
    /// Use this variant when the contract needs to represent request retry; selecting it has no side effect by itself.
    RequestRetry,
    /// Use this variant when the contract needs to represent rewrite tool result; selecting it has no side effect by itself.
    RewriteToolResult,
    /// Use this variant when the contract needs to represent modify subagent request; selecting it has no side effect by itself.
    ModifySubagentRequest,
    /// Use this variant when the contract needs to represent modify process request; selecting it has no side effect by itself.
    ModifyProcessRequest,
    /// Use this variant when the contract needs to represent validate detach; selecting it has no side effect by itself.
    ValidateDetach,
    /// Use this variant when the contract needs to represent request usage rollup repair; selecting it has no side effect by itself.
    RequestUsageRollupRepair,
    /// Use this variant when the contract needs to represent request cleanup repair; selecting it has no side effect by itself.
    RequestCleanupRepair,
    /// Use this variant when the contract needs to represent mark protected context; selecting it has no side effect by itself.
    MarkProtectedContext,
    /// Use this variant when the contract needs to represent request projection audit repair; selecting it has no side effect by itself.
    RequestProjectionAuditRepair,
    /// Use this variant when the contract needs to represent stop completion with repair needed; selecting it has no side effect by itself.
    StopCompletionWithRepairNeeded,
    /// Use this variant when the contract needs to represent stop run; selecting it has no side effect by itself.
    StopRun,
}

impl HookMutationRight {
    /// Returns the response class currently held by this value.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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

    /// Constructs this value from response class. Use it when adapting
    /// canonical SDK records without introducing a second behavior
    /// path.
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
/// Enumerates the finite hook response class cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum HookResponseClass {
    /// Use this variant when the contract needs to represent observe; selecting it has no side effect by itself.
    Observe,
    /// Use this variant when the contract needs to represent inject context; selecting it has no side effect by itself.
    InjectContext,
    /// Use this variant when the contract needs to represent modify projection; selecting it has no side effect by itself.
    ModifyProjection,
    /// Use this variant when the contract needs to represent request compaction; selecting it has no side effect by itself.
    RequestCompaction,
    /// Use this variant when the contract needs to represent modify validation hints; selecting it has no side effect by itself.
    ModifyValidationHints,
    /// Use this variant when the contract needs to represent modify tool request; selecting it has no side effect by itself.
    ModifyToolRequest,
    /// Use this variant when the contract needs to represent modify approval request; selecting it has no side effect by itself.
    ModifyApprovalRequest,
    /// Use this variant when the contract needs to represent deny; selecting it has no side effect by itself.
    Deny,
    /// Use this variant when the contract needs to represent request approval; selecting it has no side effect by itself.
    RequestApproval,
    /// Use this variant when the contract needs to represent request retry; selecting it has no side effect by itself.
    RequestRetry,
    /// Use this variant when the contract needs to represent rewrite tool result; selecting it has no side effect by itself.
    RewriteToolResult,
    /// Use this variant when the contract needs to represent modify subagent request; selecting it has no side effect by itself.
    ModifySubagentRequest,
    /// Use this variant when the contract needs to represent modify process request; selecting it has no side effect by itself.
    ModifyProcessRequest,
    /// Use this variant when the contract needs to represent validate detach; selecting it has no side effect by itself.
    ValidateDetach,
    /// Use this variant when the contract needs to represent request usage rollup repair; selecting it has no side effect by itself.
    RequestUsageRollupRepair,
    /// Use this variant when the contract needs to represent request cleanup repair; selecting it has no side effect by itself.
    RequestCleanupRepair,
    /// Use this variant when the contract needs to represent mark protected context; selecting it has no side effect by itself.
    MarkProtectedContext,
    /// Use this variant when the contract needs to represent request projection audit repair; selecting it has no side effect by itself.
    RequestProjectionAuditRepair,
    /// Use this variant when the contract needs to represent stop completion with repair needed; selecting it has no side effect by itself.
    StopCompletionWithRepairNeeded,
    /// Use this variant when the contract needs to represent stop run; selecting it has no side effect by itself.
    StopRun,
}

impl HookResponseClass {
    /// Returns whether changes behavior applies for this state.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn changes_behavior(&self) -> bool {
        !matches!(self, Self::Observe)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite hook privacy policy cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum HookPrivacyPolicy {
    /// Use this variant when the contract needs to represent envelope and redacted summary; selecting it has no side effect by itself.
    EnvelopeAndRedactedSummary,
    /// Use this variant when the contract needs to represent content refs only; selecting it has no side effect by itself.
    ContentRefsOnly,
    /// Use this variant when the contract needs to represent content capture allowed; selecting it has no side effect by itself.
    ContentCaptureAllowed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the hook spec portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct HookSpec {
    /// Stable hook id used for typed lineage, lookup, or dedupe.
    pub hook_id: HookId,
    /// Point used by this record or request.
    pub point: HookPoint,
    /// Source label or ref for this item; it is metadata and does not fetch
    /// content by itself.
    pub source: HookSource,
    /// Ordering used by this record or request.
    pub ordering: HookOrdering,
    /// Execution used by this record or request.
    pub execution: HookExecutionMode,
    /// Timeout used by this record or request.
    pub timeout: HookTimeoutPolicy,
    /// Failure used by this record or request.
    pub failure: HookFailurePolicy,
    /// Mutation rights used by this record or request.
    pub mutation_rights: HookMutationRights,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy: HookPrivacyPolicy,
    /// Policy reference that must be resolved by the host or runtime before
    /// execution.
    pub policy_ref: PolicyRef,
    /// Typed executor ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub executor_ref: HookExecutorRef,
}

impl HookSpec {
    /// Observe.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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

    /// Builds the blocking value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
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

    /// Validates the package::hooks invariants and returns a typed
    /// error on failure. Validation is pure and does not perform I/O,
    /// dispatch, journal appends, or adapter calls.
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

    /// Reports whether this value is security relevant. The check is
    /// pure and does not mutate SDK or host state.
    pub fn is_security_relevant(&self) -> bool {
        self.point.is_security_critical() || self.mutation_rights.can_change_behavior()
    }

    /// Builds the sort key value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn sort_key(&self) -> (HookPoint, HookOrderingPhase, i32, String) {
        (
            self.point.clone(),
            self.ordering.phase.clone(),
            self.ordering.order,
            self.hook_id.as_str().to_string(),
        )
    }

    /// Builds the sidecar id value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn sidecar_id(&self) -> String {
        format!("hook.{}", self.hook_id.as_str())
    }

    /// Computes the stable spec hash for this package::hooks value. The
    /// computation is deterministic and side-effect free so it can be
    /// used in package, journal, or test evidence.
    pub fn spec_hash(&self) -> Result<String, AgentError> {
        self.validate()?;
        let bytes = serde_json::to_vec(self).map_err(|error| {
            AgentError::contract_violation(format!("hook spec serialization failed: {error}"))
        })?;
        let digest = Sha256::digest(bytes);
        Ok(format!("sha256:{}", hex_lower(&digest)))
    }

    /// Builds the sidecar snapshot value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
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
            redacted_payload: None,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the hook config portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct HookConfig {
    /// Stable hook id used for typed lineage, lookup, or dedupe.
    pub hook_id: HookId,
    /// Point used by this record or request.
    pub point: HookPoint,
    /// Source label or ref for this item; it is metadata and does not fetch
    /// content by itself.
    pub source: HookSource,
    /// Ordering used by this record or request.
    pub ordering: HookOrdering,
    /// Execution used by this record or request.
    pub execution: HookExecutionMode,
    /// Timeout used by this record or request.
    pub timeout: HookTimeoutPolicy,
    /// Failure used by this record or request.
    pub failure: HookFailurePolicy,
    /// Mutation rights used by this record or request.
    pub mutation_rights: HookMutationRights,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy: HookPrivacyPolicy,
    /// Policy reference that must be resolved by the host or runtime before
    /// execution.
    pub policy_ref: PolicyRef,
    /// Typed executor ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub executor_ref: HookExecutorRef,
}

impl HookConfig {
    /// Builds the lower value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
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

/// Builds the lower code hook value.
/// This is data construction and performs no I/O, journal append, event publication, or process
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

/// Validates the package::hooks invariants and returns a typed error on
/// failure. Validation is pure and does not perform I/O, dispatch,
/// journal appends, or adapter calls.
pub fn validate_hook_specs(specs: &[HookSpec]) -> Result<(), AgentError> {
    for spec in specs {
        spec.validate()?;
    }
    Ok(())
}

/// Returns the ordered hooks for point currently held by this value.
/// This is data-only and does not perform I/O, call host ports, append journals, publish
/// events, or start processes.
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
/// Describes the hook input portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct HookInput {
    /// Stable hook id used for typed lineage, lookup, or dedupe.
    pub hook_id: HookId,
    /// Point used by this record or request.
    pub point: HookPoint,
    /// Run identifier used for lineage, filtering, replay, and dedupe.
    pub run_id: RunId,
    /// Agent identifier used for lineage, filtering, and ownership checks.
    pub agent_id: AgentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Turn identifier for one loop turn within a run.
    pub turn_id: Option<TurnId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Attempt identifier for retry, repair, provider, or tool execution
    /// evidence.
    pub attempt_id: Option<AttemptId>,
    /// Source label or ref for this item; it is metadata and does not fetch
    /// content by itself.
    pub source: SourceRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Destination label or ref for this item; it is metadata and does not
    /// deliver content by itself.
    pub destination: Option<DestinationRef>,
    /// Deterministic package fingerprint used for stale checks, package
    /// evidence, or replay comparisons.
    pub package_fingerprint: RuntimePackageFingerprint,
    /// View used by this record or request.
    pub view: HookView,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
    /// Cancellation used by this record or request.
    pub cancellation: HookCancellationToken,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the hook cancellation token portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct HookCancellationToken {
    /// Whether cancelled is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub cancelled: bool,
}

impl HookCancellationToken {
    /// Cancelled.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn cancelled() -> Self {
        Self { cancelled: true }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the hook view portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct HookView {
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy: PrivacyClass,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Typed subject refs references. Resolving them is separate from
    /// constructing this record.
    pub subject_refs: Vec<EntityRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Content references associated with this record; resolving them is a
    /// separate policy-gated step.
    pub content_refs: Vec<crate::domain::ContentRef>,
}

impl HookView {
    /// Returns an updated value with redacted configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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
/// Enumerates the finite hook response cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum HookResponse {
    /// Use this variant when the contract needs to represent observe only; selecting it has no side effect by itself.
    ObserveOnly,
    /// Use this variant when the contract needs to represent inject context; selecting it has no side effect by itself.
    InjectContext(Vec<ContextInjectionRequest>),
    /// Use this variant when the contract needs to represent modify projection; selecting it has no side effect by itself.
    ModifyProjection(ProjectionPatch),
    /// Use this variant when the contract needs to represent request compaction; selecting it has no side effect by itself.
    RequestCompaction(CompactionRequest),
    /// Use this variant when the contract needs to represent modify validation hints; selecting it has no side effect by itself.
    ModifyValidationHints(ValidationHintPatch),
    /// Use this variant when the contract needs to represent modify tool request; selecting it has no side effect by itself.
    ModifyToolRequest(ToolRequestPatch),
    /// Use this variant when the contract needs to represent modify approval request; selecting it has no side effect by itself.
    ModifyApprovalRequest(ApprovalRequestPatch),
    /// Use this variant when the contract needs to represent deny; selecting it has no side effect by itself.
    Deny(DenyReason),
    /// Use this variant when the contract needs to represent request approval; selecting it has no side effect by itself.
    RequestApproval(ApprovalRequestPatch),
    /// Use this variant when the contract needs to represent request retry; selecting it has no side effect by itself.
    RequestRetry(RetryRequest),
    /// Use this variant when the contract needs to represent rewrite tool result; selecting it has no side effect by itself.
    RewriteToolResult(ToolResultPatch),
    /// Use this variant when the contract needs to represent modify subagent request; selecting it has no side effect by itself.
    ModifySubagentRequest(SubagentRequestPatch),
    /// Use this variant when the contract needs to represent modify process request; selecting it has no side effect by itself.
    ModifyProcessRequest(ProcessRequestPatch),
    /// Use this variant when the contract needs to represent validate detach; selecting it has no side effect by itself.
    ValidateDetach(DetachValidationRequest),
    /// Use this variant when the contract needs to represent request usage rollup repair; selecting it has no side effect by itself.
    RequestUsageRollupRepair(UsageRollupRepairRequest),
    /// Use this variant when the contract needs to represent request cleanup repair; selecting it has no side effect by itself.
    RequestCleanupRepair(CleanupRepairRequest),
    /// Use this variant when the contract needs to represent mark protected context; selecting it has no side effect by itself.
    MarkProtectedContext(Vec<ContextItemId>),
    /// Use this variant when the contract needs to represent request projection audit repair; selecting it has no side effect by itself.
    RequestProjectionAuditRepair(ProjectionAuditRepairRequest),
    /// Use this variant when the contract needs to represent stop completion with repair needed; selecting it has no side effect by itself.
    StopCompletionWithRepairNeeded(RepairNeededReason),
    /// Use this variant when the contract needs to represent stop run; selecting it has no side effect by itself.
    StopRun(StopReason),
}

impl HookResponse {
    /// Returns the response class currently held by this value.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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

    /// Returns whether changes behavior applies for this state.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn changes_behavior(&self) -> bool {
        self.response_class().changes_behavior()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the context injection request portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ContextInjectionRequest {
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the projection patch portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ProjectionPatch {
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the compaction request portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct CompactionRequest {
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the validation hint patch portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ValidationHintPatch {
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the tool request patch portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ToolRequestPatch {
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the approval request patch portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ApprovalRequestPatch {
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the deny reason portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct DenyReason {
    /// Code used by this record or request.
    pub code: String,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the retry request portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct RetryRequest {
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the tool result patch portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ToolResultPatch {
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the subagent request patch portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct SubagentRequestPatch {
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the process request patch portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ProcessRequestPatch {
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the detach validation request portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct DetachValidationRequest {
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the usage rollup repair request portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct UsageRollupRepairRequest {
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the cleanup repair request portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct CleanupRepairRequest {
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the projection audit repair request portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ProjectionAuditRepairRequest {
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the repair needed reason portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct RepairNeededReason {
    /// Code used by this record or request.
    pub code: String,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the stop reason portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct StopReason {
    /// Code used by this record or request.
    pub code: String,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

/// Returns hook policy ref derived from the supplied state.
/// This is data-only and does not perform I/O, call host ports, append journals, publish
/// events, or start processes.
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
