use crate::{
    domain::{AgentError, AgentErrorKind, AgentId, DestinationRef, RunId, SourceRef},
    error::{CausalIds, RetryClassification},
    hook_ports::HookExecutorRegistry,
    hook_records::{HookMutationJournalPlan, HookRecord},
    journal::JournalCursor,
    journal_ports::RunJournal,
    package::RuntimePackageFingerprint,
    package_hooks::{
        HookCancellationToken, HookInput, HookPoint, HookResponse, HookResponseClass, HookSpec,
        HookView, ordered_hooks_for_point, validate_hook_specs,
    },
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HookLifecycleContext {
    pub run_id: RunId,
    pub agent_id: AgentId,
    pub turn_id: Option<crate::domain::TurnId>,
    pub attempt_id: Option<crate::domain::AttemptId>,
    pub source: SourceRef,
    pub destination: Option<DestinationRef>,
    pub package_fingerprint: RuntimePackageFingerprint,
    pub cancellation: HookCancellationToken,
}

impl HookLifecycleContext {
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

pub struct HookLifecycleCoordinator<'a, R, J>
where
    R: HookExecutorRegistry,
    J: RunJournal,
{
    registry: &'a R,
    journal: &'a J,
    next_journal_seq: u64,
}

impl<'a, R, J> HookLifecycleCoordinator<'a, R, J>
where
    R: HookExecutorRegistry,
    J: RunJournal,
{
    pub fn new(registry: &'a R, journal: &'a J, next_journal_seq: u64) -> Self {
        Self {
            registry,
            journal,
            next_journal_seq,
        }
    }

    pub fn validate_package_hooks(&self, specs: &[HookSpec]) -> Result<(), AgentError> {
        validate_package_hooks(specs, self.registry)
    }

    pub fn invoke_point(
        &mut self,
        specs: &[HookSpec],
        point: HookPoint,
        context: HookLifecycleContext,
        view: HookView,
    ) -> Result<Vec<HookInvocationOutcome>, AgentError> {
        let hooks = ordered_hooks_for_point(specs, point);
        let mut outcomes = Vec::with_capacity(hooks.len());
        for spec in hooks {
            outcomes.push(self.invoke_one(&spec, &context, view.clone())?);
        }
        Ok(outcomes)
    }

    fn invoke_one(
        &mut self,
        spec: &HookSpec,
        context: &HookLifecycleContext,
        view: HookView,
    ) -> Result<HookInvocationOutcome, AgentError> {
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

        self.handle_response(spec, context, invocation_id, execution.response)
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

    fn handle_response(
        &mut self,
        spec: &HookSpec,
        context: &HookLifecycleContext,
        invocation_id: String,
        response: HookResponse,
    ) -> Result<HookInvocationOutcome, AgentError> {
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
            journal_cursor: Some(terminal_cursor),
            journaled_before_apply: true,
            record: plan.hook_record,
        })
    }

    fn next_seq_block(&mut self, width: u64) -> u64 {
        let seq = self.next_journal_seq;
        self.next_journal_seq += width;
        seq
    }
}

pub fn validate_package_hooks<R>(specs: &[HookSpec], registry: &R) -> Result<(), AgentError>
where
    R: HookExecutorRegistry,
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
pub struct HookInvocationOutcome {
    pub hook_id: crate::package_hooks::HookId,
    pub status: HookInvocationStatus,
    pub response_class: Option<HookResponseClass>,
    pub journal_cursor: Option<JournalCursor>,
    pub journaled_before_apply: bool,
    pub record: HookRecord,
}

impl HookInvocationOutcome {
    fn from_record(spec: &HookSpec, status: HookInvocationStatus, record: HookRecord) -> Self {
        Self {
            hook_id: spec.hook_id.clone(),
            status,
            response_class: None,
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
            _ => crate::hook_records::HookResponseDecision::RejectedPolicy,
        };
        Self {
            hook_id: spec.hook_id.clone(),
            status,
            response_class: Some(response_class.clone()),
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
pub enum HookInvocationStatus {
    Completed,
    AppliedJournaledMutation,
    RejectedMutationRight,
    RejectedPointMatrix,
    TimedOutFailOpen,
    FailedOpen,
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
