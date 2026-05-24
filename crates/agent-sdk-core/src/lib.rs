//! Product-neutral core contracts and first-slice runtime for the Agent SDK.
//!
//! The core crate owns typed domain primitives, package snapshots, run control,
//! events, journals, policy, and test fakes. Product hosts and optional crates
//! own concrete providers, toolkits, isolation runtimes, telemetry exporters,
//! workflow engines, UI, and channel adapters.
//!
//! # Common Helper
//!
//! Simple helpers are convenience wrappers over the canonical request path.
//!
//! ```
//! use agent_sdk_core::{Agent, AgentId, RunId, RunRequest, SourceKind, SourceRef};
//!
//! let agent = Agent::builder()
//!     .id(AgentId::new("agent.docs.helper"))
//!     .name("docs helper")
//!     .build()?;
//! let source = SourceRef::with_kind(SourceKind::Host, "source.docs.helper");
//!
//! let lowered = RunRequest::text(
//!     RunId::new("run.docs.helper"),
//!     agent.id().clone(),
//!     source.clone(),
//!     "hello",
//! );
//!
//! assert_eq!(
//!     agent
//!         .typed_text_request::<DocsTodo>(
//!             RunId::new("run.docs.helper.typed"),
//!             source,
//!             "extract a todo",
//!         )
//!         .agent_id,
//!     lowered.agent_id
//! );
//!
//! # #[derive(Clone, Debug)]
//! # struct DocsTodo;
//! # impl agent_sdk_core::TypedOutputModel for DocsTodo {
//! #     const SCHEMA_ID: &'static str = "schema.docs.todo";
//! #     const SCHEMA_VERSION: agent_sdk_core::SchemaVersion =
//! #         agent_sdk_core::SchemaVersion::new(1, 0, 0);
//! #     fn schema_ref() -> agent_sdk_core::OutputSchemaRef {
//! #         agent_sdk_core::OutputContract::inline_json_schema(
//! #             agent_sdk_core::OutputSchemaId::new(Self::SCHEMA_ID),
//! #             Self::SCHEMA_VERSION,
//! #             serde_json::json!({"type": "object"}),
//! #         )
//! #         .schema
//! #     }
//! # }
//! # Ok::<(), agent_sdk_core::AgentError>(())
//! ```
//!
//! # Advanced Request Path
//!
//! Advanced callers can construct the same DTOs explicitly. Runtime execution
//! still resolves one effective `RuntimePackage` before provider calls, policy
//! checks, journal appends, events, redaction, and telemetry projections.
//!
//! ```
//! use agent_sdk_core::{
//!     AgentId, OutputContract, RunId, RunRequest, SourceKind, SourceRef,
//! };
//!
//! # #[derive(Clone, Debug)]
//! # struct DocsTodo;
//! # impl agent_sdk_core::TypedOutputModel for DocsTodo {
//! #     const SCHEMA_ID: &'static str = "schema.docs.todo.advanced";
//! #     const SCHEMA_VERSION: agent_sdk_core::SchemaVersion =
//! #         agent_sdk_core::SchemaVersion::new(1, 0, 0);
//! #     fn schema_ref() -> agent_sdk_core::OutputSchemaRef {
//! #         agent_sdk_core::OutputContract::inline_json_schema(
//! #             agent_sdk_core::OutputSchemaId::new(Self::SCHEMA_ID),
//! #             Self::SCHEMA_VERSION,
//! #             serde_json::json!({"type": "object"}),
//! #         )
//! #         .schema
//! #     }
//! # }
//! let request = RunRequest::text(
//!     RunId::new("run.docs.advanced"),
//!     AgentId::new("agent.docs.advanced"),
//!     SourceRef::with_kind(SourceKind::Host, "source.docs.advanced"),
//!     "extract a todo",
//! )
//! .with_output_contract(OutputContract::for_type::<DocsTodo>());
//!
//! assert!(request.output_contract.is_some());
//! ```
//!
//! # SemVer Posture
//!
//! The supported consumer import surface is the crate root plus documented
//! namespaces such as `agent_sdk_core::testing` and `agent_sdk_core::ports`.
//! Deep implementation modules remain review surfaces until release readiness;
//! downstream code should prefer builders, constructors, accessors, and wildcard
//! matches for public enums so the SDK can add fields and variants without
//! creating avoidable SemVer pressure.

#[path = "application/agent.rs"]
pub mod agent;
#[path = "application/agent_pool.rs"]
pub mod agent_pool;
#[path = "application/anti_entropy.rs"]
pub mod anti_entropy;
#[path = "application/isolation.rs"]
pub mod application_isolation;
#[path = "application/approval.rs"]
pub mod approval;
#[path = "ports/approval.rs"]
pub mod approval_ports;
#[path = "records/approval.rs"]
pub mod approval_records;
#[path = "testing/approval.rs"]
mod approval_testing;
#[path = "package/capability.rs"]
pub mod capability;
#[path = "application/checkpoint.rs"]
pub mod checkpoint;
#[path = "records/content.rs"]
pub mod content;
#[path = "ports/content.rs"]
pub mod content_ports;
#[path = "testing/content.rs"]
mod content_testing;
#[path = "records/context.rs"]
pub mod context;
pub mod domain;
#[path = "records/effect.rs"]
pub mod effect;
#[path = "domain/error.rs"]
pub mod error;
#[path = "records/event.rs"]
pub mod event;
#[path = "ports/event_bus.rs"]
pub mod event_bus;
#[path = "testing/event.rs"]
mod event_testing;
#[path = "records/events.rs"]
pub mod events;
#[path = "application/extension.rs"]
pub mod extension;
#[path = "ports/extension.rs"]
pub mod extension_ports;
#[path = "records/extension.rs"]
pub mod extension_records;
#[path = "testing/extension.rs"]
mod extension_testing;
#[path = "testing/fakes.rs"]
mod fakes;
#[path = "ports/hooks.rs"]
pub mod hook_ports;
#[path = "records/hooks.rs"]
pub mod hook_records;
#[path = "application/hooks.rs"]
pub mod hooks;
#[path = "testing/hooks.rs"]
mod hooks_testing;
#[path = "domain/ids.rs"]
pub mod ids;
#[path = "testing/isolation.rs"]
mod isolation_testing;
#[path = "records/journal.rs"]
pub mod journal;
#[path = "ports/journal.rs"]
pub mod journal_ports;
#[path = "application/kernel.rs"]
pub mod kernel;
#[path = "application/loop_driver.rs"]
pub mod loop_driver;
#[path = "application/loop_state.rs"]
pub mod loop_state;
#[path = "records/output.rs"]
pub mod output;
#[path = "records/output_delivery.rs"]
pub mod output_delivery;
#[path = "ports/output_delivery.rs"]
pub mod output_delivery_port;
#[path = "application/output_delivery.rs"]
pub mod output_delivery_service;
#[path = "testing/output_delivery.rs"]
mod output_delivery_testing;
pub mod package;
#[path = "package/extension.rs"]
pub mod package_extension;
#[path = "package/hooks.rs"]
pub mod package_hooks;
#[path = "package/isolation.rs"]
pub mod package_isolation;
#[path = "domain/policy.rs"]
pub mod policy;
pub mod ports;
#[path = "ports/isolation.rs"]
pub mod ports_isolation;
#[path = "domain/privacy.rs"]
pub mod privacy;
#[path = "application/projection.rs"]
pub mod projection;
#[path = "ports/provider.rs"]
pub mod provider;
#[path = "ports/providers.rs"]
pub mod providers;
#[path = "application/realtime.rs"]
pub mod realtime;
#[path = "records/realtime.rs"]
pub mod realtime_records;
#[path = "testing/realtime.rs"]
mod realtime_testing;
#[path = "records/isolation.rs"]
pub mod records_isolation;
#[path = "application/recovery.rs"]
pub mod recovery;
#[path = "domain/refs.rs"]
pub mod refs;
#[path = "application/repair.rs"]
pub mod repair;
#[path = "application/replay.rs"]
pub mod replay;
#[path = "application/run.rs"]
pub mod run;
#[path = "application/run_handle.rs"]
pub mod run_handle;
#[path = "application/runtime.rs"]
pub mod runtime;
#[path = "application/stream.rs"]
pub mod stream;
#[path = "records/stream.rs"]
pub mod stream_records;
#[path = "records/structured_output.rs"]
pub mod structured_output;
#[path = "application/subagent.rs"]
pub mod subagent;
#[path = "records/subagent.rs"]
pub mod subagent_records;
#[path = "ports/subscription.rs"]
pub mod subscription;
#[path = "application/telemetry.rs"]
pub mod telemetry;
#[path = "ports/telemetry.rs"]
pub mod telemetry_ports;
#[path = "records/telemetry.rs"]
pub mod telemetry_records;
#[path = "testing/telemetry.rs"]
mod telemetry_testing;
pub mod testing;
#[path = "application/tool.rs"]
pub mod tool_execution;
#[path = "ports/tool_pack.rs"]
pub mod tool_pack_ports;
#[path = "records/tool_pack.rs"]
pub mod tool_pack_records;
#[path = "ports/tool.rs"]
pub mod tool_ports;
#[path = "records/tool.rs"]
pub mod tool_records;
#[path = "testing/tool.rs"]
mod tool_testing;
#[path = "ports/typed_output.rs"]
pub mod typed_output_ports;
#[path = "records/validated_output.rs"]
pub mod validated_output;
#[path = "application/validation.rs"]
pub mod validation;

pub use agent::{Agent, AgentBuilder};
pub use agent_pool::{
    AgentPool, AgentPoolBuilder, AgentPoolMember, AgentPoolMessagePolicy, AgentPoolWakePolicy,
    MessageReceipt, MessageResponseContract, MessageStatus, ResumeInputPolicy, RunAddress,
    RunAddressTarget, RunMessage, WakeCondition, WakeRegistration, WakeRegistrationStatus,
};
pub use anti_entropy::{
    AntiEntropyRepair, AntiEntropyReport, AntiEntropyScanner, DerivedViewKind, DerivedViewState,
};
pub use application_isolation::{
    IsolationCleanupOutcome, IsolationDowngradeApproval, IsolationLifecycleContext,
    IsolationLifecycleCoordinator, IsolationMatchStatus, IsolationProcessOutcome,
    IsolationReadinessProfile, IsolationSelectionOutcome, PolicyDecisionScope,
};
pub use approval::ApprovalBroker;
pub use approval_ports::{ApprovalDispatchRequest, ApprovalDispatchResponse, ApprovalDispatcher};
pub use approval_records::{
    ApprovalBrokerOutcome, ApprovalDecision, ApprovalLifecycleStatus, ApprovalRecord,
    ApprovalRequest, ApprovalTerminalStatus,
};
pub use capability::{
    CapabilityId, CapabilityKind, CapabilityNamespace, CapabilityReadiness,
    CapabilityReadinessStatus, CapabilitySource, CapabilitySourceKind, CapabilitySpec,
    CapabilityVersion, CapabilityVisibility, ExecutableCapabilityRoute, ExecutorRef,
    PackageSidecarRef, ProjectionMode, ProviderCapabilityProjection,
};
pub use checkpoint::{
    CheckpointPrunePolicy, CheckpointPruneReport, CheckpointSaveOutcome, CheckpointStore,
    InMemoryCheckpointStore,
};
pub use content::{
    ArtifactRef, ArtifactVersion, ContentKind, ContentRef, ContentResolutionError,
    ContentResolutionErrorKind, ContentResolutionPolicy, ContentResolutionPurpose,
    ContentResolveRequest, ContentScope, ContentVersion, MissingContentPolicy, ResolvedContent,
    RetentionUse,
};
pub use content_ports::ContentResolver;
pub use context::{
    AgentMessage, AgentMessagePart, AgentMessageRole, ContextBudgetHint, ContextBudgetSummary,
    ContextContribution, ContextContributionId, ContextContributionKind, ContextItem,
    ContextProjection, ContextProjectionAudit, ContextSelectionDecision, ContextSelectionReason,
    ProjectedContextPart, ProjectionRole,
};
pub use domain::{
    AdapterRef, AgentId, AgentPoolId, ArtifactId, AttemptId, ContentId, CorrelationEntry,
    CorrelationKey, CorrelationValue, DedupeKey, DestinationKind, DestinationRef, EffectId,
    EntityKind, EntityRef, EventCursorId, EventId, IdValidationError, IdempotencyKey, LineageId,
    LineageRef, MAX_ID_LEN, MessageId, OutputSchemaId, PolicyKind, PolicyRef, PrivacyClass,
    RepairAttemptId, RetentionClass, RunId, RuntimePackageId, SourceKind, SourceRef, ToolCallId,
    TopicId, TraceId, TrustClass, TurnId, ValidatedOutputId, ValidationAttemptId, WakeConditionId,
};
pub use effect::{EffectIntent, EffectKind, EffectResult, EffectTerminalStatus};
pub use error::{AgentError, AgentErrorKind, CausalIds, ErrorContext, RetryClassification};
pub use event::{
    AgentEvent, ArchiveCursor, CompiledEventFilter, EventCursor, EventEnvelope, EventFamily,
    EventFilter, EventFrame, EventKind, EventOverflowNotice,
};
pub use event_bus::{AgentEventBus, AgentEventStream, EventArchive, InMemoryAgentEventBus};
pub use extension::{
    ExtensionActionContext, ExtensionActionCoordinator, ExtensionActionOutcome,
    ExtensionActionOutcomeStatus, ExtensionProtocolRecoveryContext,
    ExtensionProtocolRecoveryOutcome, recover_extension_protocol_error,
};
pub use extension_ports::{
    ExtensionActionExecutionOutput, ExtensionActionExecutionRequest, ExtensionActionExecutor,
    ExtensionActionExecutorRegistry, ExtensionActionRegistrySnapshot, ExtensionActionRequest,
    ExtensionActionRoute, ExtensionProtocolError, ExtensionProtocolErrorKind,
    ExtensionProtocolRequestId, ExtensionProtocolVersion, validate_extension_protocol_response_id,
};
pub use extension_records::{
    ExtensionActionEvent, ExtensionActionEventKind, ExtensionActionRecord,
    ExtensionActionRecordParams, ExtensionActionRecordStatus, ExtensionProtocolRecoveryRecord,
};
pub use hook_ports::{
    HookExecutionOutcome, HookExecutor, HookExecutorRegistry, InMemoryHookExecutorRegistry,
};
pub use hook_records::{
    HookMutationJournalPlan, HookRecord, HookRecordPayload, HookResponseDecision,
};
pub use hooks::{
    HookInvocationOutcome, HookInvocationStatus, HookLifecycleContext, HookLifecycleCoordinator,
};
pub use journal::{
    AgentPoolLifecycleStatus, AgentPoolRecord, ContextProjectionRecord, EventIndexProjection,
    JournalCursor, JournalRecord, JournalRecordBase, JournalRecordKind, JournalRecordPayload,
    MessageRecord, ModelAttemptRecord, PendingSideEffect, RecoveryMarker, RunCheckpoint,
    RunLifecycleRecord, RunMessageAddressTargetRecord, RunMessageDeliveryStatus, RunMessageRecord,
    StructuredOutputRecord, TerminalResultMarker, WakeRecord, WakeResumeInputPolicyRecord,
    WakeTriggerStatus,
};
pub use journal_ports::{RunJournal, append_before_effect, append_result_or_recovery};
pub use loop_state::{
    AgentStateMachine, CheckpointPolicy, LoopEventKind, LoopState, LoopStopReason,
    LoopTerminalResult, LoopTerminalStatus, LoopTrigger, MaxIterationOutcome, SideEffectPolicy,
    TransitionGuard, TransitionGuardSet, TransitionInput, TransitionOutput, TransitionRule,
    contract_state_names, transition_table, validate_transition,
};
pub use output::{
    CandidateContentRepairPolicy, ContentHash, CrateName, OutputContract, OutputMode, OutputPreset,
    OutputProjectionHint, OutputSchemaDialect, OutputSchemaRef, OutputValidatorRef,
    ProviderHintPolicy, RepairAdapterRef, RepairExhaustedBehavior, RepairPolicy, RetryBackoff,
    RetryBudget, SchemaVersion, SemanticValidatorRef, TypeName, ValidationFailureVisibility,
    ValidationPolicy,
};
pub use output_delivery::{
    OutputContentMode, OutputDeliveryDedupeRecord, OutputDeliveryEventKind,
    OutputDeliveryEventRecord, OutputDeliveryId, OutputDeliveryIntentRecord,
    OutputDeliveryJournalBase, OutputDeliveryKind, OutputDeliveryPolicy, OutputDeliveryReceipt,
    OutputDeliveryReconciliationRecord, OutputDeliveryRecord, OutputDeliveryRequest,
    OutputDeliveryRequirement, OutputDeliveryResultRecord, OutputDispatchStatus, OutputSinkRef,
    RawOutputContentPolicy, ReplayRepairDecision, TerminalAppendStatus,
    build_output_delivery_dedupe_key,
};
pub use output_delivery_port::{
    OutputSink as OutputDeliverySink, OutputSinkCapabilities,
    OutputSinkRegistry as OutputDeliverySinkRegistry,
};
pub use output_delivery_service::{
    OutputDedupeProof, OutputDeliveryCandidate, OutputDeliveryContext, OutputDeliveryDedupeIndex,
    OutputDeliveryOutcome, OutputDeliveryService,
};
pub use package::realtime::{
    REALTIME_SESSION_SIDECAR_KIND, REALTIME_SESSION_SIDECAR_VERSION, RealtimeSessionSidecar,
};
pub use package::stream::{
    STREAM_RULE_SIDECAR_KIND, STREAM_RULE_SIDECAR_VERSION, StreamRuleSidecar,
};
pub use package::tool_pack::{
    AnchorValidationRequirement, PreviewApplyRequirement, ResourceRouteSnapshot,
    ToolDiscoverySnapshot, ToolPackId, ToolPackKind, ToolPackSnapshot, ToolPackToolSnapshot,
    WorkspaceBoundsSnapshot, active_tool_pack_readiness,
};
pub use package::{
    AgentSnapshot, CapabilityCatalogSnapshot, ChildLifecyclePolicySnapshot,
    ChildPackageStripManifest, ChildRuntimePackage, ChildRuntimePackagePolicy,
    ContextHandoffPolicy, DepthBudget, FingerprintExclusionGroup, FingerprintInputGroup,
    FingerprintInputManifest, OutputContractSnapshot, OutputSinkSnapshot, PackageDelta,
    PackageSidecarSnapshot, PolicySnapshot, ProviderCapabilitySnapshot, ProviderRouteSnapshot,
    ReadinessProfile, RouteInheritanceMode, RuntimePackage, RuntimePackageBuilder,
    RuntimePackageCanonicalV1, RuntimePackageConformanceReport, RuntimePackageFingerprint,
    SubagentRoutePolicy, SubagentToolPolicy, VolatileRuntimeFields, build_child_runtime_package,
};
pub use package_extension::{
    CoreExtensionCapabilities, CoreExtensionCapabilitiesBuilder, EXTENSION_ACTION_SIDECAR_KIND,
    EXTENSION_ACTION_SIDECAR_VERSION, ExtensionActionCapability, ExtensionActionId,
    ExtensionActionIdempotency, ExtensionActionKind, ExtensionActionRef, ExtensionActionRequestId,
    ExtensionBridgeRef, ExtensionHookCapability, ExtensionId, ExtensionManifestAudit,
    ExtensionPackageCapability, ExtensionPackageResolution, ExtensionProviderCapability,
    ExtensionSubagentCapability, ExtensionToolCapability, ExtensionVersion,
    ResolvedExtensionActionSidecar, ResolvedExtensionPackage, audit_core_extension_capabilities,
};
pub use package_hooks::{
    ApprovalRequestPatch, CleanupRepairRequest, CompactionRequest, ContextInjectionRequest,
    DenyReason, DetachValidationRequest, HookCancellationToken, HookConfig, HookExecutionMode,
    HookExecutorRef, HookFailurePolicy, HookId, HookInput, HookMutationRight, HookMutationRights,
    HookOrdering, HookOrderingPhase, HookOverflowPolicy, HookPoint, HookPrivacyPolicy,
    HookQueueConfig, HookResponse, HookResponseClass, HookSource, HookSpec, HookTimeoutPolicy,
    HookView, ProjectionAuditRepairRequest, ProjectionPatch, RepairNeededReason, RetryRequest,
    StopReason, SubagentRequestPatch, ToolRequestPatch, ToolResultPatch, UsageRollupRepairRequest,
    ValidationHintPatch, hook_policy_ref, lower_code_hook, ordered_hooks_for_point,
    validate_hook_specs,
};
pub use package_isolation::{
    AmbientSecretPolicy, AuditabilityRequirement, ChildArtifactId, ChildShutdownBehavior,
    CleanupGuaranteeRequirement, CleanupMode, CleanupPlanRef, ContentRefMode,
    DataResidencyRequirement, DetachPolicy, EnvironmentLifecyclePolicy, EnvironmentSpec,
    ExecutionEnvironment, ExecutionEnvironmentBuilder, ExecutionEnvironmentId,
    ExecutionEnvironmentKind, FilesystemIsolationPolicy, ImageRef, ImageRequest, IsolatedProcessId,
    IsolatedProcessRef, IsolatedProcessSpec, IsolatedProcessSpecBuilder,
    IsolationAdapterRequirement, IsolationAdapterSessionRef, IsolationCapability,
    IsolationCapabilityReportRef, IsolationCapabilitySet, IsolationClass, IsolationFallback,
    IsolationFingerprintFields, IsolationRequirement, IsolationRequirementRef,
    IsolationRequirementSnapshot, IsolationRuntimeRef, IsolationSessionId, IsolationSessionRef,
    IsolationTrustField, IsolationTrustRequirement, LocalityRequirement, MountExpansionAudit,
    MountMode, MountPolicy, MountRef, NetworkIsolationPolicy, NetworkNamespaceRef,
    PolicyDecisionRef, ProcessContentCaptureMode, ProcessIoCapturePolicy, ProcessIoPolicy,
    ProcessIoStreamRef, ProcessOwnershipClass, ProcessOwnershipPolicy, ProcessStatsPolicy,
    ProcessStatsSnapshotRef, ReclaimPolicy, ReclaimTicketRef, RedactedEnvVar, ResourceLimits,
    RootFilesystemMode, RootfsRef, RootfsRequest, RunChildLifecyclePolicyRef,
    RuntimePackageSidecarId, SecretEnvPolicy, SecretExposurePolicy, SecretIsolationRequirement,
    SecretMountPolicy, SecretMountRef, SecretRef, SingleFileMountExpansionPolicy, StdinPolicy,
    SymlinkPolicy, TenantBoundaryRequirement, TerminalMode, TruncationPolicy, WorkspaceMountMode,
    WorkspaceMountPolicy,
};
pub use policy::{
    ApprovalDecisionKind, ApprovalPolicy, ApprovalRequestSpec, CapabilityPermission,
    ContentCaptureMode, ContentCapturePolicy, DecisionReason, DispatcherScope, EffectClass,
    EscalationPolicy, MissingDependency, PermissionPolicy, PolicyDecision, PolicyOutcome,
    PolicyStage, PrivacyPolicy, ResumePolicy, RiskClass, SandboxMode, SandboxPolicy,
    ToolRequestModification,
};
pub use ports::realtime::{
    RealtimeAdapterAck, RealtimeAdapterCall, RealtimeConnectRequest, RealtimeConnectResponse,
    RealtimeProviderAdapter,
};
pub use ports::{
    InMemoryRuntimePackageResolver, OutputSinkPort, OutputSinkRegistry, ProviderRegistry,
    RuntimePackageResolver, RuntimePolicyPort,
};
pub use ports_isolation::{
    CleanupRequest, CleanupResult, CleanupStatus, DetachTransferRequest, DetachTransferResult,
    EnvironmentPrepareRequest, ImageResolution, ImageResolveRequest, IsolationCapabilityReport,
    IsolationRuntime, IsolationRuntimeHealth, IsolationRuntimeKind, IsolationRuntimeRegistry,
    MountPlan, MountResolveRequest, NetworkPrepareRequest, PlatformReport, ProcessIoFrame,
    ProcessIoRequest, ProcessIoStream, ProcessSignal, ProcessSignalRequest, ProcessSignalResult,
    ProcessStartRequest, ProcessStartResult, ProcessStatsRequest, ProcessStatsSnapshot,
    ReclaimRequest, ReclaimResult, RootfsPrepareRequest, SecretMaterializationPlan,
    SecretPrepareRequest, SessionPrepareRequest, isolation_host_configuration_needed,
};
pub use projection::project_context_projection;
pub use provider::{
    ProviderAdapter, ProviderCapabilities, ProviderConformanceCase, ProviderMessage,
    ProviderMessageRole, ProviderModality, ProviderProjectedMetadata, ProviderProjectionPolicy,
    ProviderRequest, ProviderResponse, ProviderStopReason, ProviderStreamChunk,
    ProviderStreamDelta, ProviderStructuredOutputHint, ProviderUsage,
};
pub use realtime::{RealtimeCompletionGate, RealtimeSessionController};
pub use realtime_records::{
    RealtimeBackpressureAction, RealtimeBackpressureState, RealtimeCloseReason,
    RealtimeConnectionId, RealtimeFrameId, RealtimeInputFrame, RealtimeMediaKind,
    RealtimeOutputFrame, RealtimeResponseId, RealtimeSessionId, RealtimeSessionRecord,
    RealtimeSessionRecordKind, RealtimeSessionState, RealtimeSessionStatus,
};
pub use records_isolation::{
    ISOLATION_RECORD_SCHEMA_VERSION, IsolationCapabilityMatchRecord,
    IsolationCapabilityReportedRecord, IsolationCleanupIntentRecord, IsolationCleanupResultRecord,
    IsolationDowngradeDecisionRecord, IsolationEnvironmentPrepareIntentRecord,
    IsolationEnvironmentPrepareResultRecord, IsolationEventBase, IsolationEventKind,
    IsolationEventRecord, IsolationFailureRecord, IsolationNetworkPreparedRecord,
    IsolationProcessStartIntentRecord, IsolationProcessStartResultRecord,
    IsolationProcessStatsRecord, IsolationRecord, IsolationRequestedRecord,
};
pub use recovery::{
    RecoveryAction, RecoveryClassification, RecoveryDecision, RecoveryFailureKind,
    classify_recovery,
};
pub use repair::{
    LocalValidationRepairService, RepairAccounting, RepairDecision, RepairPolicyController,
    ValidationRepairOutcome,
};
pub use replay::{
    CursorCompatibility, DurableReplaySupport, ReplayMode, ReplayPendingSideEffect, ReplayReducer,
    ReplayRepairKind, ReplayRepairNeeded, ReplayResult, ReplayStatus, check_cursor_compatibility,
    durable_replay_support,
};
pub use run::{RunRequest, RunResult, RunStatus, StructuredOutputArtifacts};
pub use run_handle::{InMemoryRunControlStore, RunControlStore, RunHandle};
pub use runtime::{
    AgentRuntime, AgentRuntimeBuilder, CancellationHandle, EffectiveRuntimePackage,
    RunRegistryStatus, RunSnapshot,
};
pub use stream::{StreamRuleEngine, StreamRuleEngineState};
pub use stream_records::{
    MarkerId, MarkerVersion, MatchPrivacyPolicy, MatcherEngineRef, PartialOutputPolicy,
    RedactedMatch, RegexDialect, RepeatPolicy, RuleVersion, StreamAction, StreamChannel,
    StreamChannelSelector, StreamCursor, StreamCursorPrecision, StreamDelta, StreamDeltaId,
    StreamDirection, StreamIntervention, StreamInterventionId, StreamMatchId, StreamMatchRef,
    StreamMatcher, StreamRule, StreamRuleBuilder, StreamRuleId, StreamRuleRecord,
    StreamRuleRecordKind, StreamRuleRepeatStateSnapshot, StreamRuleScope, stream_policy_ref,
};
pub use structured_output::{
    RepairExhaustionRecord, RepairPrompt, RepairPromptCandidateContent, RepairRecord,
    RepairRecordKind, StructuredOutputLifecycleKind, StructuredOutputLifecycleRecord,
    ValidationErrorCode, ValidationErrorSummary, ValidationRecord, ValidationRecordKind,
};
pub use subagent::{
    ChildRunHandle, SubagentRequest, SubagentRequestId, SubagentSupervisor,
    subagent_runtime_event_frame,
};
pub use subagent_records::{
    ChildArtifactKind, ChildLifecycleAction, ChildLifecycleRecord, ChildLifecycleStatus,
    RunJournalRef, SubagentCompletedRecord, SubagentHandoffRecord, SubagentRecord,
    SubagentStartedRecord, SubagentTerminalStatus, SubagentUsageRolledUpRecord,
    SubagentWrappedEventRecord,
};
pub use subscription::{InMemorySubscriptionHub, RunSubscriptionSource};
pub use telemetry::{
    TelemetryAuthorityBoundary, TelemetryContentCaptureDecision, TelemetryContentCaptureRequest,
    TelemetryDrainReport, TelemetryFanout, TelemetryFanoutConfig, TelemetryFanoutReport,
    TelemetryOverflowPolicy, TelemetrySinkIsolationPolicy, TelemetryUsageExtractionInput,
    TelemetryUsageExtractor, evaluate_content_capture, sink_health_projection,
    terminal_run_projection_from_event,
};
pub use telemetry_ports::{TelemetrySink, TelemetrySinkAck, TelemetrySinkError, TelemetrySinkSpec};
pub use telemetry_records::{
    CostEstimateStatus, CostTelemetryRecord, CostUnits, RateTableVersion,
    TelemetryContentCaptureMode, TelemetryCostRecordId, TelemetryExportAttemptId,
    TelemetryExportCursor, TelemetryExportCursorRecord, TelemetryProjection, TelemetryProjectionId,
    TelemetryProjectionKind, TelemetryRecord, TelemetryRecordId, TelemetryRecordPayload,
    TelemetrySinkFailureKind, TelemetrySinkFailureRecord, TelemetrySinkHealth,
    TelemetrySinkHealthState, TelemetrySinkId, TelemetrySinkKind, TelemetrySinkRecoveryRecord,
    TelemetrySourceCursor, TelemetrySourceRecord, TelemetryTerminalStatus, TelemetryUsageRecordId,
    UsageTelemetryRecord, UsageUnits,
};
pub use tool_execution::{ToolExecutionContext, ToolExecutionCoordinator, ToolExecutionOutcome};
pub use tool_pack_ports::{
    ResourceReadRequest, ResourceResolution, ResourceResolver, ResourceRouter, ResourceScheme,
};
pub use tool_pack_records::{
    ShellProcessLineage, ToolDiscoveryLineage, ToolPackEffectLineage, WorkspaceMutationLineage,
    WorkspaceReadLineage,
};
pub use tool_ports::{
    AllowToolPolicy, ResolvedToolCall, ToolCallRequest, ToolExecutionOutput, ToolExecutionRequest,
    ToolExecutionStrategy, ToolExecutor, ToolExecutorRegistry, ToolPolicyPort,
    ToolRegistrySnapshot, ToolRoute, ToolRouter, allowed_tool_policy_outcome,
};
pub use tool_records::{
    CanonicalToolName, ToolCallRecord, ToolCallRecordParams, ToolCallRecordStatus, ToolResultRef,
    tool_call_journal_record,
};
pub use typed_output_ports::{TypedOutputDeserializer, TypedOutputModel};
pub use validated_output::{
    DecodedTypedOutput, OutputLineage, StructuredOutputResult, TypedOutputError,
    TypedResultPublicationRecord, TypedResultPublicationStatus, ValidatedOutput,
    ValidatedOutputParams, ValidatedOutputPublicationStep, ValidationReportRecord,
    ValidationReportRef, ValidationStatus, validate_typed_result_publication_order,
};
pub use validation::{
    HostileSchemaLimits, JsonSchemaSubsetValidator, OutputCandidate, StructuredOutputValidator,
    TerminalValidationFailure, ValidationErrorReport, ValidationSuccess,
};
