use agent_sdk_core::{
    AgentErrorKind, AgentStateMachine, CheckpointPolicy, JournalRecordKind, LoopEventKind,
    LoopState, LoopStopReason, LoopTerminalResult, LoopTerminalStatus, LoopTrigger,
    MaxIterationOutcome, RecoveryAction, RecoveryClassification, RecoveryFailureKind,
    RetryClassification, SideEffectPolicy, TransitionGuard, TransitionGuardSet, TransitionInput,
    classify_recovery, contract_state_names, transition_table,
};

#[test]
fn state_table_allows_documented_transitions() {
    let machine = AgentStateMachine;

    for rule in transition_table() {
        let output = machine
            .validate_transition(
                TransitionInput::new(rule.from_state, rule.trigger)
                    .with_guards(TransitionGuardSet::for_rule(rule)),
            )
            .expect("documented transition should validate");

        assert_eq!(output.from_state, rule.from_state);
        assert_eq!(output.trigger, rule.trigger);
        assert_eq!(output.next_state, rule.next_state);
        assert_eq!(output.guard, rule.guard);
        assert_eq!(output.events, rule.events);
        assert_eq!(output.journal_records, rule.journal_records);
        assert_eq!(output.checkpoint_policy, rule.checkpoint_policy);
        assert_eq!(output.side_effect_policy, rule.side_effect_policy);
        assert_eq!(output.terminal_result, rule.terminal_result);
        assert_eq!(output.recovery_classification, rule.recovery_classification);
    }
}

#[test]
fn state_table_matches_contract_state_snapshot_and_loop_enum() {
    let enum_names: Vec<_> = LoopState::all()
        .iter()
        .map(|state| state.contract_name())
        .collect();

    assert_eq!(enum_names, contract_state_names());
    assert_eq!(
        contract_state_names(),
        [
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
        ]
    );
}

#[test]
fn all_nonterminal_states_have_cancel_transition() {
    for state in LoopState::all()
        .iter()
        .copied()
        .filter(|state| state.requires_cancel_transition())
    {
        let output = validate(
            state,
            LoopTrigger::CancelRequested,
            TransitionGuard::CancellationRequested,
        );

        assert_eq!(output.next_state, LoopState::Cancelled);
        assert_eq!(
            output.terminal_result,
            Some(LoopTerminalResult {
                status: LoopTerminalStatus::Cancelled,
                stop_reason: LoopStopReason::Cancelled,
            })
        );
        assert_eq!(output.checkpoint_policy, CheckpointPolicy::Terminal);
        assert_eq!(
            output.side_effect_policy,
            SideEffectPolicy::ReconcileRequired
        );
        assert!(output.events.contains(&LoopEventKind::RunCancelRequested));
        assert!(output.events.contains(&LoopEventKind::RunCancelled));
    }
}

#[test]
fn state_table_rejects_undocumented_transition() {
    let machine = AgentStateMachine;
    let error = machine
        .validate_transition(TransitionInput::new(
            LoopState::Starting,
            LoopTrigger::ToolUse,
        ))
        .expect_err("undocumented transition must fail closed");

    assert_eq!(error.kind(), AgentErrorKind::InvalidStateTransition);
    assert_eq!(error.retry(), RetryClassification::RepairNeeded);
}

#[test]
fn guards_are_required_and_typed() {
    let machine = AgentStateMachine;

    let error = machine
        .validate_transition(TransitionInput::new(
            LoopState::Starting,
            LoopTrigger::StartRun,
        ))
        .expect_err("missing typed package-valid guard must fail");

    assert_eq!(error.kind(), AgentErrorKind::InvalidStateTransition);

    let output = machine
        .validate_transition(
            TransitionInput::new(LoopState::Starting, LoopTrigger::StartRun)
                .with_guard(TransitionGuard::PackageValid),
        )
        .expect("typed guard satisfies transition");

    assert_eq!(output.next_state, LoopState::ContextAssembly);
}

#[test]
fn tool_denied_and_approval_paths_do_not_start_executor() {
    let policy_deny = validate(
        LoopState::ToolPlanning,
        LoopTrigger::PolicyDeny,
        TransitionGuard::DeniedResultAllowed,
    );

    assert_eq!(policy_deny.next_state, LoopState::ToolDenied);
    assert_eq!(
        policy_deny.events,
        vec![
            LoopEventKind::ToolApprovalRequired,
            LoopEventKind::ApprovalDenied,
        ]
    );
    assert!(!policy_deny.events.contains(&LoopEventKind::ToolStarted));
    assert_eq!(policy_deny.side_effect_policy, SideEffectPolicy::None);

    let timeout = validate(
        LoopState::Approval,
        LoopTrigger::ApprovalTimeout,
        TransitionGuard::TimeoutElapsed,
    );

    assert_eq!(timeout.next_state, LoopState::ToolDenied);
    assert_eq!(
        timeout.events,
        vec![
            LoopEventKind::ApprovalTimedOut,
            LoopEventKind::ApprovalDenied,
        ]
    );
    assert!(!timeout.events.contains(&LoopEventKind::ToolStarted));

    let continue_after_denial = validate(
        LoopState::ToolDenied,
        LoopTrigger::ContinueWithDeniedResult,
        TransitionGuard::DeniedResultAllowed,
    );
    assert_eq!(continue_after_denial.next_state, LoopState::Continue);

    let fail_after_denial = validate(
        LoopState::ToolDenied,
        LoopTrigger::FailOnDenied,
        TransitionGuard::DecisionValid,
    );
    assert_eq!(fail_after_denial.next_state, LoopState::Failed);
    assert_eq!(
        fail_after_denial.terminal_result,
        Some(LoopTerminalResult {
            status: LoopTerminalStatus::Failed,
            stop_reason: LoopStopReason::ToolDenied,
        })
    );
}

#[test]
fn cancel_from_model_stream_or_approval_never_starts_tool_execution() {
    let model_stream_cancel = validate(
        LoopState::ModelStreaming,
        LoopTrigger::CancelRequested,
        TransitionGuard::CancellationRequested,
    );

    assert_eq!(model_stream_cancel.next_state, LoopState::Cancelled);
    assert!(
        !model_stream_cancel
            .events
            .contains(&LoopEventKind::ToolStarted)
    );
    assert!(
        !model_stream_cancel
            .events
            .contains(&LoopEventKind::ToolCompleted)
    );
    assert_eq!(
        model_stream_cancel.side_effect_policy,
        SideEffectPolicy::ReconcileRequired
    );

    let approval_cancel = validate(
        LoopState::Approval,
        LoopTrigger::CancelRequested,
        TransitionGuard::CancellationRequested,
    );

    assert_eq!(approval_cancel.next_state, LoopState::Cancelled);
    assert!(!approval_cancel.events.contains(&LoopEventKind::ToolStarted));
    assert_eq!(
        approval_cancel.journal_records,
        vec![JournalRecordKind::Recovery, JournalRecordKind::Run]
    );
}

#[test]
fn max_iterations_returns_typed_stop_reason_by_policy() {
    let completed = validate(
        LoopState::Continue,
        LoopTrigger::MaxIterationsReached {
            outcome: MaxIterationOutcome::Complete,
        },
        TransitionGuard::MaxIterationBudgetExhausted,
    );

    assert_eq!(completed.next_state, LoopState::Completed);
    assert_eq!(
        completed.terminal_result,
        Some(LoopTerminalResult {
            status: LoopTerminalStatus::Completed,
            stop_reason: LoopStopReason::MaxIterations {
                outcome: MaxIterationOutcome::Complete,
            },
        })
    );

    let failed = validate(
        LoopState::Continue,
        LoopTrigger::MaxIterationsReached {
            outcome: MaxIterationOutcome::Fail,
        },
        TransitionGuard::MaxIterationBudgetExhausted,
    );

    assert_eq!(failed.next_state, LoopState::Failed);
    assert_eq!(
        failed.terminal_result,
        Some(LoopTerminalResult {
            status: LoopTerminalStatus::Failed,
            stop_reason: LoopStopReason::MaxIterations {
                outcome: MaxIterationOutcome::Fail,
            },
        })
    );
}

#[test]
fn stream_rule_abort_retries_with_new_attempt_boundary() {
    let output = validate(
        LoopState::StreamIntervention,
        LoopTrigger::StreamAbortAndRetry,
        TransitionGuard::RuleActionAllowed,
    );

    assert_eq!(output.next_state, LoopState::ProviderProjection);
    assert_eq!(
        output.side_effect_policy,
        SideEffectPolicy::IdempotentRetryAllowed
    );
    assert!(
        output
            .journal_records
            .contains(&JournalRecordKind::StreamRule)
    );
    assert!(
        output
            .journal_records
            .contains(&JournalRecordKind::ModelAttempt)
    );
}

#[test]
fn recovery_requires_repair_plan_before_mutation() {
    let machine = AgentStateMachine;
    let trigger = LoopTrigger::FailureClassified {
        classification: RecoveryClassification::RepairRequired,
    };

    let error = machine
        .validate_transition(TransitionInput::new(LoopState::Failed, trigger))
        .expect_err("recovery classification requires explicit repair-plan guard");
    assert_eq!(error.kind(), AgentErrorKind::InvalidStateTransition);

    let planned = machine
        .validate_transition(
            TransitionInput::new(LoopState::Failed, trigger)
                .with_guard(TransitionGuard::RepairPlanSafe),
        )
        .expect("safe repair plan can enter recovery");

    assert_eq!(planned.next_state, LoopState::Recovery);
    assert_eq!(
        planned.recovery_classification,
        Some(RecoveryClassification::RepairRequired)
    );
    assert_eq!(
        planned.side_effect_policy,
        SideEffectPolicy::ReconcileRequired
    );
    assert_eq!(planned.events, vec![LoopEventKind::RecoveryPlanned]);
    assert_eq!(planned.journal_records, vec![JournalRecordKind::Recovery]);

    let repaired = validate(
        LoopState::Recovery,
        LoopTrigger::RepairApplied,
        TransitionGuard::InvariantRestored,
    );
    assert_eq!(repaired.next_state, LoopState::ContextAssembly);
}

#[test]
fn recovery_classifier_names_retry_repair_and_reconcile_outcomes() {
    let retry = classify_recovery(RecoveryFailureKind::ProviderFailure);
    assert_eq!(
        retry.classification,
        RecoveryClassification::RetryableSafeStep
    );
    assert_eq!(retry.action, RecoveryAction::RetrySafeStep);
    assert!(retry.idempotent_retry_allowed);
    assert_eq!(retry.retry, RetryClassification::Retryable);

    let reconcile = classify_recovery(RecoveryFailureKind::JournalAppendAfterEffect);
    assert_eq!(
        reconcile.classification,
        RecoveryClassification::ReconcileRequired
    );
    assert_eq!(reconcile.action, RecoveryAction::ReconcilePendingSideEffect);
    assert!(reconcile.repair_plan_required);
    assert_eq!(reconcile.retry, RetryClassification::RepairNeeded);

    let host = classify_recovery(RecoveryFailureKind::HostPolicyRequired);
    assert_eq!(
        host.classification,
        RecoveryClassification::HostConfigurationRequired
    );
    assert_eq!(host.retry, RetryClassification::HostConfigurationNeeded);
}

#[test]
fn transition_validation_is_repeatable_and_side_effect_free() {
    let input = TransitionInput::new(LoopState::ProviderProjection, LoopTrigger::ProjectionReady)
        .with_guard(TransitionGuard::PackageHashesMatch);

    let first = AgentStateMachine
        .validate_transition(input.clone())
        .expect("first validation");
    let second = AgentStateMachine
        .validate_transition(input)
        .expect("second validation");

    assert_eq!(first, second);
    assert_eq!(first.next_state, LoopState::ModelStreaming);
    assert_eq!(
        first.side_effect_policy,
        SideEffectPolicy::IntentBeforeEffect
    );
}

#[test]
fn root_cargo_test_target_is_shim_only() {
    let shim = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/loop_state_contract.rs"
    ))
    .expect("root shim exists");

    assert!(shim.contains("#[path = \"runtime/loop_state_contract.rs\"]"));
    assert!(!shim.contains("#[test]"));
}

fn validate(
    state: LoopState,
    trigger: LoopTrigger,
    guard: TransitionGuard,
) -> agent_sdk_core::TransitionOutput {
    AgentStateMachine
        .validate_transition(TransitionInput::new(state, trigger).with_guard(guard))
        .expect("transition should validate")
}
