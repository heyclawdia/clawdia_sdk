//! Isolation lifecycle coordination over host-provided runtimes. Use this module when
//! a workload requires environment preparation, process execution, signaling, stats,
//! or cleanup. All concrete container or VM behavior remains adapter-owned.
//!
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::{
    domain::{
        AgentError, AgentErrorKind, AgentId, PrivacyClass, RetryClassification, RunId, SourceRef,
    },
    effect::{EffectResult, EffectTerminalStatus},
    journal::{
        JournalRecord, JournalRecordBase, JournalRecordKind, JournalRecordPayload,
        PendingSideEffect, RecoveryMarker,
    },
    journal_ports::RunJournal,
    package::{RuntimePackage, RuntimePackageFingerprint},
    package_isolation::{
        CleanupPlanRef, ExecutionEnvironment, IsolatedProcessSpec, IsolationCapability,
        IsolationClass, IsolationFallback, IsolationRuntimeRef, IsolationTrustField,
        PolicyDecisionRef,
    },
    ports_isolation::{
        CleanupRequest, CleanupResult, CleanupStatus, IsolationCapabilityReport, IsolationRuntime,
        IsolationRuntimeHealth, IsolationRuntimeRegistry, ProcessStartRequest, ProcessStartResult,
    },
    records_isolation::{
        IsolationCapabilityMatchRecord, IsolationCleanupIntentRecord, IsolationCleanupResultRecord,
        IsolationDowngradeDecisionRecord, IsolationProcessStartIntentRecord,
        IsolationProcessStartResultRecord, IsolationRecord,
    },
};

#[derive(Clone)]
/// Holds isolation lifecycle coordinator application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct IsolationLifecycleCoordinator {
    journal: Arc<dyn RunJournal>,
    registry: IsolationRuntimeRegistry,
}

impl IsolationLifecycleCoordinator {
    /// Creates a new application::isolation value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new(journal: Arc<dyn RunJournal>, registry: IsolationRuntimeRegistry) -> Self {
        Self { journal, registry }
    }

    /// Selects an isolation runtime for a package environment.
    /// This validates package-sidecar requirements and reads the runtime registry capability
    /// report; it does not start a process or append journal evidence.
    pub fn select_environment(
        &self,
        package: &RuntimePackage,
        environment: &ExecutionEnvironment,
        context: IsolationLifecycleContext,
        approval: Option<IsolationDowngradeApproval>,
    ) -> Result<IsolationSelectionOutcome, AgentError> {
        self.validate_package_sidecar(package, environment)?;
        let (runtime, report) = self.selected_runtime(environment)?;
        Ok(evaluate_report(
            environment,
            &context,
            runtime.runtime_ref(),
            report,
            approval,
        ))
    }

    /// Starts one isolated process after package and downgrade policy checks.
    /// This prepares the environment, appends lifecycle evidence, and delegates
    /// only the adapter process start to the selected isolation runtime.
    pub fn start_process(
        &self,
        package: &RuntimePackage,
        environment: ExecutionEnvironment,
        process: IsolatedProcessSpec,
        context: IsolationLifecycleContext,
        approval: Option<IsolationDowngradeApproval>,
    ) -> Result<IsolationProcessOutcome, AgentError> {
        process.validate()?;
        let selection =
            self.select_environment(package, &environment, context.clone(), approval)?;
        if !selection.status.allows_adapter_call() {
            let status = selection.status;
            self.append_selection_record(&selection, &environment, &context, 1)?;
            return Ok(IsolationProcessOutcome {
                selection,
                status,
                intent_record: None,
                result_record: None,
                io_frames: Vec::new(),
                recovery_required: false,
                terminal_error: None,
            });
        }
        let runtime_ref = selection
            .selected_runtime_ref
            .clone()
            .ok_or_else(|| missing_runtime_error("isolation selection has no runtime ref"))?;
        let runtime = self
            .registry
            .get(&runtime_ref)
            .ok_or_else(|| missing_runtime_error("selected isolation runtime disappeared"))?;

        let effect_intent = process.effect_intent(&environment);
        let intent_record =
            IsolationRecord::ProcessStartIntent(IsolationProcessStartIntentRecord {
                environment_id: environment.environment_id.clone(),
                process_ref: crate::IsolatedProcessRef::new(format!(
                    "process.ref.{}",
                    process.process_id.as_str()
                )),
                effect_intent: effect_intent.clone(),
                redacted_summary: "start isolated process with redacted argv and environment"
                    .to_string(),
            });
        let intent_journal = JournalRecord::effect_intent(
            context.record_base(1, "isolation.process.intent", &environment),
            effect_intent.clone(),
        );
        self.journal
            .append(intent_journal)
            .map_err(journal_failure)?;
        self.append_isolation_record(
            &context,
            2,
            "isolation.process.intent",
            &environment,
            intent_record.clone(),
        )?;
        self.append_selection_record(&selection, &environment, &context, 3)?;

        let start_result = runtime.start_process(ProcessStartRequest {
            environment: environment.clone(),
            process,
            effect_intent: effect_intent.clone(),
        })?;
        let effect_result = process_start_effect_result(&effect_intent, &start_result);
        let result_record =
            IsolationRecord::ProcessStartResult(IsolationProcessStartResultRecord {
                environment_id: environment.environment_id.clone(),
                process_ref: start_result.process_ref.clone(),
                effect_result: effect_result.clone(),
                content_refs: effect_result.content_refs.clone(),
                redacted_summary: start_result.redacted_summary.clone(),
            });
        let result_journal = JournalRecord::effect_result(
            context.record_base(4, "isolation.process.result", &environment),
            effect_result.clone(),
        );

        match self.journal.append(result_journal) {
            Ok(_) => {
                self.append_isolation_record(
                    &context,
                    5,
                    "isolation.process.result.record",
                    &environment,
                    result_record.clone(),
                )?;
                Ok(IsolationProcessOutcome {
                    status: selection.status,
                    selection,
                    intent_record: Some(intent_record),
                    result_record: Some(result_record),
                    io_frames: start_result.io_frames,
                    recovery_required: false,
                    terminal_error: None,
                })
            }
            Err(result_error) => {
                let recovery = RecoveryMarker {
                    unsafe_pending: vec![PendingSideEffect {
                        effect_id: effect_intent.effect_id.clone(),
                        intent_record_id: "journal.isolation.process.intent".to_string(),
                        idempotency_key: effect_intent.idempotency_key.clone(),
                        dedupe_key: effect_intent.dedupe_key.clone(),
                        unsafe_pending_reason:
                            "isolated process may have started before terminal result append"
                                .to_string(),
                    }],
                    recovery_reason: format!(
                        "isolated process terminal result append failed: {}",
                        result_error.context().message
                    ),
                    policy_refs: effect_intent.policy_refs.clone(),
                };
                let recovery_record = JournalRecord::recovery(
                    context.record_base(5, "isolation.process.recovery", &environment),
                    recovery,
                );
                self.journal
                    .append(recovery_record)
                    .map_err(|recovery_error| {
                        AgentError::new(
                            AgentErrorKind::RecoveryRepairNeeded,
                            RetryClassification::RepairNeeded,
                            format!(
                                "process result append failed and recovery append failed: {}",
                                recovery_error.context().message
                            ),
                        )
                    })?;
                Ok(IsolationProcessOutcome {
                    status: selection.status,
                    selection,
                    intent_record: Some(intent_record),
                    result_record: Some(result_record),
                    io_frames: start_result.io_frames,
                    recovery_required: true,
                    terminal_error: Some(AgentError::new(
                        AgentErrorKind::RecoveryRepairNeeded,
                        RetryClassification::RepairNeeded,
                        "isolated process terminal result append failed; recovery required",
                    )),
                })
            }
        }
    }

    /// Cleanup environment.
    /// This calls the selected isolation cleanup adapter for environment resources that the
    /// runtime already prepared.
    pub fn cleanup_environment(
        &self,
        environment: ExecutionEnvironment,
        context: IsolationLifecycleContext,
    ) -> Result<IsolationCleanupOutcome, AgentError> {
        let runtime = self
            .registry
            .first()
            .ok_or_else(|| missing_runtime_error("no isolation runtime is registered"))?;
        let cleanup_plan_ref =
            CleanupPlanRef::new(format!("cleanup.{}", environment.environment_id.as_str()));
        let mut intent = crate::EffectIntent::new(
            crate::EffectId::new(format!("effect.{}", cleanup_plan_ref.as_str())),
            crate::EffectKind::ChildArtifactShutdown,
            environment.subject_ref(),
            environment.source.clone(),
            "cleanup isolated environment and child artifacts",
        );
        intent.destination = Some(environment.destination.clone());

        let intent_record = JournalRecord::effect_intent(
            context.record_base(1, "isolation.cleanup.intent", &environment),
            intent.clone(),
        );
        self.journal
            .append(intent_record)
            .map_err(journal_failure)?;
        let isolation_intent = IsolationRecord::CleanupIntent(IsolationCleanupIntentRecord {
            environment_id: environment.environment_id.clone(),
            cleanup_plan_ref: cleanup_plan_ref.clone(),
            effect_intent: intent.clone(),
            redacted_summary: "cleanup isolated environment and child artifacts".to_string(),
        });
        self.append_isolation_record(
            &context,
            2,
            "isolation.cleanup.intent.record",
            &environment,
            isolation_intent,
        )?;

        let cleanup = runtime.cleanup(CleanupRequest {
            environment: environment.clone(),
            cleanup_plan_ref: cleanup_plan_ref.clone(),
        })?;
        let terminal_status = match cleanup.status {
            CleanupStatus::Completed => EffectTerminalStatus::Completed,
            CleanupStatus::RepairNeeded => EffectTerminalStatus::Failed,
        };
        let result = EffectResult {
            effect_id: intent.effect_id.clone(),
            terminal_status,
            external_operation_id: cleanup.external_operation_id.clone(),
            reconciliation_ref: None,
            error_ref: (cleanup.status == CleanupStatus::RepairNeeded)
                .then(|| "cleanup.repair_needed".to_string()),
            content_refs: Vec::new(),
            redacted_summary: cleanup.redacted_summary.clone(),
        };
        let result_record = JournalRecord::effect_result(
            context.record_base(3, "isolation.cleanup.result", &environment),
            result.clone(),
        );
        self.journal
            .append(result_record)
            .map_err(journal_failure)?;
        let isolation_result = IsolationRecord::CleanupResult(IsolationCleanupResultRecord {
            environment_id: environment.environment_id.clone(),
            cleanup_plan_ref,
            status: cleanup.status,
            effect_result: result,
            redacted_summary: cleanup.redacted_summary.clone(),
        });
        self.append_isolation_record(
            &context,
            4,
            "isolation.cleanup.result.record",
            &environment,
            isolation_result,
        )?;

        Ok(IsolationCleanupOutcome {
            status: cleanup.status,
            cleanup_result: cleanup,
        })
    }

    fn validate_package_sidecar(
        &self,
        package: &RuntimePackage,
        environment: &ExecutionEnvironment,
    ) -> Result<(), AgentError> {
        let snapshot = package
            .isolation_requirements
            .iter()
            .find(|snapshot| snapshot.requirement_ref == environment.requirement_ref)
            .ok_or_else(|| {
                AgentError::new(
                    AgentErrorKind::IsolationFailure,
                    RetryClassification::HostConfigurationNeeded,
                    "runtime package is missing the requested isolation sidecar",
                )
            })?;
        snapshot.validate()
    }

    fn selected_runtime(
        &self,
        environment: &ExecutionEnvironment,
    ) -> Result<(Arc<dyn IsolationRuntime>, IsolationCapabilityReport), AgentError> {
        let runtime = environment
            .spec
            .requirement
            .preferred_adapters
            .iter()
            .find_map(|runtime_ref| self.registry.get(runtime_ref))
            .or_else(|| self.registry.first())
            .ok_or_else(|| missing_runtime_error("no isolation runtime is registered"))?;
        let report = runtime.capability_report()?;
        Ok((runtime, report))
    }

    fn append_selection_record(
        &self,
        selection: &IsolationSelectionOutcome,
        environment: &ExecutionEnvironment,
        context: &IsolationLifecycleContext,
        journal_seq: u64,
    ) -> Result<(), AgentError> {
        let record = selection
            .match_record
            .clone()
            .map(IsolationRecord::CapabilityMatch)
            .or_else(|| {
                selection
                    .downgrade_record
                    .clone()
                    .map(IsolationRecord::DowngradeDecision)
            });
        if let Some(record) = record {
            self.append_isolation_record(
                context,
                journal_seq,
                "isolation.selection",
                environment,
                record,
            )?;
        }
        Ok(())
    }

    fn append_isolation_record(
        &self,
        context: &IsolationLifecycleContext,
        journal_seq: u64,
        record_id: &str,
        environment: &ExecutionEnvironment,
        record: IsolationRecord,
    ) -> Result<(), AgentError> {
        let base = context.record_base(journal_seq, record_id, environment);
        self.journal
            .append(JournalRecord::feature_record(
                base,
                JournalRecordKind::Isolation,
                "isolation",
                isolation_event_kind(&record),
                environment.subject_ref(),
                Vec::new(),
                isolation_content_refs(&record),
                JournalRecordPayload::Isolation(record),
            ))
            .map(|_| ())
            .map_err(journal_failure)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Holds isolation lifecycle context application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct IsolationLifecycleContext {
    /// Run identifier used for lineage, filtering, replay, and dedupe.
    pub run_id: RunId,
    /// Agent identifier used for lineage, filtering, and ownership checks.
    pub agent_id: AgentId,
    /// Source label or ref for this item; it is metadata and does not fetch
    /// content by itself.
    pub source: SourceRef,
    /// Fingerprint of the runtime package snapshot in force when this value was produced.
    /// Use it for replay, dedupe, and package-lineage checks; the field is evidence and does
    /// not execute package behavior.
    pub runtime_package_fingerprint: RuntimePackageFingerprint,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy: PrivacyClass,
    /// Stable redaction policy id used for typed lineage, lookup, or dedupe.
    pub redaction_policy_id: String,
    /// Readiness state for a capability or package feature.
    /// Launch and package validation use it to distinguish active, reserved, and blocked
    /// surfaces.
    pub readiness_profile: IsolationReadinessProfile,
}

impl IsolationLifecycleContext {
    /// Builds the test value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn test(runtime_package_fingerprint: RuntimePackageFingerprint) -> Self {
        Self {
            run_id: RunId::new("run.isolation.contract"),
            agent_id: AgentId::new("agent.isolation.contract"),
            source: SourceRef::with_kind(crate::SourceKind::Sdk, "source.sdk.isolation"),
            runtime_package_fingerprint,
            privacy: PrivacyClass::ContentRefsOnly,
            redaction_policy_id: "policy.redaction.isolation".to_string(),
            readiness_profile: IsolationReadinessProfile::TestOnly,
        }
    }

    fn record_base(
        &self,
        journal_seq: u64,
        record_id: &str,
        environment: &ExecutionEnvironment,
    ) -> JournalRecordBase {
        let mut base = JournalRecordBase::new(
            journal_seq,
            format!("journal.{record_id}"),
            self.run_id.clone(),
            self.agent_id.clone(),
            self.source.clone(),
        );
        base.destination = Some(environment.destination.clone());
        base.runtime_package_fingerprint = self.runtime_package_fingerprint.as_str().to_string();
        base.privacy = self.privacy;
        base.redaction_policy_id = self.redaction_policy_id.clone();
        base.tags = vec!["isolation".to_string()];
        base
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite isolation readiness profile cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum IsolationReadinessProfile {
    /// Use this variant when the contract needs to represent production; selecting it has no side effect by itself.
    Production,
    /// Use this variant when the contract needs to represent test only; selecting it has no side effect by itself.
    TestOnly,
}

#[derive(Clone, Debug)]
/// Holds isolation selection outcome application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct IsolationSelectionOutcome {
    /// Finite status for this record or lifecycle stage.
    pub status: IsolationMatchStatus,
    /// Typed selected runtime ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub selected_runtime_ref: Option<IsolationRuntimeRef>,
    /// Optional capability report value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub capability_report: Option<IsolationCapabilityReport>,
    /// Optional match record value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub match_record: Option<IsolationCapabilityMatchRecord>,
    /// Optional downgrade record value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub downgrade_record: Option<IsolationDowngradeDecisionRecord>,
    /// Optional terminal error value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub terminal_error: Option<AgentError>,
}

#[derive(Clone, Debug)]
/// Holds isolation process outcome application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct IsolationProcessOutcome {
    /// Selection used by this record or request.
    pub selection: IsolationSelectionOutcome,
    /// Finite status for this record or lifecycle stage.
    pub status: IsolationMatchStatus,
    /// Optional intent record value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub intent_record: Option<IsolationRecord>,
    /// Optional result record value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub result_record: Option<IsolationRecord>,
    /// Collection of io frames values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub io_frames: Vec<crate::ProcessIoFrame>,
    /// Whether recovery required is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub recovery_required: bool,
    /// Optional terminal error value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub terminal_error: Option<AgentError>,
}

#[derive(Clone, Debug)]
/// Holds isolation cleanup outcome application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct IsolationCleanupOutcome {
    /// Finite status for this record or lifecycle stage.
    pub status: CleanupStatus,
    /// Cleanup result used by this record or request.
    pub cleanup_result: CleanupResult,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Enumerates the finite isolation match status cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum IsolationMatchStatus {
    /// Use this variant when the contract needs to represent matched; selecting it has no side effect by itself.
    Matched,
    /// Use this variant when the contract needs to represent downgrade approved; selecting it has no side effect by itself.
    DowngradeApproved,
    /// Use this variant when the contract needs to represent downgrade denied; selecting it has no side effect by itself.
    DowngradeDenied,
    /// Use this variant when the contract needs to represent unsupported host; selecting it has no side effect by itself.
    UnsupportedHost,
}

impl IsolationMatchStatus {
    fn allows_adapter_call(self) -> bool {
        matches!(self, Self::Matched | Self::DowngradeApproved)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Holds isolation downgrade approval application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct IsolationDowngradeApproval {
    /// Typed decision ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub decision_ref: PolicyDecisionRef,
    /// Scope used by this record or request.
    pub scope: PolicyDecisionScope,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Classification selectors for approved classes.
    /// Policy and projection paths use them for finite routing decisions.
    pub approved_classes: Vec<IsolationClass>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Capability downgrades explicitly approved for this isolation decision.
    /// Adapters may rely on these approvals only for the request and package fingerprint they
    /// reference.
    pub approved_capability_downgrades: Vec<IsolationCapability>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Collection of approved trust downgrades values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub approved_trust_downgrades: Vec<IsolationTrustField>,
}

impl IsolationDowngradeApproval {
    /// Builds the approved for isolation value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn approved_for_isolation(decision_ref: impl Into<PolicyDecisionRef>) -> Self {
        Self {
            decision_ref: decision_ref.into(),
            scope: PolicyDecisionScope::IsolationDowngrade,
            approved_classes: Vec::new(),
            approved_capability_downgrades: Vec::new(),
            approved_trust_downgrades: Vec::new(),
        }
    }

    /// Builds the approved for tool value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn approved_for_tool(decision_ref: impl Into<PolicyDecisionRef>) -> Self {
        Self {
            decision_ref: decision_ref.into(),
            scope: PolicyDecisionScope::ToolApproval,
            approved_classes: Vec::new(),
            approved_capability_downgrades: Vec::new(),
            approved_trust_downgrades: Vec::new(),
        }
    }

    /// Returns an updated value with approve capability configured.
    /// This evaluates or builds isolation policy state in memory and does not call the
    /// isolation adapter by itself.
    pub fn approve_capability(mut self, capability: IsolationCapability) -> Self {
        self.approved_capability_downgrades.push(capability);
        self
    }

    /// Returns an updated value with approve trust configured.
    /// This evaluates or builds isolation policy state in memory and does not call the
    /// isolation adapter by itself.
    pub fn approve_trust(mut self, field: IsolationTrustField) -> Self {
        self.approved_trust_downgrades.push(field);
        self
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Enumerates the finite policy decision scope cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum PolicyDecisionScope {
    /// Use this variant when the contract needs to represent isolation downgrade; selecting it has no side effect by itself.
    IsolationDowngrade,
    /// Use this variant when the contract needs to represent tool approval; selecting it has no side effect by itself.
    ToolApproval,
}

fn evaluate_report(
    environment: &ExecutionEnvironment,
    context: &IsolationLifecycleContext,
    selected_runtime_ref: &IsolationRuntimeRef,
    report: IsolationCapabilityReport,
    approval: Option<IsolationDowngradeApproval>,
) -> IsolationSelectionOutcome {
    let selected_class = report
        .supported_classes
        .first()
        .copied()
        .unwrap_or(IsolationClass::HostProcess);
    let requested_class = environment.spec.requirement.minimum_class;
    let capability_gaps = environment
        .spec
        .requirement
        .required_capabilities
        .missing_from(&report.capabilities);
    let trust_gaps = environment
        .spec
        .requirement
        .trust
        .gaps_against(&report.trust);
    let class_gap = !class_satisfies(requested_class, selected_class);

    if matches!(
        report.health,
        IsolationRuntimeHealth::UnsupportedHost { .. }
    ) {
        return denied_outcome(
            IsolationMatchStatus::UnsupportedHost,
            environment,
            selected_runtime_ref,
            requested_class,
            selected_class,
            capability_gaps,
            trust_gaps,
            approval,
            Some(report),
            "isolation runtime reports unsupported host",
        );
    }

    if !class_gap && capability_gaps.is_empty() && trust_gaps.is_empty() {
        let match_record = IsolationCapabilityMatchRecord {
            environment_id: environment.environment_id.clone(),
            adapter_ref: selected_runtime_ref.clone(),
            requested_class,
            selected_class,
            missing_capabilities: Vec::new(),
            trust_gaps: Vec::new(),
            redacted_summary: "isolation capability report satisfies requirement".to_string(),
        };
        return IsolationSelectionOutcome {
            status: IsolationMatchStatus::Matched,
            selected_runtime_ref: Some(selected_runtime_ref.clone()),
            capability_report: Some(report),
            match_record: Some(match_record),
            downgrade_record: None,
            terminal_error: None,
        };
    }

    if fallback_allows(
        &environment.spec.requirement.fallback,
        context,
        selected_class,
        class_gap,
        &capability_gaps,
        &trust_gaps,
        approval.as_ref(),
    ) {
        return IsolationSelectionOutcome {
            status: IsolationMatchStatus::DowngradeApproved,
            selected_runtime_ref: Some(selected_runtime_ref.clone()),
            capability_report: Some(report),
            match_record: None,
            downgrade_record: Some(downgrade_record(
                environment,
                selected_runtime_ref,
                requested_class,
                selected_class,
                capability_gaps,
                trust_gaps,
                true,
                approval,
                "isolation downgrade approved by package and isolation policy decision",
            )),
            terminal_error: None,
        };
    }

    denied_outcome(
        IsolationMatchStatus::DowngradeDenied,
        environment,
        selected_runtime_ref,
        requested_class,
        selected_class,
        capability_gaps,
        trust_gaps,
        approval,
        Some(report),
        "isolation downgrade denied before adapter execution",
    )
}

fn class_satisfies(requested: IsolationClass, selected: IsolationClass) -> bool {
    if requested == selected || requested == IsolationClass::HostProcess {
        return true;
    }
    matches!(
        (requested, selected),
        (
            IsolationClass::Sandbox,
            IsolationClass::Container
                | IsolationClass::LightweightVm
                | IsolationClass::RemoteSandbox
        )
    )
}

fn fallback_allows(
    fallback: &IsolationFallback,
    context: &IsolationLifecycleContext,
    selected_class: IsolationClass,
    class_gap: bool,
    capability_gaps: &[IsolationCapability],
    trust_gaps: &[IsolationTrustField],
    approval: Option<&IsolationDowngradeApproval>,
) -> bool {
    match fallback {
        IsolationFallback::Deny => false,
        IsolationFallback::TestOnlyHostProcess => {
            selected_class == IsolationClass::HostProcess
                && context.readiness_profile == IsolationReadinessProfile::TestOnly
        }
        IsolationFallback::AllowIfPackageAndPolicyApprove {
            accepted_classes,
            accepted_capability_downgrades,
            accepted_trust_downgrades,
            ..
        } => {
            let Some(approval) = approval else {
                return false;
            };
            if approval.scope != PolicyDecisionScope::IsolationDowngrade {
                return false;
            }
            if class_gap && !accepted_classes.contains(&selected_class) {
                return false;
            }
            if class_gap
                && !approval.approved_classes.is_empty()
                && !approval.approved_classes.contains(&selected_class)
            {
                return false;
            }
            capability_gaps.iter().all(|gap| {
                accepted_capability_downgrades.contains(gap)
                    && approval.approved_capability_downgrades.contains(gap)
            }) && trust_gaps.iter().all(|gap| {
                accepted_trust_downgrades.contains(gap)
                    && approval.approved_trust_downgrades.contains(gap)
            })
        }
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "private isolation helper keeps downgrade evidence explicit; grouping belongs with a broader selection-outcome builder"
)]
fn denied_outcome(
    status: IsolationMatchStatus,
    environment: &ExecutionEnvironment,
    selected_runtime_ref: &IsolationRuntimeRef,
    requested_class: IsolationClass,
    selected_class: IsolationClass,
    capability_gaps: Vec<IsolationCapability>,
    trust_gaps: Vec<IsolationTrustField>,
    approval: Option<IsolationDowngradeApproval>,
    report: Option<IsolationCapabilityReport>,
    summary: &str,
) -> IsolationSelectionOutcome {
    IsolationSelectionOutcome {
        status,
        selected_runtime_ref: Some(selected_runtime_ref.clone()),
        capability_report: report,
        match_record: None,
        downgrade_record: Some(downgrade_record(
            environment,
            selected_runtime_ref,
            requested_class,
            selected_class,
            capability_gaps,
            trust_gaps,
            false,
            approval,
            summary,
        )),
        terminal_error: None,
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "private isolation helper mirrors the durable downgrade decision record fields for auditability"
)]
fn downgrade_record(
    environment: &ExecutionEnvironment,
    selected_runtime_ref: &IsolationRuntimeRef,
    requested_class: IsolationClass,
    selected_class: IsolationClass,
    capability_gaps: Vec<IsolationCapability>,
    trust_gaps: Vec<IsolationTrustField>,
    approved: bool,
    approval: Option<IsolationDowngradeApproval>,
    summary: &str,
) -> IsolationDowngradeDecisionRecord {
    IsolationDowngradeDecisionRecord {
        environment_id: environment.environment_id.clone(),
        adapter_ref: selected_runtime_ref.clone(),
        requested_class,
        selected_class,
        capability_gaps,
        trust_gaps,
        approved,
        policy_decision_scope: approval.as_ref().map(|approval| approval.scope),
        policy_decision_refs: approval
            .map(|approval| vec![approval.decision_ref])
            .unwrap_or_default(),
        redacted_summary: summary.to_string(),
    }
}

fn process_start_effect_result(
    intent: &crate::EffectIntent,
    result: &ProcessStartResult,
) -> EffectResult {
    EffectResult {
        effect_id: intent.effect_id.clone(),
        terminal_status: result.terminal_status.clone(),
        external_operation_id: result.external_operation_id.clone(),
        reconciliation_ref: None,
        error_ref: None,
        content_refs: result
            .io_frames
            .iter()
            .flat_map(|frame| frame.content_refs.clone())
            .collect(),
        redacted_summary: result.redacted_summary.clone(),
    }
}

fn isolation_event_kind(record: &IsolationRecord) -> &'static str {
    match record {
        IsolationRecord::Requested(_) => "isolation_requested",
        IsolationRecord::AdapterCapabilityReported(_) => "isolation_adapter_health_checked",
        IsolationRecord::CapabilityMatch(_) => "isolation_capability_matched",
        IsolationRecord::DowngradeDecision(record) if record.approved => {
            "isolation_downgrade_approved"
        }
        IsolationRecord::DowngradeDecision(_) => "isolation_downgrade_denied",
        IsolationRecord::EnvironmentPrepareIntent(_)
        | IsolationRecord::EnvironmentPrepareResult(_) => "isolation_environment_prepared",
        IsolationRecord::ProcessStartIntent(_) | IsolationRecord::ProcessStartResult(_) => {
            "isolation_process_started"
        }
        IsolationRecord::ProcessIoFrame(_) => "isolation_process_io_captured",
        IsolationRecord::ProcessStatsSnapshot(_) => "isolation_process_stats_recorded",
        IsolationRecord::CleanupIntent(_) => "isolation_cleanup_started",
        IsolationRecord::CleanupResult(record) if record.status == CleanupStatus::Completed => {
            "isolation_cleanup_completed"
        }
        IsolationRecord::CleanupResult(_) => "isolation_cleanup_failed",
        IsolationRecord::Failed(_) => "isolation_failed",
    }
}

fn isolation_content_refs(record: &IsolationRecord) -> Vec<crate::domain::ContentRef> {
    match record {
        IsolationRecord::ProcessStartResult(record) => record.content_refs.clone(),
        IsolationRecord::CleanupResult(record) => record.effect_result.content_refs.clone(),
        _ => Vec::new(),
    }
}

fn missing_runtime_error(message: impl Into<String>) -> AgentError {
    AgentError::new(
        AgentErrorKind::IsolationFailure,
        RetryClassification::HostConfigurationNeeded,
        message,
    )
}

fn journal_failure(error: AgentError) -> AgentError {
    AgentError::new(
        AgentErrorKind::JournalFailure,
        RetryClassification::RepairNeeded,
        error.context().message,
    )
}
