//! Application-layer coordination over core primitives. Use these services to lower
//! helpers, drive runs, validate output, coordinate tools, approvals, delivery,
//! isolation, telemetry, and feature layers. Methods in this layer may call
//! configured ports, mutate in-memory stores, append journals, or publish events as
//! documented. This file contains the hooks portion of that contract.
//!
use crate::{
    domain::{AgentError, AgentErrorKind, AgentId, DestinationRef, RunId, SourceRef},
    error::{CausalIds, RetryClassification},
    hook_ports::HookExecutorRegistry,
    hook_records::{HookMutationJournalPlan, HookRecord, HookResponseDecision},
    journal::JournalCursor,
    journal_ports::RunJournal,
    package::RuntimePackageFingerprint,
    package_hooks::{
        HookCancellationToken, HookInput, HookPoint, HookResponse, HookResponseClass, HookSpec,
        HookView, ordered_hooks_for_point, validate_hook_specs,
    },
};

#[derive(Clone, Debug, Eq, PartialEq)]
/// Holds hook lifecycle context application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct HookLifecycleContext {
    /// Run identifier used for lineage, filtering, replay, and dedupe.
    pub run_id: RunId,
    /// Agent identifier used for lineage, filtering, and ownership checks.
    pub agent_id: AgentId,
    /// Turn identifier for one loop turn within a run.
    pub turn_id: Option<crate::domain::TurnId>,
    /// Attempt identifier for retry, repair, provider, or tool execution
    /// evidence.
    pub attempt_id: Option<crate::domain::AttemptId>,
    /// Source label or ref for this item; it is metadata and does not fetch
    /// content by itself.
    pub source: SourceRef,
    /// Destination label or ref for this item; it is metadata and does not
    /// deliver content by itself.
    pub destination: Option<DestinationRef>,
    /// Deterministic package fingerprint used for stale checks, package
    /// evidence, or replay comparisons.
    pub package_fingerprint: RuntimePackageFingerprint,
    /// Cancellation used by this record or request.
    pub cancellation: HookCancellationToken,
}

impl HookLifecycleContext {
    /// Creates a new application::hooks value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new(
        run_id: RunId,
        agent_id: AgentId,
        source: SourceRef,
        package_fingerprint: RuntimePackageFingerprint,
    ) -> Self {
        Self {
            run_id,
            agent_id,
            turn_id: None,
            attempt_id: None,
            source,
            destination: None,
            package_fingerprint,
            cancellation: HookCancellationToken::default(),
        }
    }
}

/// Holds hook lifecycle coordinator application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct HookLifecycleCoordinator<'a, R, J>
where
    R: HookExecutorRegistry + ?Sized,
    J: RunJournal + ?Sized,
{
    registry: &'a R,
    journal: &'a J,
    next_journal_seq: u64,
    sequence_allocator: Option<Box<dyn FnMut(u64) -> u64 + 'a>>,
}

impl<'a, R, J> HookLifecycleCoordinator<'a, R, J>
where
    R: HookExecutorRegistry + ?Sized,
    J: RunJournal + ?Sized,
{
    /// Creates a new application::hooks value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new(registry: &'a R, journal: &'a J, next_journal_seq: u64) -> Self {
        Self {
            registry,
            journal,
            next_journal_seq,
            sequence_allocator: None,
        }
    }

    /// Creates a coordinator with a caller-owned journal sequence allocator.
    /// The allocator is called only when this coordinator is about to append hook journal records.
    pub fn new_with_sequence_allocator<F>(
        registry: &'a R,
        journal: &'a J,
        next_journal_seq: u64,
        sequence_allocator: F,
    ) -> Self
    where
        F: FnMut(u64) -> u64 + 'a,
    {
        Self {
            registry,
            journal,
            next_journal_seq,
            sequence_allocator: Some(Box::new(sequence_allocator)),
        }
    }

    /// Validates the application::hooks invariants and returns a typed
    /// error on failure. Validation is pure and does not perform I/O,
    /// dispatch, journal appends, or adapter calls.
    pub fn validate_package_hooks(&self, specs: &[HookSpec]) -> Result<(), AgentError> {
        validate_package_hooks(specs, self.registry)
    }

    /// Returns the next journal sequence number this coordinator would use.
    /// Callers that share an external sequence allocator use this to synchronize after hook
    /// mutation records have been appended.
    pub fn next_journal_seq(&self) -> u64 {
        self.next_journal_seq
    }

    /// Invoke point.
    /// This invokes the configured hooks for one hook point and returns their responses; hook
    /// side effects stay behind the registered hook executors.
    pub fn invoke_point(
        &mut self,
        specs: &[HookSpec],
        point: HookPoint,
        context: HookLifecycleContext,
        view: HookView,
    ) -> Result<Vec<HookInvocationOutcome>, AgentError> {
        self.invoke_point_guarded(specs, point, context, view, |_, _| Ok(true))
    }

    /// Invoke point guarded.
    /// This invokes hooks for one point and lets the guarded domain reject behavior-changing
    /// responses before they are journaled as accepted.
    pub fn invoke_point_guarded<F>(
        &mut self,
        specs: &[HookSpec],
        point: HookPoint,
        context: HookLifecycleContext,
        view: HookView,
        mut acceptance_guard: F,
    ) -> Result<Vec<HookInvocationOutcome>, AgentError>
    where
        F: FnMut(&HookSpec, &HookResponse) -> Result<bool, AgentError>,
    {
        let hooks = ordered_hooks_for_point(specs, point);
        let mut outcomes = Vec::with_capacity(hooks.len());
        for spec in hooks {
            outcomes.push(self.invoke_one(&spec, &context, view.clone(), &mut acceptance_guard)?);
        }
        Ok(outcomes)
    }

    fn invoke_one<F>(
        &mut self,
        spec: &HookSpec,
        context: &HookLifecycleContext,
        view: HookView,
        acceptance_guard: &mut F,
    ) -> Result<HookInvocationOutcome, AgentError>
    where
        F: FnMut(&HookSpec, &HookResponse) -> Result<bool, AgentError>,
    {
        spec.validate()?;
        let invocation_id = format!("hook.invocation.{}", self.next_journal_seq);
        if context.cancellation.cancelled {
            return Ok(HookInvocationOutcome::from_record(
                spec,
                HookInvocationStatus::Cancelled,
                HookRecord::cancelled(spec, invocation_id),
            ));
        }

        let executor = self.registry.resolve(&spec.executor_ref).ok_or_else(|| {
            fail_closed_error(
                spec,
                context,
                AgentErrorKind::HostConfigurationNeeded,
                "missing hook executor ref",
            )
        })?;
        let input = HookInput {
            hook_id: spec.hook_id.clone(),
            point: spec.point.clone(),
            run_id: context.run_id.clone(),
            agent_id: context.agent_id.clone(),
            turn_id: context.turn_id.clone(),
            attempt_id: context.attempt_id.clone(),
            source: SourceRef::with_kind(crate::domain::SourceKind::Hook, spec.hook_id.as_str()),
            destination: context.destination.clone(),
            package_fingerprint: context.package_fingerprint.clone(),
            view,
            policy_refs: vec![spec.policy_ref.clone()],
            cancellation: context.cancellation.clone(),
        };

        let hook_result = executor.invoke(input);
        let execution = match hook_result {
            Ok(execution) => execution,
            Err(error) if !spec.is_security_relevant() && !spec.failure.fails_closed() => {
                return Ok(HookInvocationOutcome::from_record(
                    spec,
                    HookInvocationStatus::FailedOpen,
                    HookRecord::failed(spec, invocation_id, error.context().message),
                ));
            }
            Err(error) => {
                return Err(fail_closed_error(
                    spec,
                    context,
                    error.kind(),
                    error.context().message,
                ));
            }
        };

        if execution.elapsed_ms > spec.timeout.timeout_ms {
            return self.handle_timeout(spec, context, invocation_id, execution.elapsed_ms);
        }

        self.handle_response(
            spec,
            context,
            invocation_id,
            execution.response,
            acceptance_guard,
        )
    }

    fn handle_timeout(
        &self,
        spec: &HookSpec,
        context: &HookLifecycleContext,
        invocation_id: String,
        elapsed_ms: u64,
    ) -> Result<HookInvocationOutcome, AgentError> {
        if !spec.is_security_relevant() && !spec.failure.fails_closed() {
            return Ok(HookInvocationOutcome::from_record(
                spec,
                HookInvocationStatus::TimedOutFailOpen,
                HookRecord::timeout(spec, invocation_id, elapsed_ms),
            ));
        }
        Err(fail_closed_error(
            spec,
            context,
            AgentErrorKind::Timeout,
            "hook timed out before guarded lifecycle transition",
        ))
    }

    fn handle_response<F>(
        &mut self,
        spec: &HookSpec,
        context: &HookLifecycleContext,
        invocation_id: String,
        response: HookResponse,
        acceptance_guard: &mut F,
    ) -> Result<HookInvocationOutcome, AgentError>
    where
        F: FnMut(&HookSpec, &HookResponse) -> Result<bool, AgentError>,
    {
        let response_class = response.response_class();
        if !spec
            .point
            .allowed_response_classes()
            .contains(&response_class)
        {
            return Ok(HookInvocationOutcome::rejected(
                spec,
                invocation_id,
                response_class,
                HookInvocationStatus::RejectedPointMatrix,
            ));
        }
        if !spec
            .mutation_rights
            .allows_response_class(response_class.clone())
        {
            return Ok(HookInvocationOutcome::rejected(
                spec,
                invocation_id,
                response_class,
                HookInvocationStatus::RejectedMutationRight,
            ));
        }

        if !response.changes_behavior() {
            return Ok(HookInvocationOutcome::from_record(
                spec,
                HookInvocationStatus::Completed,
                HookRecord::completed(spec, invocation_id, 0),
            ));
        }
        match acceptance_guard(spec, &response) {
            Ok(true) => {}
            Ok(false) => {
                return self.append_rejected_response_decision(
                    spec,
                    context,
                    invocation_id,
                    response_class,
                    HookInvocationStatus::RejectedPolicy,
                );
            }
            Err(error) => {
                self.append_rejected_response_decision(
                    spec,
                    context,
                    invocation_id,
                    response_class,
                    HookInvocationStatus::RejectedPolicy,
                )?;
                return Err(error);
            }
        }

        let journal_seq = self.next_seq_block(3);
        let record_id = format!("journal.hook.{}.{}", spec.hook_id.as_str(), journal_seq);
        let plan = HookMutationJournalPlan::accepted_response(
            journal_seq,
            record_id,
            context.run_id.clone(),
            context.agent_id.clone(),
            context.source.clone(),
            spec,
            invocation_id.clone(),
            response_class.clone(),
            context.package_fingerprint.as_str(),
        );
        self.journal
            .append(plan.hook_journal_record.clone())
            .map_err(|error| {
                fail_closed_error(
                    spec,
                    context,
                    error.kind(),
                    "hook response journal append failed before apply",
                )
            })?;
        let _intent_cursor = self
            .journal
            .append(plan.intent_journal_record.clone())
            .map_err(|error| {
                fail_closed_error(
                    spec,
                    context,
                    error.kind(),
                    "hook mutation journal append failed before apply",
                )
            })?;
        let terminal_cursor = self
            .journal
            .append(plan.result_journal_record.clone())
            .map_err(|error| {
                fail_closed_error(
                    spec,
                    context,
                    error.kind(),
                    "hook mutation terminal result append failed before apply",
                )
            })?;
        Ok(HookInvocationOutcome {
            hook_id: spec.hook_id.clone(),
            status: HookInvocationStatus::AppliedJournaledMutation,
            response_class: Some(response_class),
            accepted_response: Some(response),
            journal_cursor: Some(terminal_cursor),
            journaled_before_apply: true,
            record: plan.hook_record,
        })
    }

    fn append_rejected_response_decision(
        &mut self,
        spec: &HookSpec,
        context: &HookLifecycleContext,
        invocation_id: String,
        response_class: HookResponseClass,
        status: HookInvocationStatus,
    ) -> Result<HookInvocationOutcome, AgentError> {
        let journal_seq = self.next_seq_block(1);
        let record_id = format!("journal.hook.{}.{}", spec.hook_id.as_str(), journal_seq);
        let (record, journal_record) = HookRecord::rejected_response_journal_record(
            journal_seq,
            record_id,
            context.run_id.clone(),
            context.agent_id.clone(),
            context.source.clone(),
            spec,
            invocation_id,
            HookResponseDecision::RejectedPolicy,
            response_class.clone(),
            context.package_fingerprint.as_str(),
        );
        let cursor = self.journal.append(journal_record).map_err(|error| {
            fail_closed_error(
                spec,
                context,
                error.kind(),
                "hook rejected response journal append failed before returning policy rejection",
            )
        })?;
        Ok(HookInvocationOutcome {
            hook_id: spec.hook_id.clone(),
            status,
            response_class: Some(response_class),
            accepted_response: None,
            journal_cursor: Some(cursor),
            journaled_before_apply: false,
            record,
        })
    }

    fn next_seq_block(&mut self, width: u64) -> u64 {
        let seq = if let Some(sequence_allocator) = &mut self.sequence_allocator {
            sequence_allocator(width)
        } else {
            self.next_journal_seq
        };
        self.next_journal_seq = seq.saturating_add(width);
        seq
    }
}

/// Validates the application::hooks invariants and returns a typed
/// error on failure. Validation is pure and does not perform I/O,
/// dispatch, journal appends, or adapter calls.
pub fn validate_package_hooks<R>(specs: &[HookSpec], registry: &R) -> Result<(), AgentError>
where
    R: HookExecutorRegistry + ?Sized,
{
    validate_hook_specs(specs)?;
    for spec in specs {
        if !registry.contains(&spec.executor_ref) {
            return Err(AgentError::new(
                AgentErrorKind::InvalidPackage,
                RetryClassification::HostConfigurationNeeded,
                format!(
                    "hook executor {} is not resolved before start_run",
                    spec.executor_ref.as_str()
                ),
            ));
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Holds hook invocation outcome application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct HookInvocationOutcome {
    /// Stable hook id used for typed lineage, lookup, or dedupe.
    pub hook_id: crate::package_hooks::HookId,
    /// Finite status for this record or lifecycle stage.
    pub status: HookInvocationStatus,
    /// Classification value for response class.
    /// Policy and projection paths use it for finite routing decisions.
    pub response_class: Option<HookResponseClass>,
    /// Accepted hook response for behavior-changing outcomes.
    /// Callers may lower this into the guarded domain operation after journal-before-apply
    /// succeeds.
    pub accepted_response: Option<HookResponse>,
    /// Cursor identifying a replay, export, or subscription position.
    /// Use it to resume without widening the original scope.
    pub journal_cursor: Option<JournalCursor>,
    /// Whether journaled before apply is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub journaled_before_apply: bool,
    /// Record used by this record or request.
    pub record: HookRecord,
}

impl HookInvocationOutcome {
    fn from_record(spec: &HookSpec, status: HookInvocationStatus, record: HookRecord) -> Self {
        Self {
            hook_id: spec.hook_id.clone(),
            status,
            response_class: None,
            accepted_response: None,
            journal_cursor: None,
            journaled_before_apply: false,
            record,
        }
    }

    fn rejected(
        spec: &HookSpec,
        invocation_id: String,
        response_class: HookResponseClass,
        status: HookInvocationStatus,
    ) -> Self {
        let decision = match status {
            HookInvocationStatus::RejectedMutationRight => {
                crate::hook_records::HookResponseDecision::RejectedMutationRight
            }
            HookInvocationStatus::RejectedPointMatrix => {
                crate::hook_records::HookResponseDecision::RejectedPointMatrix
            }
            HookInvocationStatus::RejectedPolicy => {
                crate::hook_records::HookResponseDecision::RejectedPolicy
            }
            _ => crate::hook_records::HookResponseDecision::RejectedPolicy,
        };
        Self {
            hook_id: spec.hook_id.clone(),
            status,
            response_class: Some(response_class.clone()),
            accepted_response: None,
            journal_cursor: None,
            journaled_before_apply: false,
            record: HookRecord::response_decision(
                spec,
                invocation_id,
                decision,
                response_class,
                Vec::new(),
            ),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Enumerates the finite hook invocation status cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum HookInvocationStatus {
    /// Use this variant when the contract needs to represent completed; selecting it has no side effect by itself.
    Completed,
    /// Use this variant when the contract needs to represent applied journaled mutation; selecting it has no side effect by itself.
    AppliedJournaledMutation,
    /// Use this variant when the contract needs to represent rejected mutation right; selecting it has no side effect by itself.
    RejectedMutationRight,
    /// Use this variant when the contract needs to represent rejected point matrix; selecting it has no side effect by itself.
    RejectedPointMatrix,
    /// Use this variant when the contract needs to represent rejected policy; selecting it has no side effect by itself.
    RejectedPolicy,
    /// Use this variant when the contract needs to represent timed out fail open; selecting it has no side effect by itself.
    TimedOutFailOpen,
    /// Use this variant when the contract needs to represent failed open; selecting it has no side effect by itself.
    FailedOpen,
    /// Use this variant when the contract needs to represent cancelled; selecting it has no side effect by itself.
    Cancelled,
}

fn fail_closed_error(
    spec: &HookSpec,
    context: &HookLifecycleContext,
    kind: AgentErrorKind,
    message: impl Into<String>,
) -> AgentError {
    let kind = match spec.failure {
        crate::package_hooks::HookFailurePolicy::Deny => AgentErrorKind::PolicyDenial,
        crate::package_hooks::HookFailurePolicy::InterruptRun
        | crate::package_hooks::HookFailurePolicy::FailRun => AgentErrorKind::HookFailure,
        crate::package_hooks::HookFailurePolicy::FailOpenObserveOnly => kind,
    };
    AgentError::new(kind, RetryClassification::RepairNeeded, message).with_causal_ids(CausalIds {
        run_id: Some(context.run_id.clone()),
        ..CausalIds::default()
    })
}
