//! Tool execution coordination over the shared effect spine. Use this module after
//! tool routing and policy have selected an executor. Execution may call tool
//! adapters and must keep intent/result records observable through journals and
//! events.
//!
use std::sync::Arc;

use crate::{
    domain::{
        AgentError, AgentErrorKind, AgentId, EffectId, EntityKind, EntityRef, PolicyRef,
        PrivacyClass, RetryClassification, RunId, SourceRef, TurnId,
    },
    effect::{EffectIntent, EffectKind, EffectResult, EffectTerminalStatus},
    journal::{JournalRecord, JournalRecordBase, PendingSideEffect, RecoveryMarker},
    journal_ports::RunJournal,
    policy::{MissingDependency, PolicyOutcome, PolicyStage},
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
    strategy: ToolExecutionStrategy,
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
            strategy: ToolExecutionStrategy::default(),
        }
    }

    /// Returns this value with its policy setting replaced. The method
    /// follows builder-style data construction and does not execute
    /// external work.
    pub fn with_policy(mut self, policy: Arc<dyn ToolPolicyPort>) -> Self {
        self.policy = Some(policy);
        self
    }

    /// Returns this value with its strategy setting replaced. The
    /// method follows builder-style data construction and does not
    /// execute external work.
    pub fn with_strategy(mut self, strategy: ToolExecutionStrategy) -> Self {
        self.strategy = strategy;
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
        let resolved = self.router.resolve(request)?;
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

        let pre_tool = policy.evaluate_pre_tool(&resolved)?;
        if !pre_tool.is_allowed() {
            return Ok(ToolExecutionOutcome::denied(
                initial_record.with_denial(pre_tool),
            ));
        }

        let Some(executor_ref) = &resolved.route.executor_ref else {
            return Ok(ToolExecutionOutcome::denied(initial_record.with_denial(
                PolicyOutcome::fail_closed(PolicyStage::PreTool, MissingDependency::ExecutorRef),
            )));
        };
        let Some(executor) = self.executors.get(executor_ref) else {
            return Ok(ToolExecutionOutcome::denied(initial_record.with_denial(
                PolicyOutcome::fail_closed(PolicyStage::PreTool, MissingDependency::ExecutorRef),
            )));
        };

        let effect_id = context.effect_id(&resolved);
        let intent = self.effect_intent(effect_id.clone(), &resolved);
        let mut record = initial_record.with_intent(intent.clone());
        let tool_subject = record.subject_ref();
        let intent_record = tool_call_journal_record(
            context.record_base(0, "tool.intent", Some(resolved.route.destination.clone())),
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
        let result_record = tool_call_journal_record(
            context.record_base(1, "tool.result", Some(resolved.route.destination.clone())),
            terminal_record,
            "tool_result_recorded",
        );

        match journal.append(result_record) {
            Ok(cursor) => {
                let post_tool = policy.evaluate_post_tool(&resolved, &output)?;
                record = record.with_result(result, post_tool.clone());
                Ok(ToolExecutionOutcome {
                    record,
                    intent_cursor: Some(intent_cursor),
                    terminal_cursor: Some(cursor),
                    post_tool_policy: Some(post_tool),
                    recovery_required: false,
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
                    context.record_base(1, "tool.recovery", Some(resolved.route.destination)),
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
                })
            }
        }
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

    fn effect_id(&self, resolved: &ResolvedToolCall) -> EffectId {
        EffectId::new(format!("effect.{}", resolved.request.tool_call_id.as_str()))
    }

    fn record_id(&self, suffix: &str) -> String {
        format!("{}.{}", self.record_id_prefix, suffix)
    }

    fn record_base(
        &self,
        offset: u64,
        suffix: &str,
        destination: Option<crate::domain::DestinationRef>,
    ) -> JournalRecordBase {
        let mut base = JournalRecordBase::new(
            self.next_journal_seq + offset,
            self.record_id(suffix),
            self.run_id.clone(),
            self.agent_id.clone(),
            self.source.clone(),
        );
        base.turn_id = self.turn_id.clone();
        base.destination = destination;
        base.timestamp_millis = self.timestamp_millis + offset;
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
}

impl ToolExecutionOutcome {
    fn denied(record: ToolCallRecord) -> Self {
        Self {
            record,
            intent_cursor: None,
            terminal_cursor: None,
            post_tool_policy: None,
            recovery_required: false,
        }
    }
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
