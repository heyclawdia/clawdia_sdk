use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{
    error::{AgentError, AgentErrorKind, RetryClassification},
    journal::JournalRecordKind,
    recovery::RecoveryClassification,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LoopState {
    Starting,
    ContextAssembly,
    ProviderProjection,
    ModelStreaming,
    StreamIntervention,
    ToolPlanning,
    Approval,
    ToolDenied,
    ToolExecution,
    Interrupted,
    WaitingForResume,
    Compaction,
    Continue,
    Recovery,
    Completed,
    Cancelled,
    Failed,
}

impl LoopState {
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

    pub fn all() -> &'static [Self] {
        &Self::ALL
    }

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

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Cancelled)
    }

    pub fn can_carry_terminal_result(self) -> bool {
        matches!(self, Self::Completed | Self::Cancelled | Self::Failed)
    }

    pub fn requires_cancel_transition(self) -> bool {
        !self.is_terminal()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LoopTrigger {
    StartRun,
    ContextReady,
    ProjectionReady,
    ToolUse,
    StreamRuleMatch,
    EndTurn,
    ProviderFailure,
    MaxIterationsReached {
        outcome: MaxIterationOutcome,
    },
    CompactionNeeded,
    StreamStopRun,
    StreamAbortAndRetry,
    StreamPauseForApproval,
    StreamUnsafeIntervention,
    PolicyAllow,
    PolicyAsk,
    PolicyDeny,
    Approved,
    ApprovalDenied,
    ApprovalTimeout,
    ApprovalTransportFatal,
    StreamApprovalResumed,
    ContinueWithDeniedResult,
    FailOnDenied,
    ToolComplete,
    ToolInterrupt,
    ToolFailure,
    WaitForResume,
    ResumeAllowed,
    ResumeDenied,
    CompactionComplete,
    ContinueLoop,
    FailureClassified {
        classification: RecoveryClassification,
    },
    RepairApplied,
    RepairCompletedTerminal,
    RecoveryIrrecoverable,
    CancelRequested,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MaxIterationOutcome {
    Complete,
    Fail,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionGuard {
    None,
    PackageValid,
    BudgetValid,
    PackageHashesMatch,
    ModelMessageHasToolCalls,
    RuleActionAllowed,
    FinalMessageComplete,
    ProviderFailureClassified,
    PermissionsPass,
    DispatcherAvailableOrEscalationConfigured,
    DeniedResultAllowed,
    DecisionValid,
    TimeoutElapsed,
    ApprovalTransportFatal,
    TerminalStatusAppended,
    InterruptResumable,
    ResumeTokenRequired,
    CheckpointAndPackageValid,
    ProtectedContextPreserved,
    RepairPlanSafe,
    InvariantRestored,
    MaxIterationBudgetExhausted,
    CancellationRequested,
    UnsafeIntervention,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct TransitionGuardSet {
    #[serde(default)]
    satisfied: BTreeSet<TransitionGuard>,
}

impl TransitionGuardSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(mut self, guard: TransitionGuard) -> Self {
        self.satisfied.insert(guard);
        self
    }

    pub fn contains(&self, guard: TransitionGuard) -> bool {
        guard == TransitionGuard::None || self.satisfied.contains(&guard)
    }

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
pub enum CheckpointPolicy {
    None,
    Before,
    After,
    BeforeAndAfter,
    Terminal,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SideEffectPolicy {
    None,
    IntentBeforeEffect,
    IdempotentRetryAllowed,
    NonIdempotentFailClosed,
    ReconcileRequired,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LoopEventKind {
    RunStarted,
    RunCompleted,
    RunFailed,
    RunCancelled,
    RunCancelRequested,
    RunCheckpointed,
    RunResumeRequested,
    RunResumeFailed,
    ContextAssembled,
    ProviderRequestProjected,
    ModelAttemptStarted,
    ModelAttemptFailed,
    ModelMessageCompleted,
    ToolRequested,
    ToolApprovalRequired,
    ToolStarted,
    ToolCompleted,
    ToolFailed,
    ToolInterrupted,
    ApprovalRequested,
    ApprovalResponded,
    ApprovalTimedOut,
    ApprovalDenied,
    StreamRuleMatched,
    StreamInterventionApplied,
    ContextCompactionCompleted,
    RecoveryPlanned,
    ReplayCompleted,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LoopTerminalStatus {
    Completed,
    Cancelled,
    Failed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LoopStopReason {
    EndTurn,
    StreamRuleStop,
    MaxIterations { outcome: MaxIterationOutcome },
    Cancelled,
    ProviderFailure,
    UnsafeStreamIntervention,
    ToolDenied,
    ApprovalDenied,
    ApprovalTimeout,
    ApprovalTransportFatal,
    ToolFailure,
    ResumeDenied,
    RecoveryCompleted,
    RecoveryIrrecoverable,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct LoopTerminalResult {
    pub status: LoopTerminalStatus,
    pub stop_reason: LoopStopReason,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransitionRule {
    pub from_state: LoopState,
    pub trigger: LoopTrigger,
    pub guard: TransitionGuard,
    pub events: &'static [LoopEventKind],
    pub journal_records: &'static [JournalRecordKind],
    pub checkpoint_policy: CheckpointPolicy,
    pub side_effect_policy: SideEffectPolicy,
    pub next_state: LoopState,
    pub terminal_result: Option<LoopTerminalResult>,
    pub recovery_classification: Option<RecoveryClassification>,
}

impl TransitionRule {
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
pub struct TransitionInput {
    pub state: LoopState,
    pub trigger: LoopTrigger,
    #[serde(default)]
    pub guards: TransitionGuardSet,
}

impl TransitionInput {
    pub fn new(state: LoopState, trigger: LoopTrigger) -> Self {
        Self {
            state,
            trigger,
            guards: TransitionGuardSet::default(),
        }
    }

    pub fn with_guard(mut self, guard: TransitionGuard) -> Self {
        self.guards = self.guards.with(guard);
        self
    }

    pub fn with_guards(mut self, guards: TransitionGuardSet) -> Self {
        self.guards = guards;
        self
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TransitionOutput {
    pub from_state: LoopState,
    pub trigger: LoopTrigger,
    pub guard: TransitionGuard,
    pub events: Vec<LoopEventKind>,
    pub journal_records: Vec<JournalRecordKind>,
    pub checkpoint_policy: CheckpointPolicy,
    pub side_effect_policy: SideEffectPolicy,
    pub next_state: LoopState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_result: Option<LoopTerminalResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_classification: Option<RecoveryClassification>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AgentStateMachine;

impl AgentStateMachine {
    pub fn transition_table(&self) -> &'static [TransitionRule] {
        transition_table()
    }

    pub fn validate_transition(
        &self,
        input: TransitionInput,
    ) -> Result<TransitionOutput, AgentError> {
        validate_transition(input)
    }
}

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

pub fn transition_table() -> &'static [TransitionRule] {
    TRANSITION_TABLE
}

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
