//! Application-layer coordination over core primitives. Use these services to lower
//! helpers, drive runs, validate output, coordinate tools, approvals, delivery,
//! isolation, telemetry, and feature layers. Methods in this layer may call
//! configured ports, mutate in-memory stores, append journals, or publish events as
//! documented. This file contains the loop state portion of that contract.
//!
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{
    error::{AgentError, AgentErrorKind, RetryClassification},
    journal::JournalRecordKind,
    recovery::RecoveryClassification,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
/// Names the finite phases of the SDK run loop.
/// Use it in state tables, journal records, and tests to describe where a run is paused or
/// executing; the enum is a marker only and side effects happen in the driver transition that
/// enters the phase.
pub enum LoopState {
    /// The run has been accepted and initial package, policy, and registry checks are beginning.
    Starting,
    /// The runtime is collecting context contributions before anything is shown to a provider.
    ContextAssembly,
    /// Context has been admitted and is being projected into the provider-visible request shape.
    ProviderProjection,
    /// A provider call is active and model deltas may be emitted.
    ModelStreaming,
    /// Stream rules are inspecting, interrupting, or transforming an active model stream.
    StreamIntervention,
    /// The runtime is interpreting provider output into candidate tool calls or loop actions.
    ToolPlanning,
    /// A tool or extension action is waiting for an approval decision before execution.
    Approval,
    /// Policy or approval denied a requested tool before the executor was started.
    ToolDenied,
    /// A journal-backed tool intent has been recorded and the configured executor may be running.
    ToolExecution,
    /// Cancellation, timeout, stream policy, or host intervention interrupted active work.
    Interrupted,
    /// The run is suspended and requires a wake, resume signal, or durable replay input.
    WaitingForResume,
    /// The runtime is compacting context or summaries before continuing the loop.
    Compaction,
    /// The current iteration completed and the driver is preparing another loop iteration.
    Continue,
    /// Replay, repair, or reconciliation is resolving an incomplete or uncertain effect.
    Recovery,
    /// The run reached a successful terminal result.
    Completed,
    /// The run reached a cancellation terminal result.
    Cancelled,
    /// The run reached an unrecoverable failure terminal result.
    Failed,
}

impl LoopState {
    /// Constant value for the application::loop_state contract. Use it
    /// to keep SDK records and tests aligned on the same stable value.
    pub const ALL: [Self; 17] = [
        Self::Starting,
        Self::ContextAssembly,
        Self::ProviderProjection,
        Self::ModelStreaming,
        Self::StreamIntervention,
        Self::ToolPlanning,
        Self::Approval,
        Self::ToolDenied,
        Self::ToolExecution,
        Self::Interrupted,
        Self::WaitingForResume,
        Self::Compaction,
        Self::Continue,
        Self::Recovery,
        Self::Completed,
        Self::Cancelled,
        Self::Failed,
    ];

    /// Returns the all currently held by this value.
    /// This is state-table metadata only and does not advance a run or mutate loop state.
    pub fn all() -> &'static [Self] {
        &Self::ALL
    }

    /// Returns the stable contract spelling for this loop state.
    /// This is a pure mapping used by fixtures and diagnostics; it does not advance the loop.
    pub fn contract_name(self) -> &'static str {
        match self {
            Self::Starting => "Starting",
            Self::ContextAssembly => "ContextAssembly",
            Self::ProviderProjection => "ProviderProjection",
            Self::ModelStreaming => "ModelStreaming",
            Self::StreamIntervention => "StreamIntervention",
            Self::ToolPlanning => "ToolPlanning",
            Self::Approval => "Approval",
            Self::ToolDenied => "ToolDenied",
            Self::ToolExecution => "ToolExecution",
            Self::Interrupted => "Interrupted",
            Self::WaitingForResume => "WaitingForResume",
            Self::Compaction => "Compaction",
            Self::Continue => "Continue",
            Self::Recovery => "Recovery",
            Self::Completed => "Completed",
            Self::Cancelled => "Cancelled",
            Self::Failed => "Failed",
        }
    }

    /// Reports whether this value is terminal. The check is pure and
    /// does not mutate SDK or host state.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Cancelled)
    }

    /// Returns whether can carry terminal result applies for this state.
    /// This is state-table metadata only and does not advance a run or mutate loop state.
    pub fn can_carry_terminal_result(self) -> bool {
        matches!(self, Self::Completed | Self::Cancelled | Self::Failed)
    }

    /// Returns whether requires cancel transition applies for this state.
    /// This is state-table metadata only and does not advance a run or mutate loop state.
    pub fn requires_cancel_transition(self) -> bool {
        !self.is_terminal()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite loop trigger cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum LoopTrigger {
    /// Use this variant when the contract needs to represent start run; selecting it has no side effect by itself.
    StartRun,
    /// Use this variant when the contract needs to represent context ready; selecting it has no side effect by itself.
    ContextReady,
    /// Use this variant when the contract needs to represent projection ready; selecting it has no side effect by itself.
    ProjectionReady,
    /// Use this variant when the contract needs to represent tool use; selecting it has no side effect by itself.
    ToolUse,
    /// Use this variant when the contract needs to represent stream rule match; selecting it has no side effect by itself.
    StreamRuleMatch,
    /// Use this variant when the contract needs to represent end turn; selecting it has no side effect by itself.
    EndTurn,
    /// Use this variant when the contract needs to represent provider failure; selecting it has no side effect by itself.
    ProviderFailure,
    /// Use this variant when the contract needs to represent max iterations reached; selecting it has no side effect by itself.
    MaxIterationsReached {
        /// Outcome used by this record or request.
        outcome: MaxIterationOutcome,
    },
    /// Use this variant when the contract needs to represent compaction needed; selecting it has no side effect by itself.
    CompactionNeeded,
    /// Use this variant when the contract needs to represent stream stop run; selecting it has no side effect by itself.
    StreamStopRun,
    /// Use this variant when the contract needs to represent stream abort and retry; selecting it has no side effect by itself.
    StreamAbortAndRetry,
    /// Use this variant when the contract needs to represent stream pause for approval; selecting it has no side effect by itself.
    StreamPauseForApproval,
    /// Use this variant when the contract needs to represent stream unsafe intervention; selecting it has no side effect by itself.
    StreamUnsafeIntervention,
    /// Use this variant when the contract needs to represent policy allow; selecting it has no side effect by itself.
    PolicyAllow,
    /// Use this variant when the contract needs to represent policy ask; selecting it has no side effect by itself.
    PolicyAsk,
    /// Use this variant when the contract needs to represent policy deny; selecting it has no side effect by itself.
    PolicyDeny,
    /// Use this variant when the contract needs to represent approved; selecting it has no side effect by itself.
    Approved,
    /// Use this variant when the contract needs to represent approval denied; selecting it has no side effect by itself.
    ApprovalDenied,
    /// Use this variant when the contract needs to represent approval timeout; selecting it has no side effect by itself.
    ApprovalTimeout,
    /// Use this variant when the contract needs to represent approval transport fatal; selecting it has no side effect by itself.
    ApprovalTransportFatal,
    /// Use this variant when the contract needs to represent stream approval resumed; selecting it has no side effect by itself.
    StreamApprovalResumed,
    /// Use this variant when the contract needs to represent continue with denied result; selecting it has no side effect by itself.
    ContinueWithDeniedResult,
    /// Use this variant when the contract needs to represent fail on denied; selecting it has no side effect by itself.
    FailOnDenied,
    /// Use this variant when the contract needs to represent tool complete; selecting it has no side effect by itself.
    ToolComplete,
    /// Use this variant when the contract needs to represent tool interrupt; selecting it has no side effect by itself.
    ToolInterrupt,
    /// Use this variant when the contract needs to represent tool failure; selecting it has no side effect by itself.
    ToolFailure,
    /// Use this variant when the contract needs to represent wait for resume; selecting it has no side effect by itself.
    WaitForResume,
    /// Use this variant when the contract needs to represent resume allowed; selecting it has no side effect by itself.
    ResumeAllowed,
    /// Use this variant when the contract needs to represent resume denied; selecting it has no side effect by itself.
    ResumeDenied,
    /// Use this variant when the contract needs to represent compaction complete; selecting it has no side effect by itself.
    CompactionComplete,
    /// Use this variant when the contract needs to represent continue loop; selecting it has no side effect by itself.
    ContinueLoop,
    /// Use this variant when the contract needs to represent failure classified; selecting it has no side effect by itself.
    FailureClassified {
        /// Classification used by this record or request.
        classification: RecoveryClassification,
    },
    /// Use this variant when the contract needs to represent repair applied; selecting it has no side effect by itself.
    RepairApplied,
    /// Use this variant when the contract needs to represent repair completed terminal; selecting it has no side effect by itself.
    RepairCompletedTerminal,
    /// Use this variant when the contract needs to represent recovery irrecoverable; selecting it has no side effect by itself.
    RecoveryIrrecoverable,
    /// Use this variant when the contract needs to represent cancel requested; selecting it has no side effect by itself.
    CancelRequested,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite max iteration outcome cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum MaxIterationOutcome {
    /// Use this variant when the contract needs to represent complete; selecting it has no side effect by itself.
    Complete,
    /// Use this variant when the contract needs to represent fail; selecting it has no side effect by itself.
    Fail,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite transition guard cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum TransitionGuard {
    /// Use this variant when the contract needs to represent none; selecting it has no side effect by itself.
    None,
    /// Use this variant when the contract needs to represent package valid; selecting it has no side effect by itself.
    PackageValid,
    /// Use this variant when the contract needs to represent budget valid; selecting it has no side effect by itself.
    BudgetValid,
    /// Use this variant when the contract needs to represent package hashes match; selecting it has no side effect by itself.
    PackageHashesMatch,
    /// Use this variant when the contract needs to represent model message has tool calls; selecting it has no side effect by itself.
    ModelMessageHasToolCalls,
    /// Use this variant when the contract needs to represent rule action allowed; selecting it has no side effect by itself.
    RuleActionAllowed,
    /// Use this variant when the contract needs to represent final message complete; selecting it has no side effect by itself.
    FinalMessageComplete,
    /// Use this variant when the contract needs to represent provider failure classified; selecting it has no side effect by itself.
    ProviderFailureClassified,
    /// Use this variant when the contract needs to represent permissions pass; selecting it has no side effect by itself.
    PermissionsPass,
    /// Use this variant when the contract needs to represent dispatcher available or escalation configured; selecting it has no side effect by itself.
    DispatcherAvailableOrEscalationConfigured,
    /// Use this variant when the contract needs to represent denied result allowed; selecting it has no side effect by itself.
    DeniedResultAllowed,
    /// Use this variant when the contract needs to represent decision valid; selecting it has no side effect by itself.
    DecisionValid,
    /// Use this variant when the contract needs to represent timeout elapsed; selecting it has no side effect by itself.
    TimeoutElapsed,
    /// Use this variant when the contract needs to represent approval transport fatal; selecting it has no side effect by itself.
    ApprovalTransportFatal,
    /// Use this variant when the contract needs to represent terminal status appended; selecting it has no side effect by itself.
    TerminalStatusAppended,
    /// Use this variant when the contract needs to represent interrupt resumable; selecting it has no side effect by itself.
    InterruptResumable,
    /// Use this variant when the contract needs to represent resume token required; selecting it has no side effect by itself.
    ResumeTokenRequired,
    /// Use this variant when the contract needs to represent checkpoint and package valid; selecting it has no side effect by itself.
    CheckpointAndPackageValid,
    /// Use this variant when the contract needs to represent protected context preserved; selecting it has no side effect by itself.
    ProtectedContextPreserved,
    /// Use this variant when the contract needs to represent repair plan safe; selecting it has no side effect by itself.
    RepairPlanSafe,
    /// Use this variant when the contract needs to represent invariant restored; selecting it has no side effect by itself.
    InvariantRestored,
    /// Use this variant when the contract needs to represent max iteration budget exhausted; selecting it has no side effect by itself.
    MaxIterationBudgetExhausted,
    /// Use this variant when the contract needs to represent cancellation requested; selecting it has no side effect by itself.
    CancellationRequested,
    /// Use this variant when the contract needs to represent unsafe intervention; selecting it has no side effect by itself.
    UnsafeIntervention,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
/// Holds transition guard set application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct TransitionGuardSet {
    #[serde(default)]
    satisfied: BTreeSet<TransitionGuard>,
}

impl TransitionGuardSet {
    /// Creates a new application::loop_state value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns an updated value with with configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn with(mut self, guard: TransitionGuard) -> Self {
        self.satisfied.insert(guard);
        self
    }

    /// Reads the stored contains without registry or runtime work.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn contains(&self, guard: TransitionGuard) -> bool {
        guard == TransitionGuard::None || self.satisfied.contains(&guard)
    }

    /// Returns for rule derived from the supplied state.
    /// This uses only local coordinator state and performs no hidden host work.
    pub fn for_rule(rule: &TransitionRule) -> Self {
        if rule.guard == TransitionGuard::None {
            Self::default()
        } else {
            Self::default().with(rule.guard)
        }
    }
}

impl<const N: usize> From<[TransitionGuard; N]> for TransitionGuardSet {
    fn from(guards: [TransitionGuard; N]) -> Self {
        let mut set = Self::default();
        for guard in guards {
            set.satisfied.insert(guard);
        }
        set
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite checkpoint policy cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum CheckpointPolicy {
    /// Use this variant when the contract needs to represent none; selecting it has no side effect by itself.
    None,
    /// Use this variant when the contract needs to represent before; selecting it has no side effect by itself.
    Before,
    /// Use this variant when the contract needs to represent after; selecting it has no side effect by itself.
    After,
    /// Use this variant when the contract needs to represent before and after; selecting it has no side effect by itself.
    BeforeAndAfter,
    /// Use this variant when the contract needs to represent terminal; selecting it has no side effect by itself.
    Terminal,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite side effect policy cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum SideEffectPolicy {
    /// Use this variant when the contract needs to represent none; selecting it has no side effect by itself.
    None,
    /// Use this variant when the contract needs to represent intent before effect; selecting it has no side effect by itself.
    IntentBeforeEffect,
    /// Use this variant when the contract needs to represent idempotent retry allowed; selecting it has no side effect by itself.
    IdempotentRetryAllowed,
    /// Use this variant when the contract needs to represent non idempotent fail closed; selecting it has no side effect by itself.
    NonIdempotentFailClosed,
    /// Use this variant when the contract needs to represent reconcile required; selecting it has no side effect by itself.
    ReconcileRequired,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite loop event kind cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum LoopEventKind {
    /// Use this variant when the contract needs to represent run started; selecting it has no side effect by itself.
    RunStarted,
    /// Use this variant when the contract needs to represent run completed; selecting it has no side effect by itself.
    RunCompleted,
    /// Use this variant when the contract needs to represent run failed; selecting it has no side effect by itself.
    RunFailed,
    /// Use this variant when the contract needs to represent run cancelled; selecting it has no side effect by itself.
    RunCancelled,
    /// Use this variant when the contract needs to represent run cancel requested; selecting it has no side effect by itself.
    RunCancelRequested,
    /// Use this variant when the contract needs to represent run checkpointed; selecting it has no side effect by itself.
    RunCheckpointed,
    /// Use this variant when the contract needs to represent run resume requested; selecting it has no side effect by itself.
    RunResumeRequested,
    /// Use this variant when the contract needs to represent run resume failed; selecting it has no side effect by itself.
    RunResumeFailed,
    /// Use this variant when the contract needs to represent context assembled; selecting it has no side effect by itself.
    ContextAssembled,
    /// Use this variant when the contract needs to represent provider request projected; selecting it has no side effect by itself.
    ProviderRequestProjected,
    /// Use this variant when the contract needs to represent model attempt started; selecting it has no side effect by itself.
    ModelAttemptStarted,
    /// Use this variant when the contract needs to represent model attempt failed; selecting it has no side effect by itself.
    ModelAttemptFailed,
    /// Use this variant when the contract needs to represent model message completed; selecting it has no side effect by itself.
    ModelMessageCompleted,
    /// Use this variant when the contract needs to represent tool requested; selecting it has no side effect by itself.
    ToolRequested,
    /// Use this variant when the contract needs to represent tool approval required; selecting it has no side effect by itself.
    ToolApprovalRequired,
    /// Use this variant when the contract needs to represent tool started; selecting it has no side effect by itself.
    ToolStarted,
    /// Use this variant when the contract needs to represent tool completed; selecting it has no side effect by itself.
    ToolCompleted,
    /// Use this variant when the contract needs to represent tool failed; selecting it has no side effect by itself.
    ToolFailed,
    /// Use this variant when the contract needs to represent tool interrupted; selecting it has no side effect by itself.
    ToolInterrupted,
    /// Use this variant when the contract needs to represent approval requested; selecting it has no side effect by itself.
    ApprovalRequested,
    /// Use this variant when the contract needs to represent approval responded; selecting it has no side effect by itself.
    ApprovalResponded,
    /// Use this variant when the contract needs to represent approval timed out; selecting it has no side effect by itself.
    ApprovalTimedOut,
    /// Use this variant when the contract needs to represent approval denied; selecting it has no side effect by itself.
    ApprovalDenied,
    /// Use this variant when the contract needs to represent stream rule matched; selecting it has no side effect by itself.
    StreamRuleMatched,
    /// Use this variant when the contract needs to represent stream intervention applied; selecting it has no side effect by itself.
    StreamInterventionApplied,
    /// Use this variant when the contract needs to represent context compaction completed; selecting it has no side effect by itself.
    ContextCompactionCompleted,
    /// Use this variant when the contract needs to represent recovery planned; selecting it has no side effect by itself.
    RecoveryPlanned,
    /// Use this variant when the contract needs to represent replay completed; selecting it has no side effect by itself.
    ReplayCompleted,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite loop terminal status cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum LoopTerminalStatus {
    /// Use this variant when the contract needs to represent completed; selecting it has no side effect by itself.
    Completed,
    /// Use this variant when the contract needs to represent cancelled; selecting it has no side effect by itself.
    Cancelled,
    /// Use this variant when the contract needs to represent failed; selecting it has no side effect by itself.
    Failed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite loop stop reason cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum LoopStopReason {
    /// Use this variant when the contract needs to represent end turn; selecting it has no side effect by itself.
    EndTurn,
    /// Use this variant when the contract needs to represent stream rule stop; selecting it has no side effect by itself.
    StreamRuleStop,
    /// Use this variant when the contract needs to represent max iterations; selecting it has no side effect by itself.
    MaxIterations {
        /// Outcome used by this record or request.
        outcome: MaxIterationOutcome,
    },
    /// Use this variant when the contract needs to represent cancelled; selecting it has no side effect by itself.
    Cancelled,
    /// Use this variant when the contract needs to represent provider failure; selecting it has no side effect by itself.
    ProviderFailure,
    /// Use this variant when the contract needs to represent unsafe stream intervention; selecting it has no side effect by itself.
    UnsafeStreamIntervention,
    /// Use this variant when the contract needs to represent tool denied; selecting it has no side effect by itself.
    ToolDenied,
    /// Use this variant when the contract needs to represent approval denied; selecting it has no side effect by itself.
    ApprovalDenied,
    /// Use this variant when the contract needs to represent approval timeout; selecting it has no side effect by itself.
    ApprovalTimeout,
    /// Use this variant when the contract needs to represent approval transport fatal; selecting it has no side effect by itself.
    ApprovalTransportFatal,
    /// Use this variant when the contract needs to represent tool failure; selecting it has no side effect by itself.
    ToolFailure,
    /// Use this variant when the contract needs to represent resume denied; selecting it has no side effect by itself.
    ResumeDenied,
    /// Use this variant when the contract needs to represent recovery completed; selecting it has no side effect by itself.
    RecoveryCompleted,
    /// Use this variant when the contract needs to represent recovery irrecoverable; selecting it has no side effect by itself.
    RecoveryIrrecoverable,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
/// Holds loop terminal result application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct LoopTerminalResult {
    /// Finite status for this record or lifecycle stage.
    pub status: LoopTerminalStatus,
    /// Stop reason used by this record or request.
    pub stop_reason: LoopStopReason,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Holds transition rule application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct TransitionRule {
    /// From state used by this record or request.
    pub from_state: LoopState,
    /// Trigger used by this record or request.
    pub trigger: LoopTrigger,
    /// Guard used by this record or request.
    pub guard: TransitionGuard,
    /// Bounded events included in this record. Limits and truncation are
    /// represented by companion metadata when applicable.
    pub events: &'static [LoopEventKind],
    /// Journal record kinds produced by this capability or feature.
    /// Use them to keep replay and recovery fixtures aligned with the public contract.
    pub journal_records: &'static [JournalRecordKind],
    /// Checkpoint policy used by this record or request.
    pub checkpoint_policy: CheckpointPolicy,
    /// Side effect policy used by this record or request.
    pub side_effect_policy: SideEffectPolicy,
    /// Next state used by this record or request.
    pub next_state: LoopState,
    /// Optional terminal result value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub terminal_result: Option<LoopTerminalResult>,
    /// Optional recovery classification value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub recovery_classification: Option<RecoveryClassification>,
}

impl TransitionRule {
    /// Returns output derived from the supplied state.
    /// This uses only local coordinator state and performs no hidden host work.
    pub fn output(&self) -> TransitionOutput {
        TransitionOutput {
            from_state: self.from_state,
            trigger: self.trigger,
            guard: self.guard,
            events: self.events.to_vec(),
            journal_records: self.journal_records.to_vec(),
            checkpoint_policy: self.checkpoint_policy,
            side_effect_policy: self.side_effect_policy,
            next_state: self.next_state,
            terminal_result: self.terminal_result,
            recovery_classification: self.recovery_classification,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Holds transition input application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct TransitionInput {
    /// State used by this record or request.
    pub state: LoopState,
    /// Trigger used by this record or request.
    pub trigger: LoopTrigger,
    #[serde(default)]
    /// Guards used by this record or request.
    pub guards: TransitionGuardSet,
}

impl TransitionInput {
    /// Creates a new application::loop_state value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new(state: LoopState, trigger: LoopTrigger) -> Self {
        Self {
            state,
            trigger,
            guards: TransitionGuardSet::default(),
        }
    }

    /// Returns this value with its guard setting replaced. The method
    /// follows builder-style data construction and does not execute
    /// external work.
    pub fn with_guard(mut self, guard: TransitionGuard) -> Self {
        self.guards = self.guards.with(guard);
        self
    }

    /// Returns this value with its guards setting replaced. The method
    /// follows builder-style data construction and does not execute
    /// external work.
    pub fn with_guards(mut self, guards: TransitionGuardSet) -> Self {
        self.guards = guards;
        self
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Holds transition output application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct TransitionOutput {
    /// From state used by this record or request.
    pub from_state: LoopState,
    /// Trigger used by this record or request.
    pub trigger: LoopTrigger,
    /// Guard used by this record or request.
    pub guard: TransitionGuard,
    /// Bounded events included in this record. Limits and truncation are
    /// represented by companion metadata when applicable.
    pub events: Vec<LoopEventKind>,
    /// Journal record kinds produced by this capability or feature.
    /// Use them to keep replay and recovery fixtures aligned with the public contract.
    pub journal_records: Vec<JournalRecordKind>,
    /// Checkpoint policy used by this record or request.
    pub checkpoint_policy: CheckpointPolicy,
    /// Side effect policy used by this record or request.
    pub side_effect_policy: SideEffectPolicy,
    /// Next state used by this record or request.
    pub next_state: LoopState,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional terminal result value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub terminal_result: Option<LoopTerminalResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional recovery classification value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub recovery_classification: Option<RecoveryClassification>,
}

#[derive(Clone, Copy, Debug, Default)]
/// Holds agent state machine application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct AgentStateMachine;

impl AgentStateMachine {
    /// Returns the transition table derived from this value.
    /// This uses only local coordinator state and performs no hidden host work.
    pub fn transition_table(&self) -> &'static [TransitionRule] {
        transition_table()
    }

    /// Validates the application::loop_state invariants and returns a
    /// typed error on failure. Validation is pure and does not perform
    /// I/O, dispatch, journal appends, or adapter calls.
    pub fn validate_transition(
        &self,
        input: TransitionInput,
    ) -> Result<TransitionOutput, AgentError> {
        validate_transition(input)
    }
}

/// Validates the application::loop_state invariants and returns a typed
/// error on failure. Validation is pure and does not perform I/O,
/// dispatch, journal appends, or adapter calls.
pub fn validate_transition(input: TransitionInput) -> Result<TransitionOutput, AgentError> {
    let rule = transition_table()
        .iter()
        .find(|rule| rule.from_state == input.state && rule.trigger == input.trigger)
        .ok_or_else(|| invalid_transition(input.state, input.trigger, None))?;

    if !input.guards.contains(rule.guard) {
        return Err(invalid_transition(
            input.state,
            input.trigger,
            Some(rule.guard),
        ));
    }

    Ok(rule.output())
}

/// Returns the transition table derived from this value.
/// This derives SDK state locally and does not call host adapters.
pub fn transition_table() -> &'static [TransitionRule] {
    TRANSITION_TABLE
}

/// Returns the contract state names derived from this value.
/// This derives SDK state locally and does not call host adapters.
pub fn contract_state_names() -> &'static [&'static str] {
    CONTRACT_STATE_NAMES
}

fn invalid_transition(
    state: LoopState,
    trigger: LoopTrigger,
    missing_guard: Option<TransitionGuard>,
) -> AgentError {
    let guard_message = missing_guard
        .map(|guard| format!("; missing guard {guard:?}"))
        .unwrap_or_default();
    AgentError::new(
        AgentErrorKind::InvalidStateTransition,
        RetryClassification::RepairNeeded,
        format!(
            "loop transition from {:?} with trigger {:?} is not allowed{}",
            state, trigger, guard_message
        ),
    )
}

const CONTRACT_STATE_NAMES: &[&str] = &[
    "Starting",
    "ContextAssembly",
    "ProviderProjection",
    "ModelStreaming",
    "StreamIntervention",
    "ToolPlanning",
    "Approval",
    "ToolDenied",
    "ToolExecution",
    "Interrupted",
    "WaitingForResume",
    "Compaction",
    "Continue",
    "Recovery",
    "Completed",
    "Cancelled",
    "Failed",
];

const TRANSITION_TABLE: &[TransitionRule] = &[
    TransitionRule {
        from_state: LoopState::Starting,
        trigger: LoopTrigger::StartRun,
        guard: TransitionGuard::PackageValid,
        events: &[LoopEventKind::RunStarted],
        journal_records: &[JournalRecordKind::Run],
        checkpoint_policy: CheckpointPolicy::After,
        side_effect_policy: SideEffectPolicy::None,
        next_state: LoopState::ContextAssembly,
        terminal_result: None,
        recovery_classification: None,
    },
    TransitionRule {
        from_state: LoopState::ContextAssembly,
        trigger: LoopTrigger::ContextReady,
        guard: TransitionGuard::BudgetValid,
        events: &[LoopEventKind::ContextAssembled],
        journal_records: &[JournalRecordKind::Context],
        checkpoint_policy: CheckpointPolicy::After,
        side_effect_policy: SideEffectPolicy::None,
        next_state: LoopState::ProviderProjection,
        terminal_result: None,
        recovery_classification: None,
    },
    TransitionRule {
        from_state: LoopState::ProviderProjection,
        trigger: LoopTrigger::ProjectionReady,
        guard: TransitionGuard::PackageHashesMatch,
        events: &[
            LoopEventKind::ProviderRequestProjected,
            LoopEventKind::ModelAttemptStarted,
        ],
        journal_records: &[JournalRecordKind::Context, JournalRecordKind::ModelAttempt],
        checkpoint_policy: CheckpointPolicy::Before,
        side_effect_policy: SideEffectPolicy::IntentBeforeEffect,
        next_state: LoopState::ModelStreaming,
        terminal_result: None,
        recovery_classification: None,
    },
    TransitionRule {
        from_state: LoopState::ModelStreaming,
        trigger: LoopTrigger::ToolUse,
        guard: TransitionGuard::ModelMessageHasToolCalls,
        events: &[
            LoopEventKind::ModelMessageCompleted,
            LoopEventKind::ToolRequested,
        ],
        journal_records: &[JournalRecordKind::ModelAttempt, JournalRecordKind::Tool],
        checkpoint_policy: CheckpointPolicy::After,
        side_effect_policy: SideEffectPolicy::None,
        next_state: LoopState::ToolPlanning,
        terminal_result: None,
        recovery_classification: None,
    },
    TransitionRule {
        from_state: LoopState::ModelStreaming,
        trigger: LoopTrigger::StreamRuleMatch,
        guard: TransitionGuard::RuleActionAllowed,
        events: &[LoopEventKind::StreamRuleMatched],
        journal_records: &[JournalRecordKind::StreamRule],
        checkpoint_policy: CheckpointPolicy::None,
        side_effect_policy: SideEffectPolicy::None,
        next_state: LoopState::StreamIntervention,
        terminal_result: None,
        recovery_classification: None,
    },
    TransitionRule {
        from_state: LoopState::ModelStreaming,
        trigger: LoopTrigger::CompactionNeeded,
        guard: TransitionGuard::BudgetValid,
        events: &[LoopEventKind::RunCheckpointed],
        journal_records: &[JournalRecordKind::Context],
        checkpoint_policy: CheckpointPolicy::Before,
        side_effect_policy: SideEffectPolicy::None,
        next_state: LoopState::Compaction,
        terminal_result: None,
        recovery_classification: None,
    },
    TransitionRule {
        from_state: LoopState::ModelStreaming,
        trigger: LoopTrigger::EndTurn,
        guard: TransitionGuard::FinalMessageComplete,
        events: &[
            LoopEventKind::ModelMessageCompleted,
            LoopEventKind::RunCompleted,
        ],
        journal_records: &[
            JournalRecordKind::ModelAttempt,
            JournalRecordKind::Message,
            JournalRecordKind::Run,
        ],
        checkpoint_policy: CheckpointPolicy::Terminal,
        side_effect_policy: SideEffectPolicy::None,
        next_state: LoopState::Completed,
        terminal_result: Some(LoopTerminalResult {
            status: LoopTerminalStatus::Completed,
            stop_reason: LoopStopReason::EndTurn,
        }),
        recovery_classification: None,
    },
    TransitionRule {
        from_state: LoopState::ModelStreaming,
        trigger: LoopTrigger::ProviderFailure,
        guard: TransitionGuard::ProviderFailureClassified,
        events: &[LoopEventKind::ModelAttemptFailed, LoopEventKind::RunFailed],
        journal_records: &[
            JournalRecordKind::ModelAttempt,
            JournalRecordKind::Recovery,
            JournalRecordKind::Run,
        ],
        checkpoint_policy: CheckpointPolicy::Terminal,
        side_effect_policy: SideEffectPolicy::ReconcileRequired,
        next_state: LoopState::Failed,
        terminal_result: Some(LoopTerminalResult {
            status: LoopTerminalStatus::Failed,
            stop_reason: LoopStopReason::ProviderFailure,
        }),
        recovery_classification: None,
    },
    TransitionRule {
        from_state: LoopState::StreamIntervention,
        trigger: LoopTrigger::StreamStopRun,
        guard: TransitionGuard::RuleActionAllowed,
        events: &[
            LoopEventKind::StreamInterventionApplied,
            LoopEventKind::RunCompleted,
        ],
        journal_records: &[JournalRecordKind::StreamRule, JournalRecordKind::Run],
        checkpoint_policy: CheckpointPolicy::Terminal,
        side_effect_policy: SideEffectPolicy::None,
        next_state: LoopState::Completed,
        terminal_result: Some(LoopTerminalResult {
            status: LoopTerminalStatus::Completed,
            stop_reason: LoopStopReason::StreamRuleStop,
        }),
        recovery_classification: None,
    },
    TransitionRule {
        from_state: LoopState::StreamIntervention,
        trigger: LoopTrigger::StreamAbortAndRetry,
        guard: TransitionGuard::RuleActionAllowed,
        events: &[LoopEventKind::StreamInterventionApplied],
        journal_records: &[
            JournalRecordKind::StreamRule,
            JournalRecordKind::ModelAttempt,
        ],
        checkpoint_policy: CheckpointPolicy::After,
        side_effect_policy: SideEffectPolicy::IdempotentRetryAllowed,
        next_state: LoopState::ProviderProjection,
        terminal_result: None,
        recovery_classification: None,
    },
    TransitionRule {
        from_state: LoopState::StreamIntervention,
        trigger: LoopTrigger::StreamPauseForApproval,
        guard: TransitionGuard::RuleActionAllowed,
        events: &[
            LoopEventKind::StreamInterventionApplied,
            LoopEventKind::ApprovalRequested,
        ],
        journal_records: &[JournalRecordKind::StreamRule, JournalRecordKind::Approval],
        checkpoint_policy: CheckpointPolicy::After,
        side_effect_policy: SideEffectPolicy::None,
        next_state: LoopState::Approval,
        terminal_result: None,
        recovery_classification: None,
    },
    TransitionRule {
        from_state: LoopState::StreamIntervention,
        trigger: LoopTrigger::StreamUnsafeIntervention,
        guard: TransitionGuard::UnsafeIntervention,
        events: &[LoopEventKind::RunFailed],
        journal_records: &[JournalRecordKind::StreamRule, JournalRecordKind::Run],
        checkpoint_policy: CheckpointPolicy::Terminal,
        side_effect_policy: SideEffectPolicy::None,
        next_state: LoopState::Failed,
        terminal_result: Some(LoopTerminalResult {
            status: LoopTerminalStatus::Failed,
            stop_reason: LoopStopReason::UnsafeStreamIntervention,
        }),
        recovery_classification: None,
    },
    TransitionRule {
        from_state: LoopState::ToolPlanning,
        trigger: LoopTrigger::PolicyAllow,
        guard: TransitionGuard::PermissionsPass,
        events: &[LoopEventKind::ToolStarted],
        journal_records: &[JournalRecordKind::Tool],
        checkpoint_policy: CheckpointPolicy::Before,
        side_effect_policy: SideEffectPolicy::IntentBeforeEffect,
        next_state: LoopState::ToolExecution,
        terminal_result: None,
        recovery_classification: None,
    },
    TransitionRule {
        from_state: LoopState::ToolPlanning,
        trigger: LoopTrigger::PolicyAsk,
        guard: TransitionGuard::DispatcherAvailableOrEscalationConfigured,
        events: &[LoopEventKind::ApprovalRequested],
        journal_records: &[JournalRecordKind::Approval],
        checkpoint_policy: CheckpointPolicy::After,
        side_effect_policy: SideEffectPolicy::None,
        next_state: LoopState::Approval,
        terminal_result: None,
        recovery_classification: None,
    },
    TransitionRule {
        from_state: LoopState::ToolPlanning,
        trigger: LoopTrigger::PolicyDeny,
        guard: TransitionGuard::DeniedResultAllowed,
        events: &[
            LoopEventKind::ToolApprovalRequired,
            LoopEventKind::ApprovalDenied,
        ],
        journal_records: &[JournalRecordKind::Approval, JournalRecordKind::Tool],
        checkpoint_policy: CheckpointPolicy::After,
        side_effect_policy: SideEffectPolicy::None,
        next_state: LoopState::ToolDenied,
        terminal_result: None,
        recovery_classification: None,
    },
    TransitionRule {
        from_state: LoopState::Approval,
        trigger: LoopTrigger::Approved,
        guard: TransitionGuard::DecisionValid,
        events: &[LoopEventKind::ApprovalResponded, LoopEventKind::ToolStarted],
        journal_records: &[JournalRecordKind::Approval, JournalRecordKind::Tool],
        checkpoint_policy: CheckpointPolicy::Before,
        side_effect_policy: SideEffectPolicy::IntentBeforeEffect,
        next_state: LoopState::ToolExecution,
        terminal_result: None,
        recovery_classification: None,
    },
    TransitionRule {
        from_state: LoopState::Approval,
        trigger: LoopTrigger::ApprovalDenied,
        guard: TransitionGuard::DecisionValid,
        events: &[
            LoopEventKind::ApprovalResponded,
            LoopEventKind::ApprovalDenied,
        ],
        journal_records: &[JournalRecordKind::Approval],
        checkpoint_policy: CheckpointPolicy::After,
        side_effect_policy: SideEffectPolicy::None,
        next_state: LoopState::ToolDenied,
        terminal_result: None,
        recovery_classification: None,
    },
    TransitionRule {
        from_state: LoopState::Approval,
        trigger: LoopTrigger::ApprovalTimeout,
        guard: TransitionGuard::TimeoutElapsed,
        events: &[
            LoopEventKind::ApprovalTimedOut,
            LoopEventKind::ApprovalDenied,
        ],
        journal_records: &[JournalRecordKind::Approval],
        checkpoint_policy: CheckpointPolicy::After,
        side_effect_policy: SideEffectPolicy::None,
        next_state: LoopState::ToolDenied,
        terminal_result: None,
        recovery_classification: None,
    },
    TransitionRule {
        from_state: LoopState::Approval,
        trigger: LoopTrigger::ApprovalTransportFatal,
        guard: TransitionGuard::ApprovalTransportFatal,
        events: &[LoopEventKind::RunFailed],
        journal_records: &[JournalRecordKind::Approval, JournalRecordKind::Recovery],
        checkpoint_policy: CheckpointPolicy::After,
        side_effect_policy: SideEffectPolicy::ReconcileRequired,
        next_state: LoopState::Failed,
        terminal_result: Some(LoopTerminalResult {
            status: LoopTerminalStatus::Failed,
            stop_reason: LoopStopReason::ApprovalTransportFatal,
        }),
        recovery_classification: None,
    },
    TransitionRule {
        from_state: LoopState::Approval,
        trigger: LoopTrigger::StreamApprovalResumed,
        guard: TransitionGuard::DecisionValid,
        events: &[LoopEventKind::ApprovalResponded],
        journal_records: &[JournalRecordKind::Approval],
        checkpoint_policy: CheckpointPolicy::After,
        side_effect_policy: SideEffectPolicy::None,
        next_state: LoopState::ProviderProjection,
        terminal_result: None,
        recovery_classification: None,
    },
    TransitionRule {
        from_state: LoopState::ToolDenied,
        trigger: LoopTrigger::ContinueWithDeniedResult,
        guard: TransitionGuard::DeniedResultAllowed,
        events: &[LoopEventKind::ToolCompleted],
        journal_records: &[JournalRecordKind::Tool],
        checkpoint_policy: CheckpointPolicy::After,
        side_effect_policy: SideEffectPolicy::None,
        next_state: LoopState::Continue,
        terminal_result: None,
        recovery_classification: None,
    },
    TransitionRule {
        from_state: LoopState::ToolDenied,
        trigger: LoopTrigger::FailOnDenied,
        guard: TransitionGuard::DecisionValid,
        events: &[LoopEventKind::RunFailed],
        journal_records: &[JournalRecordKind::Recovery, JournalRecordKind::Run],
        checkpoint_policy: CheckpointPolicy::Terminal,
        side_effect_policy: SideEffectPolicy::None,
        next_state: LoopState::Failed,
        terminal_result: Some(LoopTerminalResult {
            status: LoopTerminalStatus::Failed,
            stop_reason: LoopStopReason::ToolDenied,
        }),
        recovery_classification: None,
    },
    TransitionRule {
        from_state: LoopState::ToolExecution,
        trigger: LoopTrigger::ToolComplete,
        guard: TransitionGuard::TerminalStatusAppended,
        events: &[LoopEventKind::ToolCompleted],
        journal_records: &[JournalRecordKind::Tool],
        checkpoint_policy: CheckpointPolicy::After,
        side_effect_policy: SideEffectPolicy::None,
        next_state: LoopState::Continue,
        terminal_result: None,
        recovery_classification: None,
    },
    TransitionRule {
        from_state: LoopState::ToolExecution,
        trigger: LoopTrigger::ToolInterrupt,
        guard: TransitionGuard::InterruptResumable,
        events: &[LoopEventKind::ToolInterrupted],
        journal_records: &[JournalRecordKind::Tool],
        checkpoint_policy: CheckpointPolicy::After,
        side_effect_policy: SideEffectPolicy::ReconcileRequired,
        next_state: LoopState::Interrupted,
        terminal_result: None,
        recovery_classification: None,
    },
    TransitionRule {
        from_state: LoopState::ToolExecution,
        trigger: LoopTrigger::ToolFailure,
        guard: TransitionGuard::TerminalStatusAppended,
        events: &[LoopEventKind::ToolFailed, LoopEventKind::RunFailed],
        journal_records: &[
            JournalRecordKind::Tool,
            JournalRecordKind::Recovery,
            JournalRecordKind::Run,
        ],
        checkpoint_policy: CheckpointPolicy::Terminal,
        side_effect_policy: SideEffectPolicy::ReconcileRequired,
        next_state: LoopState::Failed,
        terminal_result: Some(LoopTerminalResult {
            status: LoopTerminalStatus::Failed,
            stop_reason: LoopStopReason::ToolFailure,
        }),
        recovery_classification: None,
    },
    TransitionRule {
        from_state: LoopState::Interrupted,
        trigger: LoopTrigger::WaitForResume,
        guard: TransitionGuard::ResumeTokenRequired,
        events: &[LoopEventKind::RunCheckpointed],
        journal_records: &[JournalRecordKind::Recovery],
        checkpoint_policy: CheckpointPolicy::After,
        side_effect_policy: SideEffectPolicy::None,
        next_state: LoopState::WaitingForResume,
        terminal_result: None,
        recovery_classification: None,
    },
    TransitionRule {
        from_state: LoopState::WaitingForResume,
        trigger: LoopTrigger::ResumeAllowed,
        guard: TransitionGuard::CheckpointAndPackageValid,
        events: &[
            LoopEventKind::RunResumeRequested,
            LoopEventKind::ReplayCompleted,
        ],
        journal_records: &[JournalRecordKind::Recovery],
        checkpoint_policy: CheckpointPolicy::After,
        side_effect_policy: SideEffectPolicy::IdempotentRetryAllowed,
        next_state: LoopState::ToolExecution,
        terminal_result: None,
        recovery_classification: None,
    },
    TransitionRule {
        from_state: LoopState::WaitingForResume,
        trigger: LoopTrigger::ResumeDenied,
        guard: TransitionGuard::CheckpointAndPackageValid,
        events: &[LoopEventKind::RunResumeFailed, LoopEventKind::RunFailed],
        journal_records: &[JournalRecordKind::Recovery, JournalRecordKind::Run],
        checkpoint_policy: CheckpointPolicy::Terminal,
        side_effect_policy: SideEffectPolicy::None,
        next_state: LoopState::Failed,
        terminal_result: Some(LoopTerminalResult {
            status: LoopTerminalStatus::Failed,
            stop_reason: LoopStopReason::ResumeDenied,
        }),
        recovery_classification: None,
    },
    TransitionRule {
        from_state: LoopState::Compaction,
        trigger: LoopTrigger::CompactionComplete,
        guard: TransitionGuard::ProtectedContextPreserved,
        events: &[LoopEventKind::ContextCompactionCompleted],
        journal_records: &[JournalRecordKind::Context],
        checkpoint_policy: CheckpointPolicy::After,
        side_effect_policy: SideEffectPolicy::None,
        next_state: LoopState::ContextAssembly,
        terminal_result: None,
        recovery_classification: None,
    },
    TransitionRule {
        from_state: LoopState::Continue,
        trigger: LoopTrigger::ContinueLoop,
        guard: TransitionGuard::BudgetValid,
        events: &[],
        journal_records: &[],
        checkpoint_policy: CheckpointPolicy::None,
        side_effect_policy: SideEffectPolicy::None,
        next_state: LoopState::ContextAssembly,
        terminal_result: None,
        recovery_classification: None,
    },
    TransitionRule {
        from_state: LoopState::Continue,
        trigger: LoopTrigger::MaxIterationsReached {
            outcome: MaxIterationOutcome::Complete,
        },
        guard: TransitionGuard::MaxIterationBudgetExhausted,
        events: &[LoopEventKind::RunCompleted],
        journal_records: &[JournalRecordKind::Run],
        checkpoint_policy: CheckpointPolicy::Terminal,
        side_effect_policy: SideEffectPolicy::None,
        next_state: LoopState::Completed,
        terminal_result: Some(LoopTerminalResult {
            status: LoopTerminalStatus::Completed,
            stop_reason: LoopStopReason::MaxIterations {
                outcome: MaxIterationOutcome::Complete,
            },
        }),
        recovery_classification: None,
    },
    TransitionRule {
        from_state: LoopState::Continue,
        trigger: LoopTrigger::MaxIterationsReached {
            outcome: MaxIterationOutcome::Fail,
        },
        guard: TransitionGuard::MaxIterationBudgetExhausted,
        events: &[LoopEventKind::RunFailed],
        journal_records: &[JournalRecordKind::Run],
        checkpoint_policy: CheckpointPolicy::Terminal,
        side_effect_policy: SideEffectPolicy::None,
        next_state: LoopState::Failed,
        terminal_result: Some(LoopTerminalResult {
            status: LoopTerminalStatus::Failed,
            stop_reason: LoopStopReason::MaxIterations {
                outcome: MaxIterationOutcome::Fail,
            },
        }),
        recovery_classification: None,
    },
    TransitionRule {
        from_state: LoopState::Failed,
        trigger: LoopTrigger::FailureClassified {
            classification: RecoveryClassification::RetryableSafeStep,
        },
        guard: TransitionGuard::RepairPlanSafe,
        events: &[LoopEventKind::RecoveryPlanned],
        journal_records: &[JournalRecordKind::Recovery],
        checkpoint_policy: CheckpointPolicy::After,
        side_effect_policy: SideEffectPolicy::IdempotentRetryAllowed,
        next_state: LoopState::Recovery,
        terminal_result: None,
        recovery_classification: Some(RecoveryClassification::RetryableSafeStep),
    },
    TransitionRule {
        from_state: LoopState::Failed,
        trigger: LoopTrigger::FailureClassified {
            classification: RecoveryClassification::ReconcileRequired,
        },
        guard: TransitionGuard::RepairPlanSafe,
        events: &[LoopEventKind::RecoveryPlanned],
        journal_records: &[JournalRecordKind::Recovery],
        checkpoint_policy: CheckpointPolicy::After,
        side_effect_policy: SideEffectPolicy::ReconcileRequired,
        next_state: LoopState::Recovery,
        terminal_result: None,
        recovery_classification: Some(RecoveryClassification::ReconcileRequired),
    },
    TransitionRule {
        from_state: LoopState::Failed,
        trigger: LoopTrigger::FailureClassified {
            classification: RecoveryClassification::RepairRequired,
        },
        guard: TransitionGuard::RepairPlanSafe,
        events: &[LoopEventKind::RecoveryPlanned],
        journal_records: &[JournalRecordKind::Recovery],
        checkpoint_policy: CheckpointPolicy::After,
        side_effect_policy: SideEffectPolicy::ReconcileRequired,
        next_state: LoopState::Recovery,
        terminal_result: None,
        recovery_classification: Some(RecoveryClassification::RepairRequired),
    },
    TransitionRule {
        from_state: LoopState::Recovery,
        trigger: LoopTrigger::RepairApplied,
        guard: TransitionGuard::InvariantRestored,
        events: &[LoopEventKind::ReplayCompleted],
        journal_records: &[JournalRecordKind::Recovery],
        checkpoint_policy: CheckpointPolicy::After,
        side_effect_policy: SideEffectPolicy::None,
        next_state: LoopState::ContextAssembly,
        terminal_result: None,
        recovery_classification: None,
    },
    TransitionRule {
        from_state: LoopState::Recovery,
        trigger: LoopTrigger::RepairCompletedTerminal,
        guard: TransitionGuard::InvariantRestored,
        events: &[LoopEventKind::ReplayCompleted, LoopEventKind::RunCompleted],
        journal_records: &[JournalRecordKind::Recovery, JournalRecordKind::Run],
        checkpoint_policy: CheckpointPolicy::Terminal,
        side_effect_policy: SideEffectPolicy::None,
        next_state: LoopState::Completed,
        terminal_result: Some(LoopTerminalResult {
            status: LoopTerminalStatus::Completed,
            stop_reason: LoopStopReason::RecoveryCompleted,
        }),
        recovery_classification: None,
    },
    TransitionRule {
        from_state: LoopState::Recovery,
        trigger: LoopTrigger::RecoveryIrrecoverable,
        guard: TransitionGuard::RepairPlanSafe,
        events: &[LoopEventKind::RunFailed],
        journal_records: &[JournalRecordKind::Recovery, JournalRecordKind::Run],
        checkpoint_policy: CheckpointPolicy::Terminal,
        side_effect_policy: SideEffectPolicy::None,
        next_state: LoopState::Failed,
        terminal_result: Some(LoopTerminalResult {
            status: LoopTerminalStatus::Failed,
            stop_reason: LoopStopReason::RecoveryIrrecoverable,
        }),
        recovery_classification: Some(RecoveryClassification::Irrecoverable),
    },
    cancel_rule(LoopState::Starting),
    cancel_rule(LoopState::ContextAssembly),
    cancel_rule(LoopState::ProviderProjection),
    cancel_rule(LoopState::ModelStreaming),
    cancel_rule(LoopState::StreamIntervention),
    cancel_rule(LoopState::ToolPlanning),
    cancel_rule(LoopState::Approval),
    cancel_rule(LoopState::ToolDenied),
    cancel_rule(LoopState::ToolExecution),
    cancel_rule(LoopState::Interrupted),
    cancel_rule(LoopState::WaitingForResume),
    cancel_rule(LoopState::Compaction),
    cancel_rule(LoopState::Continue),
    cancel_rule(LoopState::Recovery),
    cancel_rule(LoopState::Failed),
];

const fn cancel_rule(from_state: LoopState) -> TransitionRule {
    TransitionRule {
        from_state,
        trigger: LoopTrigger::CancelRequested,
        guard: TransitionGuard::CancellationRequested,
        events: &[
            LoopEventKind::RunCancelRequested,
            LoopEventKind::RunCancelled,
        ],
        journal_records: &[JournalRecordKind::Recovery, JournalRecordKind::Run],
        checkpoint_policy: CheckpointPolicy::Terminal,
        side_effect_policy: SideEffectPolicy::ReconcileRequired,
        next_state: LoopState::Cancelled,
        terminal_result: Some(LoopTerminalResult {
            status: LoopTerminalStatus::Cancelled,
            stop_reason: LoopStopReason::Cancelled,
        }),
        recovery_classification: None,
    }
}
