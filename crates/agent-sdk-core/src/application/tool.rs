//! Tool execution coordination over the shared effect spine. Use this module after
//! tool routing and policy have selected an executor. Execution may call tool
//! adapters and must keep intent/result records observable through journals and
//! events.
//!
use std::sync::Arc;

use crate::{
    approval::ApprovalBroker,
    approval_ports::ApprovalDispatcher,
    approval_records::ApprovalRequest,
    domain::{
        AgentError, AgentErrorKind, AgentId, ApprovalRequestId, EffectId, EntityKind, EntityRef,
        JournalCursor, PolicyKind, PolicyRef, PrivacyClass, RetryClassification, RunId, SessionId,
        SourceRef, TurnId,
    },
    effect::{EffectIntent, EffectKind, EffectResult, EffectTerminalStatus},
    hook_ports::HookExecutorRegistry,
    hooks::{
        HookInvocationOutcome, HookInvocationStatus, HookLifecycleContext, HookLifecycleCoordinator,
    },
    journal::{JournalRecord, JournalRecordBase, PendingSideEffect, RecoveryMarker},
    journal_ports::RunJournal,
    package::RuntimePackageFingerprint,
    package_hooks::{
        HookMutationRight, HookPoint, HookResponse, HookResponseClass, HookSpec, HookView,
    },
    policy::{
        ApprovalDecisionKind, DispatcherScope, MissingDependency, PolicyOutcome, PolicyStage,
    },
    tool_ports::{
        ResolvedToolCall, ToolCallRequest, ToolExecutionOutput, ToolExecutionRequest,
        ToolExecutionStrategy, ToolExecutorRegistry, ToolPolicyPort, ToolRouter,
    },
    tool_records::{ToolCallRecord, ToolCallRecordParams, tool_call_journal_record},
};

#[derive(Clone)]
/// Holds tool execution coordinator application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct ToolExecutionCoordinator {
    router: ToolRouter,
    executors: ToolExecutorRegistry,
    policy: Option<Arc<dyn ToolPolicyPort>>,
    approval_dispatcher: Option<Arc<dyn ApprovalDispatcher>>,
    strategy: ToolExecutionStrategy,
    hooks: Vec<HookSpec>,
    hook_registry: Option<Arc<dyn HookExecutorRegistry>>,
}

impl ToolExecutionCoordinator {
    /// Creates a new application::tool value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new(router: ToolRouter, executors: ToolExecutorRegistry) -> Self {
        Self {
            router,
            executors,
            policy: None,
            approval_dispatcher: None,
            strategy: ToolExecutionStrategy::default(),
            hooks: Vec::new(),
            hook_registry: None,
        }
    }

    /// Returns this value with its policy setting replaced. The method
    /// follows builder-style data construction and does not execute
    /// external work.
    pub fn with_policy(mut self, policy: Arc<dyn ToolPolicyPort>) -> Self {
        self.policy = Some(policy);
        self
    }

    /// Returns this coordinator with a host-owned approval dispatcher
    /// configured for high-risk approval-gated tools.
    pub fn with_approval_dispatcher(mut self, dispatcher: Arc<dyn ApprovalDispatcher>) -> Self {
        self.approval_dispatcher = Some(dispatcher);
        self
    }

    /// Returns this value with its strategy setting replaced. The
    /// method follows builder-style data construction and does not
    /// execute external work.
    pub fn with_strategy(mut self, strategy: ToolExecutionStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Returns this value with tool lifecycle hooks configured.
    /// This stores hook specs and a host-owned executor registry for future tool execution;
    /// it does not invoke hook or tool executors.
    pub fn with_hooks<R>(mut self, hooks: impl IntoIterator<Item = HookSpec>, registry: R) -> Self
    where
        R: HookExecutorRegistry + 'static,
    {
        self.hooks = hooks.into_iter().collect();
        self.hook_registry = Some(Arc::new(registry));
        self
    }

    /// Executes one tool request with journal intent/result gating.
    /// The executor may perform host work, but this coordinator appends the
    /// required journal records around that call and prevents execution if the
    /// intent record cannot be persisted.
    pub fn execute<J>(
        &self,
        journal: &J,
        request: ToolCallRequest,
        context: ToolExecutionContext,
    ) -> Result<ToolExecutionOutcome, AgentError>
    where
        J: RunJournal + ?Sized,
    {
        let mut next_journal_seq = context.next_journal_seq;
        validate_tool_hook_support(&self.hooks)?;
        self.preflight_tool_hooks(&context)?;
        let mut resolved = self.router.resolve(request)?;
        let initial_record = self.requested_record(&resolved, &context);

        if resolved.route.policy_refs.is_empty() {
            return Ok(ToolExecutionOutcome::denied(initial_record.with_denial(
                PolicyOutcome::fail_closed(PolicyStage::PreTool, MissingDependency::PolicyRef),
            )));
        }

        let Some(policy) = &self.policy else {
            return Ok(ToolExecutionOutcome::denied(initial_record.with_denial(
                PolicyOutcome::fail_closed(PolicyStage::PreTool, MissingDependency::PolicySnapshot),
            )));
        };

        if let Some(outcome) = self.invoke_before_tool_hooks(
            journal,
            &context,
            &mut next_journal_seq,
            &mut resolved,
            &initial_record,
        )? {
            return Ok(outcome);
        }
        let active_record = self.requested_record(&resolved, &context);

        let pre_tool = policy.evaluate_pre_tool(&resolved)?;
        if !pre_tool.is_allowed() {
            return Ok(ToolExecutionOutcome::denied(
                active_record.with_denial(pre_tool),
            ));
        }

        let mut journal_records_appended = 0_u64;
        if requires_approval_dispatch(&resolved) {
            let approval_request =
                approval_request_for_tool(&resolved, &context, next_journal_seq)?;
            let approval_outcome = ApprovalBroker::with_next_journal_seq(next_journal_seq)
                .request_approval(
                    approval_request,
                    self.approval_dispatcher.as_deref(),
                    journal,
                )?;
            journal_records_appended += 2;
            next_journal_seq = next_journal_seq.saturating_add(2);
            if !approval_outcome.releases_tool_execution() {
                return Ok(ToolExecutionOutcome {
                    record: active_record.with_denial(PolicyOutcome::fail_closed(
                        PolicyStage::PreTool,
                        MissingDependency::ApprovalDispatcher,
                    )),
                    intent_cursor: None,
                    terminal_cursor: None,
                    post_tool_policy: None,
                    recovery_required: false,
                    journal_records_appended,
                });
            }
        }

        let Some(executor_ref) = &resolved.route.executor_ref else {
            return Ok(ToolExecutionOutcome::denied(active_record.with_denial(
                PolicyOutcome::fail_closed(PolicyStage::PreTool, MissingDependency::ExecutorRef),
            )));
        };
        let Some(executor) = self.executors.get(executor_ref) else {
            return Ok(ToolExecutionOutcome::denied(active_record.with_denial(
                PolicyOutcome::fail_closed(PolicyStage::PreTool, MissingDependency::ExecutorRef),
            )));
        };

        let effect_id = context.effect_id(&resolved, None);
        let intent = self.effect_intent(effect_id.clone(), &resolved);
        let mut record = active_record.with_intent(intent.clone());
        let tool_subject = record.subject_ref();
        let intent_seq = reserve_journal_seq(&mut next_journal_seq, 1);
        let intent_record = tool_call_journal_record(
            context.record_base_at(
                intent_seq,
                "tool.intent",
                Some(resolved.route.destination.clone()),
            ),
            record.clone(),
            "tool_intent_recorded",
        );

        let intent_cursor = journal.append(intent_record).map_err(|error| {
            AgentError::new(
                AgentErrorKind::JournalFailure,
                RetryClassification::RepairNeeded,
                error.context().message,
            )
            .with_subject(tool_subject)
        })?;

        let request = ToolExecutionRequest {
            resolved_call: resolved.clone(),
            effect_intent: intent.clone(),
            strategy: self.strategy.clone(),
        };
        let output = match executor.execute(&request) {
            Ok(output) => output,
            Err(error) => ToolExecutionOutput::failed(
                "tool executor failed before returning a terminal envelope",
                format!("{:?}", error.kind()),
            ),
        };
        let result = output.to_effect_result(effect_id.clone());
        let terminal_record = record.clone().with_result(
            result.clone(),
            policy_outcome(
                PolicyStage::PostTool,
                resolved.request.source.clone(),
                resolved.route.destination.clone(),
                resolved.route.policy_refs.clone(),
            ),
        );
        let result_seq = reserve_journal_seq(&mut next_journal_seq, 1);
        let result_record = tool_call_journal_record(
            context.record_base_at(
                result_seq,
                "tool.result",
                Some(resolved.route.destination.clone()),
            ),
            terminal_record,
            "tool_result_recorded",
        );

        match journal.append(result_record) {
            Ok(cursor) => {
                let after_tool = self.invoke_after_tool_hooks(
                    journal,
                    &context,
                    &mut next_journal_seq,
                    &resolved,
                    &record,
                    output,
                )?;
                let post_tool = policy.evaluate_post_tool(&resolved, &after_tool.output)?;
                let final_result = after_tool.output.to_effect_result(effect_id.clone());
                let terminal_cursor = after_tool.terminal_cursor.unwrap_or(cursor);
                record = record.with_result(final_result, post_tool.clone());
                Ok(ToolExecutionOutcome {
                    record,
                    intent_cursor: Some(intent_cursor),
                    terminal_cursor: Some(terminal_cursor),
                    post_tool_policy: Some(post_tool),
                    recovery_required: false,
                    journal_records_appended: journal_records_appended + 2,
                })
            }
            Err(result_error) => {
                let unsafe_pending_reason =
                    unsafe_pending_reason(&resolved, &result, &resolved.route.policy_refs);
                let recovery = RecoveryMarker {
                    unsafe_pending: vec![PendingSideEffect {
                        effect_id,
                        intent_record_id: context.record_id("tool.intent"),
                        idempotency_key: resolved.request.idempotency_key.clone(),
                        dedupe_key: resolved.request.dedupe_key.clone(),
                        unsafe_pending_reason: unsafe_pending_reason.clone(),
                    }],
                    recovery_reason: format!(
                        "tool terminal result append failed: {}",
                        result_error.context().message
                    ),
                    policy_refs: resolved.route.policy_refs.clone(),
                };
                let recovery_record = JournalRecord::recovery(
                    context.record_base_at(
                        result_seq,
                        "tool.recovery",
                        Some(resolved.route.destination),
                    ),
                    recovery,
                );
                let cursor = journal.append(recovery_record).map_err(|recovery_error| {
                    AgentError::new(
                        AgentErrorKind::RecoveryRepairNeeded,
                        RetryClassification::RepairNeeded,
                        format!(
                            "tool result append failed and recovery append failed: {}; recovery: {}",
                            result_error.context().message,
                            recovery_error.context().message
                        ),
                    )
                    .with_subject(record.subject_ref())
                })?;
                record = record.with_recovery_required(result, unsafe_pending_reason);
                Ok(ToolExecutionOutcome {
                    record,
                    intent_cursor: Some(intent_cursor),
                    terminal_cursor: Some(cursor),
                    post_tool_policy: None,
                    recovery_required: true,
                    journal_records_appended: journal_records_appended + 2,
                })
            }
        }
    }

    fn invoke_before_tool_hooks<J>(
        &self,
        journal: &J,
        context: &ToolExecutionContext,
        next_journal_seq: &mut u64,
        resolved: &mut ResolvedToolCall,
        initial_record: &ToolCallRecord,
    ) -> Result<Option<ToolExecutionOutcome>, AgentError>
    where
        J: RunJournal + ?Sized,
    {
        let outcomes = self.invoke_tool_hook_point_guarded(
            journal,
            context,
            next_journal_seq,
            HookPoint::BeforeToolCall,
            before_tool_hook_view(resolved),
            |_, response| validate_tool_hook_response_bounds(context, response),
        )?;
        fail_on_rejected_tool_hook_mutation(&outcomes, context)?;

        for outcome in outcomes {
            match outcome.accepted_response {
                Some(HookResponse::Deny(reason)) => {
                    let effect_id = context.hook_tool_effect_id(resolved, "denied");
                    let result = EffectResult {
                        effect_id,
                        terminal_status: EffectTerminalStatus::DeniedBeforeExecution,
                        external_operation_id: None,
                        reconciliation_ref: None,
                        error_ref: Some(reason.code),
                        content_refs: Vec::new(),
                        redacted_summary: reason.redacted_summary,
                    };
                    let denied_record = initial_record.clone().with_hook_denial(
                        outcome.hook_id,
                        result,
                        hook_policy_denied_outcome(
                            PolicyStage::PreTool,
                            resolved.request.source.clone(),
                            resolved.route.destination.clone(),
                            resolved.route.policy_refs.clone(),
                        ),
                    );
                    let denied_seq = reserve_journal_seq(next_journal_seq, 1);
                    let denied_journal_record = tool_call_journal_record(
                        context.record_base_at(
                            denied_seq,
                            "tool.denied_by_hook",
                            Some(resolved.route.destination.clone()),
                        ),
                        denied_record.clone(),
                        "tool_denied_by_hook",
                    );
                    let cursor = journal.append(denied_journal_record).map_err(|error| {
                        AgentError::new(
                            AgentErrorKind::JournalFailure,
                            RetryClassification::RepairNeeded,
                            error.context().message,
                        )
                        .with_subject(denied_record.subject_ref())
                    })?;
                    return Ok(Some(ToolExecutionOutcome {
                        record: denied_record,
                        intent_cursor: None,
                        terminal_cursor: Some(cursor),
                        post_tool_policy: None,
                        recovery_required: false,
                        journal_records_appended: 1,
                    }));
                }
                Some(HookResponse::ModifyToolRequest(patch)) => {
                    let original_summary = resolved.request.redacted_args_summary.clone();
                    let patched_summary = patch.redacted_summary.clone();
                    let mut patched_resolved = resolved.clone();
                    patched_resolved.request.redacted_args_summary = patched_summary.clone();
                    let modified_record = self
                        .requested_record(&patched_resolved, context)
                        .with_hook_request_modification(
                            outcome.hook_id,
                            original_summary,
                            patched_summary.clone(),
                        );
                    let modified_seq = reserve_journal_seq(next_journal_seq, 1);
                    let modified_journal_record = tool_call_journal_record(
                        context.record_base_at(
                            modified_seq,
                            "tool.request_modified",
                            Some(resolved.route.destination.clone()),
                        ),
                        modified_record,
                        "tool_request_modified",
                    );
                    journal.append(modified_journal_record).map_err(|error| {
                        AgentError::new(
                            AgentErrorKind::JournalFailure,
                            RetryClassification::RepairNeeded,
                            error.context().message,
                        )
                        .with_subject(initial_record.subject_ref())
                    })?;
                    resolved.request.redacted_args_summary = patched_summary;
                }
                _ => {}
            }
        }
        Ok(None)
    }

    fn invoke_after_tool_hooks<J>(
        &self,
        journal: &J,
        context: &ToolExecutionContext,
        next_journal_seq: &mut u64,
        resolved: &ResolvedToolCall,
        intent_record: &ToolCallRecord,
        output: ToolExecutionOutput,
    ) -> Result<AfterToolHookOutcome, AgentError>
    where
        J: RunJournal + ?Sized,
    {
        let mut final_output = output;
        let mut terminal_cursor = None;
        let outcomes = self.invoke_tool_hook_point_guarded(
            journal,
            context,
            next_journal_seq,
            HookPoint::AfterToolCall,
            after_tool_hook_view(resolved, &final_output),
            |_, response| validate_tool_hook_response_bounds(context, response),
        )?;
        fail_on_rejected_tool_hook_mutation(&outcomes, context)?;

        for outcome in outcomes {
            if let Some(HookResponse::RewriteToolResult(patch)) = outcome.accepted_response {
                let original_summary = final_output.redacted_summary.clone();
                let mut rewritten_output = final_output.clone();
                rewritten_output.redacted_summary = patch.redacted_summary.clone();
                let rewritten_result =
                    rewritten_output.to_effect_result(context.effect_id(resolved, None));
                let rewritten_record = intent_record.clone().with_hook_result_rewrite(
                    outcome.hook_id,
                    rewritten_result,
                    original_summary,
                    patch.redacted_summary,
                );
                let rewrite_seq = reserve_journal_seq(next_journal_seq, 1);
                let rewrite_journal_record = tool_call_journal_record(
                    context.record_base_at(
                        rewrite_seq,
                        "tool.result_rewritten",
                        Some(resolved.route.destination.clone()),
                    ),
                    rewritten_record,
                    "tool_result_rewritten",
                );
                let cursor = journal.append(rewrite_journal_record).map_err(|error| {
                    AgentError::new(
                        AgentErrorKind::JournalFailure,
                        RetryClassification::RepairNeeded,
                        error.context().message,
                    )
                    .with_subject(intent_record.subject_ref())
                })?;
                terminal_cursor = Some(cursor);
                final_output = rewritten_output;
            }
        }
        Ok(AfterToolHookOutcome {
            output: final_output,
            terminal_cursor,
        })
    }

    fn preflight_tool_hooks(&self, context: &ToolExecutionContext) -> Result<(), AgentError> {
        let active_tool_hooks = self
            .hooks
            .iter()
            .filter(|spec| {
                matches!(
                    spec.point,
                    HookPoint::BeforeToolCall | HookPoint::AfterToolCall
                )
            })
            .cloned()
            .collect::<Vec<_>>();
        if active_tool_hooks.is_empty() {
            return Ok(());
        }
        let registry = self.hook_registry.as_ref().ok_or_else(|| {
            tool_hook_error(
                context,
                AgentErrorKind::InvalidPackage,
                "tool lifecycle hooks require a hook executor registry",
            )
        })?;
        crate::hooks::validate_package_hooks(&active_tool_hooks, registry.as_ref())
    }

    fn invoke_tool_hook_point_guarded<J, F>(
        &self,
        journal: &J,
        context: &ToolExecutionContext,
        next_journal_seq: &mut u64,
        point: HookPoint,
        view: HookView,
        mut acceptance_guard: F,
    ) -> Result<Vec<HookInvocationOutcome>, AgentError>
    where
        J: RunJournal + ?Sized,
        F: FnMut(&HookSpec, &HookResponse) -> Result<(), AgentError>,
    {
        let point_hooks = self
            .hooks
            .iter()
            .filter(|spec| spec.point == point)
            .cloned()
            .collect::<Vec<_>>();
        if point_hooks.is_empty() {
            return Ok(Vec::new());
        }
        let registry = self.hook_registry.as_ref().ok_or_else(|| {
            tool_hook_error(
                context,
                AgentErrorKind::InvalidPackage,
                "tool lifecycle hooks require a hook executor registry",
            )
        })?;
        crate::hooks::validate_package_hooks(&point_hooks, registry.as_ref())?;

        let mut hook_context = HookLifecycleContext::new(
            context.run_id.clone(),
            context.agent_id.clone(),
            context.source.clone(),
            RuntimePackageFingerprint(context.runtime_package_fingerprint.clone()),
        );
        hook_context.session_id = context.session_id.clone();
        hook_context.turn_id = context.turn_id.clone();
        let mut coordinator = HookLifecycleCoordinator::new_with_sequence_allocator(
            registry.as_ref(),
            journal,
            *next_journal_seq,
            |width| reserve_journal_seq(next_journal_seq, width),
        );
        coordinator.invoke_point_guarded(
            &point_hooks,
            point,
            hook_context,
            view,
            |spec, response| {
                acceptance_guard(spec, response)?;
                Ok(true)
            },
        )
    }

    fn requested_record(
        &self,
        resolved: &ResolvedToolCall,
        context: &ToolExecutionContext,
    ) -> ToolCallRecord {
        ToolCallRecord::requested(ToolCallRecordParams {
            tool_call_id: resolved.request.tool_call_id.clone(),
            run_id: context.run_id.clone(),
            turn_id: context.turn_id.clone(),
            capability_id: resolved.route.capability_id.clone(),
            canonical_tool_name: resolved.route.canonical_tool_name.clone(),
            namespace: resolved.route.namespace.clone(),
            source: resolved.request.source.clone(),
            destination: resolved.route.destination.clone(),
            executor_ref: resolved.route.executor_ref.clone(),
            policy_refs: resolved.route.policy_refs.clone(),
            sidecar_refs: resolved.route.sidecar_refs.clone(),
            effect_class: resolved.route.effect_class.clone(),
            risk_class: resolved.route.risk_class.clone(),
            privacy: resolved.route.privacy,
            retention: resolved.route.retention,
            requested_args_refs: resolved.request.requested_args_refs.clone(),
            redacted_args_summary: resolved.request.redacted_args_summary.clone(),
            idempotency_key: resolved.request.idempotency_key.clone(),
        })
    }

    fn effect_intent(&self, effect_id: EffectId, resolved: &ResolvedToolCall) -> EffectIntent {
        let mut intent = EffectIntent::new(
            effect_id,
            EffectKind::ToolExecution,
            EntityRef::new(EntityKind::ToolCall, resolved.request.tool_call_id.clone()),
            resolved.request.source.clone(),
            format!(
                "execute tool {} with redacted arguments",
                resolved.route.canonical_tool_name.as_str()
            ),
        );
        intent.destination = Some(resolved.route.destination.clone());
        intent.policy_refs = resolved.route.policy_refs.clone();
        intent.idempotency_key = resolved.request.idempotency_key.clone();
        intent.dedupe_key = resolved.request.dedupe_key.clone();
        intent.content_refs = resolved.request.requested_args_refs.clone();
        intent
    }
}

const TOOL_HOOK_MAX_REDACTED_SUMMARY_CHARS: usize = 2048;

#[derive(Clone, Debug)]
struct AfterToolHookOutcome {
    output: ToolExecutionOutput,
    terminal_cursor: Option<JournalCursor>,
}

fn validate_tool_hook_support(specs: &[HookSpec]) -> Result<(), AgentError> {
    let mut before_tool_total_hooks = 0_usize;
    let mut before_tool_mutating_hooks = 0_usize;
    let mut after_tool_total_hooks = 0_usize;
    let mut after_tool_mutating_hooks = 0_usize;
    for spec in specs {
        match spec.point {
            HookPoint::BeforeToolCall => {
                before_tool_total_hooks += 1;
                if spec.mutation_rights.can_change_behavior() {
                    before_tool_mutating_hooks += 1;
                }
                validate_supported_tool_hook_rights(
                    spec,
                    &[
                        HookMutationRight::Observe,
                        HookMutationRight::Deny,
                        HookMutationRight::ModifyToolRequest,
                    ],
                )?;
            }
            HookPoint::AfterToolCall => {
                after_tool_total_hooks += 1;
                if spec.mutation_rights.can_change_behavior() {
                    after_tool_mutating_hooks += 1;
                }
                validate_supported_tool_hook_rights(
                    spec,
                    &[
                        HookMutationRight::Observe,
                        HookMutationRight::RewriteToolResult,
                    ],
                )?;
            }
            _ => {}
        }
    }
    if before_tool_mutating_hooks > 0 && before_tool_total_hooks > 1 {
        return Err(AgentError::new(
            AgentErrorKind::InvalidPackage,
            RetryClassification::HostConfigurationNeeded,
            "tool execution supports behavior-changing BeforeToolCall hooks only when they are the only hook at that point",
        ));
    }
    if after_tool_mutating_hooks > 0 && after_tool_total_hooks > 1 {
        return Err(AgentError::new(
            AgentErrorKind::InvalidPackage,
            RetryClassification::HostConfigurationNeeded,
            "tool execution supports behavior-changing AfterToolCall hooks only when they are the only hook at that point",
        ));
    }
    Ok(())
}

fn validate_supported_tool_hook_rights(
    spec: &HookSpec,
    supported: &[HookMutationRight],
) -> Result<(), AgentError> {
    if let Some(right) = spec
        .mutation_rights
        .rights
        .iter()
        .find(|right| !supported.contains(right))
    {
        return Err(AgentError::new(
            AgentErrorKind::InvalidPackage,
            RetryClassification::HostConfigurationNeeded,
            format!(
                "tool execution does not lower {:?} hooks at {:?}",
                right, spec.point
            ),
        ));
    }
    Ok(())
}

fn validate_tool_hook_response_bounds(
    context: &ToolExecutionContext,
    response: &HookResponse,
) -> Result<(), AgentError> {
    match response {
        HookResponse::Deny(reason) => validate_tool_hook_summary_bound(
            context,
            "tool hook deny reason",
            &reason.redacted_summary,
        ),
        HookResponse::ModifyToolRequest(patch) => validate_tool_hook_summary_bound(
            context,
            "tool hook request patch",
            &patch.redacted_summary,
        ),
        HookResponse::RewriteToolResult(patch) => validate_tool_hook_summary_bound(
            context,
            "tool hook result patch",
            &patch.redacted_summary,
        ),
        _ => Ok(()),
    }
}

fn validate_tool_hook_summary_bound(
    context: &ToolExecutionContext,
    label: &str,
    summary: &str,
) -> Result<(), AgentError> {
    if summary.chars().count() > TOOL_HOOK_MAX_REDACTED_SUMMARY_CHARS {
        return Err(tool_hook_error(
            context,
            AgentErrorKind::PolicyDenial,
            format!("{label} exceeds tool hook redacted summary bound"),
        ));
    }
    Ok(())
}

fn fail_on_rejected_tool_hook_mutation(
    outcomes: &[HookInvocationOutcome],
    context: &ToolExecutionContext,
) -> Result<(), AgentError> {
    for outcome in outcomes {
        if matches!(
            outcome.status,
            HookInvocationStatus::RejectedMutationRight
                | HookInvocationStatus::RejectedPointMatrix
                | HookInvocationStatus::RejectedPolicy
        ) && outcome
            .response_class
            .as_ref()
            .is_some_and(HookResponseClass::changes_behavior)
        {
            return Err(tool_hook_error(
                context,
                AgentErrorKind::PolicyDenial,
                "tool hook behavior-changing response was rejected before apply",
            ));
        }
    }
    Ok(())
}

fn before_tool_hook_view(resolved: &ResolvedToolCall) -> HookView {
    let mut view = HookView::redacted(bounded_tool_hook_view_summary(format!(
        "before tool call: {}",
        resolved.request.redacted_args_summary
    )));
    view.subject_refs = vec![tool_call_entity_ref(resolved)];
    view.content_refs = resolved.request.requested_args_refs.clone();
    view
}

fn after_tool_hook_view(resolved: &ResolvedToolCall, output: &ToolExecutionOutput) -> HookView {
    let mut view = HookView::redacted(bounded_tool_hook_view_summary(format!(
        "after tool call: {}",
        output.redacted_summary
    )));
    view.subject_refs = vec![tool_call_entity_ref(resolved)];
    view.content_refs = output.content_refs.clone();
    view
}

fn bounded_tool_hook_view_summary(summary: impl Into<String>) -> String {
    let summary = summary.into();
    let summary_chars = summary.chars().count();
    if summary_chars <= TOOL_HOOK_MAX_REDACTED_SUMMARY_CHARS {
        return summary;
    }
    let suffix = format!(
        " [truncated; original_chars={summary_chars}; limit={TOOL_HOOK_MAX_REDACTED_SUMMARY_CHARS}]"
    );
    let suffix_chars = suffix.chars().count();
    let prefix_limit = TOOL_HOOK_MAX_REDACTED_SUMMARY_CHARS.saturating_sub(suffix_chars);
    let mut bounded = summary.chars().take(prefix_limit).collect::<String>();
    bounded.push_str(&suffix);
    bounded
}

fn tool_call_entity_ref(resolved: &ResolvedToolCall) -> EntityRef {
    EntityRef::new(EntityKind::ToolCall, resolved.request.tool_call_id.clone())
}

fn reserve_journal_seq(next_journal_seq: &mut u64, width: u64) -> u64 {
    let seq = *next_journal_seq;
    *next_journal_seq = seq.saturating_add(width);
    seq
}

fn hook_policy_denied_outcome(
    stage: PolicyStage,
    source: SourceRef,
    destination: crate::domain::DestinationRef,
    policy_refs: Vec<PolicyRef>,
) -> PolicyOutcome {
    PolicyOutcome {
        stage,
        decision: crate::policy::PolicyDecision::deny("tool.hook.denied"),
        subject: None,
        source: Some(source),
        destination: Some(destination),
        policy_refs,
        privacy: PrivacyClass::Internal,
        retention: crate::domain::RetentionClass::RunScoped,
    }
}

fn tool_hook_error(
    context: &ToolExecutionContext,
    kind: AgentErrorKind,
    message: impl Into<String>,
) -> AgentError {
    AgentError::new(kind, RetryClassification::RepairNeeded, message).with_causal_ids(
        crate::error::CausalIds {
            run_id: Some(context.run_id.clone()),
            ..crate::error::CausalIds::default()
        },
    )
}

fn policy_outcome(
    stage: PolicyStage,
    source: SourceRef,
    destination: crate::domain::DestinationRef,
    policy_refs: Vec<PolicyRef>,
) -> PolicyOutcome {
    PolicyOutcome {
        stage,
        decision: crate::policy::PolicyDecision::allow("tool.terminal.pending_post_policy"),
        subject: None,
        source: Some(source),
        destination: Some(destination),
        policy_refs,
        privacy: PrivacyClass::Internal,
        retention: crate::domain::RetentionClass::RunScoped,
    }
}

#[derive(Clone, Debug)]
/// Holds tool execution context application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct ToolExecutionContext {
    /// Run identifier used for lineage, filtering, replay, and dedupe.
    pub run_id: RunId,
    /// Agent identifier used for lineage, filtering, and ownership checks.
    pub agent_id: AgentId,
    /// Optional host-provided session identifier for grouping related turns.
    pub session_id: Option<SessionId>,
    /// Turn identifier for one loop turn within a run.
    pub turn_id: Option<TurnId>,
    /// Source label or ref for this item; it is metadata and does not fetch
    /// content by itself.
    pub source: SourceRef,
    /// Fingerprint of the runtime package snapshot in force when this value was produced.
    /// Use it for replay, dedupe, and package-lineage checks; the field is evidence and does
    /// not execute package behavior.
    pub runtime_package_fingerprint: String,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy: PrivacyClass,
    /// Stable redaction policy id used for typed lineage, lookup, or dedupe.
    pub redaction_policy_id: String,
    /// Next journal seq used by this record or request.
    pub next_journal_seq: u64,
    /// Timestamp in milliseconds associated with this record.
    /// Use it for ordering and diagnostics; durable causality still comes from ids and cursors.
    pub timestamp_millis: u64,
    /// Record id prefix used by this record or request.
    pub record_id_prefix: String,
}

impl ToolExecutionContext {
    /// Creates a new application::tool value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new(
        run_id: RunId,
        agent_id: AgentId,
        source: SourceRef,
        runtime_package_fingerprint: impl Into<String>,
    ) -> Self {
        Self {
            run_id,
            agent_id,
            session_id: None,
            turn_id: None,
            source,
            runtime_package_fingerprint: runtime_package_fingerprint.into(),
            privacy: PrivacyClass::ContentRefsOnly,
            redaction_policy_id: "redaction.tool.default".to_string(),
            next_journal_seq: 1,
            timestamp_millis: 0,
            record_id_prefix: "journal.record.tool".to_string(),
        }
    }

    fn effect_id(&self, resolved: &ResolvedToolCall, suffix: Option<&str>) -> EffectId {
        match suffix {
            Some(suffix) => EffectId::new(format!(
                "effect.{}.{}",
                resolved.request.tool_call_id.as_str(),
                suffix
            )),
            None => EffectId::new(format!("effect.{}", resolved.request.tool_call_id.as_str())),
        }
    }

    fn hook_tool_effect_id(&self, resolved: &ResolvedToolCall, suffix: &str) -> EffectId {
        self.effect_id(resolved, Some(suffix))
    }

    fn record_id(&self, suffix: &str) -> String {
        format!("{}.{}", self.record_id_prefix, suffix)
    }

    fn record_base_at(
        &self,
        journal_seq: u64,
        suffix: &str,
        destination: Option<crate::domain::DestinationRef>,
    ) -> JournalRecordBase {
        let mut base = JournalRecordBase::new(
            journal_seq,
            self.record_id(suffix),
            self.run_id.clone(),
            self.agent_id.clone(),
            self.source.clone(),
        );
        base.session_id = self.session_id.clone();
        base.turn_id = self.turn_id.clone();
        base.destination = destination;
        base.timestamp_millis = self.timestamp_millis + journal_seq.saturating_sub(1);
        base.runtime_package_fingerprint = self.runtime_package_fingerprint.clone();
        base.privacy = self.privacy;
        base.redaction_policy_id = self.redaction_policy_id.clone();
        base.tags = vec!["tool_execution".to_string()];
        base
    }
}

#[derive(Clone, Debug)]
/// Holds tool execution outcome application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct ToolExecutionOutcome {
    /// Record used by this record or request.
    pub record: ToolCallRecord,
    /// Cursor identifying a replay, export, or subscription position.
    /// Use it to resume without widening the original scope.
    pub intent_cursor: Option<crate::domain::JournalCursor>,
    /// Cursor identifying a replay, export, or subscription position.
    /// Use it to resume without widening the original scope.
    pub terminal_cursor: Option<crate::domain::JournalCursor>,
    /// Optional post tool policy value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub post_tool_policy: Option<PolicyOutcome>,
    /// Whether recovery required is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub recovery_required: bool,
    /// Count of journal records appended by the coordinator for this outcome.
    pub journal_records_appended: u64,
}

impl ToolExecutionOutcome {
    fn denied(record: ToolCallRecord) -> Self {
        Self {
            record,
            intent_cursor: None,
            terminal_cursor: None,
            post_tool_policy: None,
            recovery_required: false,
            journal_records_appended: 0,
        }
    }
}

fn requires_approval_dispatch(resolved: &ResolvedToolCall) -> bool {
    resolved.route.requires_approval
        && matches!(
            resolved.route.risk_class,
            crate::policy::RiskClass::High | crate::policy::RiskClass::Critical
        )
        && resolved
            .route
            .policy_refs
            .iter()
            .any(|policy| policy.kind == PolicyKind::Approval)
}

fn approval_request_for_tool(
    resolved: &ResolvedToolCall,
    context: &ToolExecutionContext,
    next_journal_seq: u64,
) -> Result<ApprovalRequest, AgentError> {
    let requested_args_ref = resolved
        .request
        .requested_args_refs
        .first()
        .cloned()
        .ok_or_else(|| AgentError::missing_required_field("approval_request.requested_args_ref"))?;
    let turn_id = context
        .turn_id
        .clone()
        .unwrap_or_else(|| TurnId::new(format!("turn.{}", context.run_id.as_str())));
    Ok(ApprovalRequest {
        approval_request_id: ApprovalRequestId::new(format!(
            "approval.request.{}",
            resolved.request.tool_call_id.as_str()
        )),
        approval_dispatch_effect_id: context.effect_id(resolved, Some("approval")),
        run_id: context.run_id.clone(),
        session_id: context.session_id.clone(),
        agent_id: context.agent_id.clone(),
        turn_id,
        tool_call_id: resolved.request.tool_call_id.clone(),
        source: resolved.request.source.clone(),
        destination: resolved.route.destination.clone(),
        canonical_tool_name: resolved.request.canonical_tool_name.as_str().to_string(),
        tool_source: resolved.route.source.clone(),
        effect_class: resolved.route.effect_class.clone(),
        risk_class: resolved.route.risk_class.clone(),
        requested_args_ref,
        redacted_args_summary: resolved.request.redacted_args_summary.clone(),
        policy_refs: resolved.route.policy_refs.clone(),
        dispatcher_scope: DispatcherScope::SourceScoped,
        timeout_ms: 120_000,
        allowed_decisions: vec![ApprovalDecisionKind::Approved, ApprovalDecisionKind::Denied],
        created_at_millis: context.timestamp_millis + next_journal_seq.saturating_sub(1),
        runtime_package_fingerprint: RuntimePackageFingerprint(
            context.runtime_package_fingerprint.clone(),
        ),
    })
}

fn unsafe_pending_reason(
    resolved: &ResolvedToolCall,
    result: &EffectResult,
    policy_refs: &[PolicyRef],
) -> String {
    if resolved.request.idempotency_key.is_some() {
        return "terminal result append failed; idempotency key permits reconciler review"
            .to_string();
    }

    let policy_summary = policy_refs
        .iter()
        .map(|policy_ref| policy_ref.as_str())
        .collect::<Vec<_>>()
        .join(",");
    match result.terminal_status {
        EffectTerminalStatus::Completed | EffectTerminalStatus::Unknown => format!(
            "terminal result append failed after potentially non-idempotent tool execution; policy_refs={policy_summary}"
        ),
        _ => format!(
            "terminal result append failed for tool execution; policy_refs={policy_summary}"
        ),
    }
}
