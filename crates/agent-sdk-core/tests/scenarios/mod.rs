use agent_sdk_core::{
    DestinationKind, EffectKind, EventFamily, EventKind, JournalRecordKind, SourceKind,
    testing::{normalize_json_value, read_fixture},
};
use serde_json::{Value, json};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FakePort {
    FakeProvider,
    ScriptedApprovalDispatcher,
    ScriptedToolExecutor,
    ScriptedOutputSink,
    FakeRemoteChannel,
    ScriptedRealtimeAdapter,
    FakeMemoryPort,
    InMemoryCheckpointStore,
    FakeIsolationRuntime,
    FakeAntiEntropyScanner,
    FakeSubagentRuntime,
    ScriptedExtensionActionExecutor,
    InMemoryEventBus,
    InMemoryJournal,
    ScriptedTelemetrySink,
}

impl FakePort {
    fn as_str(self) -> &'static str {
        match self {
            Self::FakeProvider => "fake_provider",
            Self::ScriptedApprovalDispatcher => "scripted_approval_dispatcher",
            Self::ScriptedToolExecutor => "scripted_tool_executor",
            Self::ScriptedOutputSink => "scripted_output_sink",
            Self::FakeRemoteChannel => "fake_remote_channel",
            Self::ScriptedRealtimeAdapter => "scripted_realtime_adapter",
            Self::FakeMemoryPort => "fake_memory_port",
            Self::InMemoryCheckpointStore => "in_memory_checkpoint_store",
            Self::FakeIsolationRuntime => "fake_isolation_runtime",
            Self::FakeAntiEntropyScanner => "fake_anti_entropy_scanner",
            Self::FakeSubagentRuntime => "fake_subagent_runtime",
            Self::ScriptedExtensionActionExecutor => "scripted_extension_action_executor",
            Self::InMemoryEventBus => "in_memory_event_bus",
            Self::InMemoryJournal => "in_memory_journal",
            Self::ScriptedTelemetrySink => "scripted_telemetry_sink",
        }
    }

    fn is_fake_or_in_memory(self) -> bool {
        matches!(
            self,
            Self::FakeProvider
                | Self::ScriptedApprovalDispatcher
                | Self::ScriptedToolExecutor
                | Self::ScriptedOutputSink
                | Self::FakeRemoteChannel
                | Self::ScriptedRealtimeAdapter
                | Self::FakeMemoryPort
                | Self::InMemoryCheckpointStore
                | Self::FakeIsolationRuntime
                | Self::FakeAntiEntropyScanner
                | Self::FakeSubagentRuntime
                | Self::ScriptedExtensionActionExecutor
                | Self::InMemoryEventBus
                | Self::InMemoryJournal
                | Self::ScriptedTelemetrySink
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StepKind {
    SdkDecision,
    ProjectionAudit,
    Approval,
    Validation,
    StreamIntervention,
    JournalAppend,
    FakePortCall,
    LiveEmit,
    DurableExport,
    HostBoundary,
    Recovery,
}

#[derive(Clone, Debug)]
struct ScenarioStep {
    kind: StepKind,
    label: &'static str,
    port: FakePort,
    effect_kind: Option<EffectKind>,
    host_owned: bool,
}

impl ScenarioStep {
    fn new(kind: StepKind, label: &'static str, port: FakePort) -> Self {
        Self {
            kind,
            label,
            port,
            effect_kind: None,
            host_owned: false,
        }
    }

    fn effect(mut self, kind: EffectKind) -> Self {
        self.effect_kind = Some(kind);
        self
    }

    fn host_owned(mut self) -> Self {
        self.host_owned = true;
        self
    }
}

#[derive(Clone, Debug)]
struct Scenario {
    workflow: &'static str,
    source_kind: SourceKind,
    destination_kind: DestinationKind,
    sdk_primitives: Vec<&'static str>,
    host_owned_boundaries: Vec<&'static str>,
    event_families: Vec<EventFamily>,
    event_kinds: Vec<EventKind>,
    journal_records: Vec<JournalRecordKind>,
    effect_kinds: Vec<EffectKind>,
    steps: Vec<ScenarioStep>,
}

#[test]
fn scenario_matrix_maps_generic_workflows_to_sdk_primitives_and_host_boundaries() {
    let summary = normalize_json_value(json!({
        "schema_version": 1,
        "matrix": scenarios().iter().map(matrix_row).collect::<Vec<_>>(),
    }));

    assert_eq!(
        summary,
        read_fixture("tests/fixtures/scenarios/scenario-matrix-v1.json")
            .expect("scenario matrix fixture")
    );
}

#[test]
fn scripted_scenarios_use_only_fake_in_memory_or_scripted_ports() {
    for scenario in scenarios() {
        assert!(
            scenario
                .steps
                .iter()
                .all(|step| step.port.is_fake_or_in_memory()),
            "{} used a non-fake port",
            scenario.workflow
        );
        assert!(
            scenario
                .steps
                .iter()
                .any(|step| step.port == FakePort::InMemoryJournal),
            "{} must include an in-memory journal proof surface",
            scenario.workflow
        );
    }
}

#[test]
fn side_effecting_scenarios_journal_intent_before_fake_port_execution() {
    for scenario in scenarios() {
        for effect_kind in scenario.executed_effect_kinds() {
            let intent_position = scenario.steps.iter().position(|step| {
                step.kind == StepKind::JournalAppend
                    && step.effect_kind.as_ref() == Some(&effect_kind)
            });
            let execution_position = scenario.steps.iter().position(|step| {
                step.requires_journaled_intent() && step.effect_kind.as_ref() == Some(&effect_kind)
            });

            if let Some(execution_position) = execution_position {
                let intent_position = intent_position.unwrap_or_else(|| {
                    panic!(
                        "{} executes {:?} without a prior journaled effect intent",
                        scenario.workflow, &effect_kind
                    )
                });
                assert!(
                    intent_position < execution_position,
                    "{} must journal {:?} intent before fake port execution",
                    scenario.workflow,
                    &effect_kind
                );
            }
        }
    }
}

#[test]
fn scenario_ordering_preserves_policy_projection_validation_and_recovery_boundaries() {
    let scenarios = scenarios();

    assert_before(
        &scenarios,
        "desktop_web_chat_approval",
        StepKind::ProjectionAudit,
        StepKind::FakePortCall,
        Some(EffectKind::ProviderRequest),
        "context projection audit must precede provider calls",
    );
    assert_before(
        &scenarios,
        "desktop_web_chat_approval",
        StepKind::Approval,
        StepKind::FakePortCall,
        Some(EffectKind::ToolExecution),
        "approval must release tool execution",
    );
    assert_before(
        &scenarios,
        "structured_output",
        StepKind::Validation,
        StepKind::FakePortCall,
        Some(EffectKind::OutputDelivery),
        "validated output must precede output sink publication",
    );
    assert_before(
        &scenarios,
        "realtime_stream_safeguard",
        StepKind::StreamIntervention,
        StepKind::FakePortCall,
        Some(EffectKind::ProviderRequest),
        "stream intervention must gate realtime/tool output delivery",
    );
    assert_before(
        &scenarios,
        "memory_context_compaction",
        StepKind::ProjectionAudit,
        StepKind::FakePortCall,
        Some(EffectKind::ProviderRequest),
        "compaction must rebuild projection before provider resume",
    );
    assert_before(
        &scenarios,
        "tool_pack_isolation_repair",
        StepKind::Recovery,
        StepKind::DurableExport,
        None,
        "anti-entropy repair must reconcile durable records before telemetry export",
    );
}

#[test]
fn product_specific_behavior_stays_out_of_sdk_owned_scenario_surfaces() {
    let forbidden = [
        "clawdia",
        "codex",
        "chatgpt",
        "slack",
        "discord",
        "gmail",
        "imessage",
        "vercel",
        "github",
        "linear",
        "notion",
        "marketplace",
    ];

    for scenario in scenarios() {
        for primitive in &scenario.sdk_primitives {
            assert_no_forbidden_product_term(scenario.workflow, "primitive", primitive, &forbidden);
        }
        for step in scenario.steps.iter().filter(|step| !step.host_owned) {
            assert_no_forbidden_product_term(scenario.workflow, "sdk step", step.label, &forbidden);
        }
    }
}

#[test]
fn host_boundaries_are_explicit_and_never_marked_as_sdk_authority() {
    for scenario in scenarios() {
        assert!(
            !scenario.host_owned_boundaries.is_empty(),
            "{} must name host-owned boundaries",
            scenario.workflow
        );
        assert!(
            scenario
                .steps
                .iter()
                .filter(|step| step.kind == StepKind::HostBoundary)
                .all(|step| step.host_owned),
            "{} has a host boundary step marked as SDK-owned",
            scenario.workflow
        );
    }
}

#[test]
fn live_event_flow_never_substitutes_display_history_for_durable_truth() {
    let scenarios = scenarios();
    let scenario = scenario("live_vs_durable_event_flow", &scenarios);
    let journal_position = position(scenario, StepKind::JournalAppend);
    let live_position = position(scenario, StepKind::LiveEmit);
    let export_position = position(scenario, StepKind::DurableExport);
    let host_display_position = scenario
        .steps
        .iter()
        .position(|step| step.label == "host display event drops from bounded store")
        .expect("display drop is modeled");

    assert!(
        journal_position < live_position,
        "live frames are projections after durable journal append"
    );
    assert!(
        host_display_position < export_position,
        "trace export must continue after display-event loss"
    );
    assert!(
        scenario
            .host_owned_boundaries
            .contains(&"bounded display event store"),
        "display storage remains host-owned"
    );
}

fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario {
            workflow: "desktop_web_chat_approval",
            source_kind: SourceKind::Host,
            destination_kind: DestinationKind::OutputSink,
            sdk_primitives: vec![
                "AgentRuntime",
                "RunRequest",
                "ContextAssembler",
                "RuntimePackage",
                "ProviderAdapter",
                "ApprovalBroker",
                "ToolExecutor",
                "OutputSink",
            ],
            host_owned_boundaries: vec![
                "desktop or web chat UI",
                "approval prompt copy",
                "conversation store",
                "display event transport",
            ],
            event_families: vec![
                EventFamily::Run,
                EventFamily::Context,
                EventFamily::Model,
                EventFamily::Approval,
                EventFamily::Tool,
                EventFamily::OutputDelivery,
            ],
            event_kinds: vec![
                EventKind::RunStarted,
                EventKind::ProviderRequestProjected,
                EventKind::ApprovalRequested,
                EventKind::ToolStarted,
                EventKind::OutputDispatchCompleted,
            ],
            journal_records: vec![
                JournalRecordKind::Run,
                JournalRecordKind::Context,
                JournalRecordKind::ModelAttempt,
                JournalRecordKind::Approval,
                JournalRecordKind::Tool,
                JournalRecordKind::OutputDispatch,
            ],
            effect_kinds: vec![
                EffectKind::ProviderRequest,
                EffectKind::ApprovalDispatch,
                EffectKind::ToolExecution,
                EffectKind::OutputDelivery,
            ],
            steps: vec![
                ScenarioStep::new(
                    StepKind::JournalAppend,
                    "run started and context projection journaled",
                    FakePort::InMemoryJournal,
                ),
                ScenarioStep::new(
                    StepKind::ProjectionAudit,
                    "context projection audited before provider request",
                    FakePort::InMemoryJournal,
                ),
                ScenarioStep::new(
                    StepKind::JournalAppend,
                    "provider request intent journaled after projection audit",
                    FakePort::InMemoryJournal,
                )
                .effect(EffectKind::ProviderRequest),
                ScenarioStep::new(
                    StepKind::FakePortCall,
                    "projected request streamed by fake provider",
                    FakePort::FakeProvider,
                )
                .effect(EffectKind::ProviderRequest),
                ScenarioStep::new(
                    StepKind::JournalAppend,
                    "approval dispatch intent journaled",
                    FakePort::InMemoryJournal,
                )
                .effect(EffectKind::ApprovalDispatch),
                ScenarioStep::new(
                    StepKind::Approval,
                    "approval broker receives finite host decision",
                    FakePort::ScriptedApprovalDispatcher,
                ),
                ScenarioStep::new(
                    StepKind::JournalAppend,
                    "approved tool execution intent journaled",
                    FakePort::InMemoryJournal,
                )
                .effect(EffectKind::ToolExecution),
                ScenarioStep::new(
                    StepKind::FakePortCall,
                    "approved tool executed by scripted executor",
                    FakePort::ScriptedToolExecutor,
                )
                .effect(EffectKind::ToolExecution),
                ScenarioStep::new(
                    StepKind::JournalAppend,
                    "output delivery intent journaled with dedupe key",
                    FakePort::InMemoryJournal,
                )
                .effect(EffectKind::OutputDelivery),
                ScenarioStep::new(
                    StepKind::FakePortCall,
                    "final answer delivered by scripted output sink",
                    FakePort::ScriptedOutputSink,
                )
                .effect(EffectKind::OutputDelivery),
                ScenarioStep::new(
                    StepKind::HostBoundary,
                    "host renders chat transcript and approval prompt",
                    FakePort::InMemoryEventBus,
                )
                .host_owned(),
            ],
        },
        Scenario {
            workflow: "cli_headless_approval",
            source_kind: SourceKind::ScheduledTask,
            destination_kind: DestinationKind::OutputSink,
            sdk_primitives: vec![
                "SourceRef",
                "DestinationRef",
                "EscalationPolicy",
                "ApprovalBroker",
                "ToolExecutor",
                "OutputSink",
            ],
            host_owned_boundaries: vec![
                "terminal prompt",
                "scheduler",
                "escalation manager",
                "approval transport",
            ],
            event_families: vec![EventFamily::Run, EventFamily::Approval, EventFamily::Tool],
            event_kinds: vec![
                EventKind::ApprovalDispatchUnavailable,
                EventKind::ApprovalDenied,
                EventKind::ToolDenied,
            ],
            journal_records: vec![
                JournalRecordKind::Run,
                JournalRecordKind::Approval,
                JournalRecordKind::Tool,
            ],
            effect_kinds: vec![EffectKind::ApprovalDispatch],
            steps: vec![
                ScenarioStep::new(
                    StepKind::JournalAppend,
                    "headless source and destination recorded",
                    FakePort::InMemoryJournal,
                ),
                ScenarioStep::new(
                    StepKind::JournalAppend,
                    "approval dispatch intent journaled",
                    FakePort::InMemoryJournal,
                )
                .effect(EffectKind::ApprovalDispatch),
                ScenarioStep::new(
                    StepKind::Approval,
                    "missing dispatcher denies by default",
                    FakePort::ScriptedApprovalDispatcher,
                ),
                ScenarioStep::new(
                    StepKind::SdkDecision,
                    "tool release is denied without ambient fail-open",
                    FakePort::InMemoryJournal,
                ),
                ScenarioStep::new(
                    StepKind::HostBoundary,
                    "host may provide separate escalation manager",
                    FakePort::ScriptedApprovalDispatcher,
                )
                .host_owned(),
            ],
        },
        Scenario {
            workflow: "remote_output_dedupe",
            source_kind: SourceKind::RemoteChannel,
            destination_kind: DestinationKind::RemoteChannel,
            sdk_primitives: vec![
                "RemoteChannelAdapter",
                "RunRequest",
                "DestinationRef",
                "OutputSink",
                "DedupeKey",
                "RunJournal",
            ],
            host_owned_boundaries: vec![
                "remote channel transport",
                "message database",
                "ack lookup",
                "retry scheduler",
            ],
            event_families: vec![EventFamily::Run, EventFamily::OutputDelivery],
            event_kinds: vec![
                EventKind::OutputDispatchRequested,
                EventKind::OutputDispatchDeduped,
                EventKind::OutputDispatchCompleted,
            ],
            journal_records: vec![
                JournalRecordKind::Run,
                JournalRecordKind::OutputDispatch,
                JournalRecordKind::Recovery,
            ],
            effect_kinds: vec![EffectKind::OutputDelivery],
            steps: vec![
                ScenarioStep::new(
                    StepKind::JournalAppend,
                    "remote inbound source ref and output sink destination recorded",
                    FakePort::InMemoryJournal,
                ),
                ScenarioStep::new(
                    StepKind::JournalAppend,
                    "output delivery intent journaled with stable dedupe key",
                    FakePort::InMemoryJournal,
                )
                .effect(EffectKind::OutputDelivery),
                ScenarioStep::new(
                    StepKind::FakePortCall,
                    "remote reply send attempted through fake channel",
                    FakePort::FakeRemoteChannel,
                )
                .effect(EffectKind::OutputDelivery),
                ScenarioStep::new(
                    StepKind::Recovery,
                    "replay sees completed dedupe key and avoids resend",
                    FakePort::InMemoryJournal,
                ),
                ScenarioStep::new(
                    StepKind::HostBoundary,
                    "host owns channel credentials and ack persistence",
                    FakePort::FakeRemoteChannel,
                )
                .host_owned(),
            ],
        },
        Scenario {
            workflow: "realtime_stream_safeguard",
            source_kind: SourceKind::Extension,
            destination_kind: DestinationKind::OutputSink,
            sdk_primitives: vec![
                "RealtimeSessionSidecar",
                "RealtimeProviderAdapter",
                "StreamDelta",
                "StreamRuleEngine",
                "StreamIntervention",
                "ApprovalBroker",
                "ContentRef",
            ],
            host_owned_boundaries: vec![
                "microphone permission",
                "wake or listening UI",
                "audio rendering",
                "approval token transport",
            ],
            event_families: vec![
                EventFamily::Realtime,
                EventFamily::StreamRule,
                EventFamily::Approval,
                EventFamily::OutputDelivery,
            ],
            event_kinds: vec![
                EventKind::RealtimeRestartRequested,
                EventKind::RealtimeRestartStarted,
                EventKind::StreamRuleMatched,
                EventKind::StreamInterventionApplied,
                EventKind::RealtimeRestartCompleted,
            ],
            journal_records: vec![
                JournalRecordKind::RealtimeSession,
                JournalRecordKind::StreamRule,
                JournalRecordKind::Approval,
                JournalRecordKind::OutputDispatch,
            ],
            effect_kinds: vec![EffectKind::ProviderRequest, EffectKind::OutputDelivery],
            steps: vec![
                ScenarioStep::new(
                    StepKind::JournalAppend,
                    "realtime session and restart request recorded",
                    FakePort::InMemoryJournal,
                )
                .effect(EffectKind::ProviderRequest),
                ScenarioStep::new(
                    StepKind::StreamIntervention,
                    "stream rule masks unsafe transcript chunk before sink",
                    FakePort::InMemoryJournal,
                ),
                ScenarioStep::new(
                    StepKind::FakePortCall,
                    "realtime restart uses scripted adapter after outbound gate",
                    FakePort::ScriptedRealtimeAdapter,
                )
                .effect(EffectKind::ProviderRequest),
                ScenarioStep::new(
                    StepKind::JournalAppend,
                    "output delivery intent journaled after masked stream",
                    FakePort::InMemoryJournal,
                )
                .effect(EffectKind::OutputDelivery),
                ScenarioStep::new(
                    StepKind::FakePortCall,
                    "masked realtime output delivered by scripted sink",
                    FakePort::ScriptedOutputSink,
                )
                .effect(EffectKind::OutputDelivery),
                ScenarioStep::new(
                    StepKind::HostBoundary,
                    "host owns media capture and voice activity rendering",
                    FakePort::ScriptedRealtimeAdapter,
                )
                .host_owned(),
            ],
        },
        Scenario {
            workflow: "structured_output",
            source_kind: SourceKind::Host,
            destination_kind: DestinationKind::OutputSink,
            sdk_primitives: vec![
                "OutputContract",
                "StructuredOutputValidator",
                "ValidatedOutput",
                "StreamRuleEngine",
                "OutputSink",
                "EffectIntent",
            ],
            host_owned_boundaries: vec![
                "schema authoring UI",
                "business scoring",
                "form rendering",
                "sink credentials",
            ],
            event_families: vec![
                EventFamily::StructuredOutput,
                EventFamily::StreamRule,
                EventFamily::OutputDelivery,
            ],
            event_kinds: vec![
                EventKind::StructuredOutputValidationStarted,
                EventKind::StructuredOutputRepairRequested,
                EventKind::StructuredOutputValidated,
                EventKind::OutputDispatchCompleted,
            ],
            journal_records: vec![
                JournalRecordKind::StructuredOutput,
                JournalRecordKind::StreamRule,
                JournalRecordKind::OutputDispatch,
            ],
            effect_kinds: vec![EffectKind::ProviderRequest, EffectKind::OutputDelivery],
            steps: vec![
                ScenarioStep::new(
                    StepKind::JournalAppend,
                    "structured output request and schema ref recorded",
                    FakePort::InMemoryJournal,
                ),
                ScenarioStep::new(
                    StepKind::StreamIntervention,
                    "stream rule masks candidate text before validation",
                    FakePort::InMemoryJournal,
                ),
                ScenarioStep::new(
                    StepKind::Validation,
                    "local validator accepts repaired typed value",
                    FakePort::InMemoryJournal,
                ),
                ScenarioStep::new(
                    StepKind::JournalAppend,
                    "output delivery intent journaled after validation",
                    FakePort::InMemoryJournal,
                )
                .effect(EffectKind::OutputDelivery),
                ScenarioStep::new(
                    StepKind::FakePortCall,
                    "validated result delivered by scripted output sink",
                    FakePort::ScriptedOutputSink,
                )
                .effect(EffectKind::OutputDelivery),
                ScenarioStep::new(
                    StepKind::HostBoundary,
                    "host renders typed result and owns downstream workflow",
                    FakePort::ScriptedOutputSink,
                )
                .host_owned(),
            ],
        },
        Scenario {
            workflow: "memory_context_compaction",
            source_kind: SourceKind::Memory,
            destination_kind: DestinationKind::Provider,
            sdk_primitives: vec![
                "ContextContribution",
                "ContextAssembler",
                "ContextProjection",
                "ContextProjectionAudit",
                "MemoryPort",
                "CheckpointStore",
            ],
            host_owned_boundaries: vec![
                "memory backend",
                "memory browsing UI",
                "memory ingestion product",
                "extension proposal source",
            ],
            event_families: vec![
                EventFamily::Context,
                EventFamily::Recovery,
                EventFamily::Model,
            ],
            event_kinds: vec![
                EventKind::ContextAssembled,
                EventKind::RunCheckpointed,
                EventKind::ReplayCompleted,
                EventKind::ProviderRequestProjected,
            ],
            journal_records: vec![
                JournalRecordKind::Context,
                JournalRecordKind::Checkpoint,
                JournalRecordKind::Recovery,
                JournalRecordKind::ModelAttempt,
            ],
            effect_kinds: vec![EffectKind::ProviderRequest, EffectKind::MemoryWrite],
            steps: vec![
                ScenarioStep::new(
                    StepKind::FakePortCall,
                    "protected memory refs retrieved through fake memory port",
                    FakePort::FakeMemoryPort,
                ),
                ScenarioStep::new(
                    StepKind::JournalAppend,
                    "compaction checkpoint content ref manifest saved",
                    FakePort::InMemoryJournal,
                ),
                ScenarioStep::new(
                    StepKind::SdkDecision,
                    "checkpoint projection snapshot stored for deterministic resume",
                    FakePort::InMemoryCheckpointStore,
                ),
                ScenarioStep::new(
                    StepKind::ProjectionAudit,
                    "projection audit omits sensitive proposal before provider resume",
                    FakePort::InMemoryJournal,
                ),
                ScenarioStep::new(
                    StepKind::JournalAppend,
                    "provider resume intent journaled after compaction audit",
                    FakePort::InMemoryJournal,
                )
                .effect(EffectKind::ProviderRequest),
                ScenarioStep::new(
                    StepKind::FakePortCall,
                    "provider resumes from admitted context projection",
                    FakePort::FakeProvider,
                )
                .effect(EffectKind::ProviderRequest),
                ScenarioStep::new(
                    StepKind::HostBoundary,
                    "host owns memory browser and ingestion policy UI",
                    FakePort::FakeMemoryPort,
                )
                .host_owned(),
            ],
        },
        Scenario {
            workflow: "tool_pack_isolation_repair",
            source_kind: SourceKind::Tool,
            destination_kind: DestinationKind::ExternalRuntime,
            sdk_primitives: vec![
                "ToolPack",
                "CapabilitySpec",
                "HookSpec",
                "IsolationRuntime",
                "EffectIntent",
                "EffectResult",
                "AntiEntropyJob",
            ],
            host_owned_boundaries: vec![
                "installed tool packs",
                "workspace policy",
                "concrete runtime",
                "repair scheduler",
            ],
            event_families: vec![
                EventFamily::Tool,
                EventFamily::Isolation,
                EventFamily::Recovery,
                EventFamily::Telemetry,
            ],
            event_kinds: vec![
                EventKind::ToolStarted,
                EventKind::IsolationEnvironmentPrepared,
                EventKind::IsolationProcessStarted,
                EventKind::ToolRecoveryRequired,
                EventKind::UsageRecorded,
            ],
            journal_records: vec![
                JournalRecordKind::Tool,
                JournalRecordKind::Isolation,
                JournalRecordKind::EffectIntent,
                JournalRecordKind::EffectResult,
                JournalRecordKind::Recovery,
                JournalRecordKind::Telemetry,
            ],
            effect_kinds: vec![EffectKind::FileWrite, EffectKind::IsolatedProcessStart],
            steps: vec![
                ScenarioStep::new(
                    StepKind::SdkDecision,
                    "stale anchored edit denied before write",
                    FakePort::ScriptedToolExecutor,
                ),
                ScenarioStep::new(
                    StepKind::JournalAppend,
                    "isolated process start intent journaled",
                    FakePort::InMemoryJournal,
                )
                .effect(EffectKind::IsolatedProcessStart),
                ScenarioStep::new(
                    StepKind::FakePortCall,
                    "process starts through fake isolation runtime",
                    FakePort::FakeIsolationRuntime,
                )
                .effect(EffectKind::IsolatedProcessStart),
                ScenarioStep::new(
                    StepKind::Recovery,
                    "anti-entropy repairs missing terminal append",
                    FakePort::FakeAntiEntropyScanner,
                ),
                ScenarioStep::new(
                    StepKind::DurableExport,
                    "telemetry summary cursor re-exported from repaired journal",
                    FakePort::ScriptedTelemetrySink,
                ),
                ScenarioStep::new(
                    StepKind::HostBoundary,
                    "host owns workspace roots and runtime adapter installation",
                    FakePort::FakeIsolationRuntime,
                )
                .host_owned(),
            ],
        },
        Scenario {
            workflow: "subagent_supervision",
            source_kind: SourceKind::Subagent,
            destination_kind: DestinationKind::AgentPool,
            sdk_primitives: vec![
                "AgentPool",
                "RunMessage",
                "WakeCondition",
                "SubagentSupervisor",
                "SubagentRequest",
                "RuntimePackage",
                "ContextHandoffPolicy",
            ],
            host_owned_boundaries: vec![
                "child progress display",
                "conversation promotion",
                "provider route registry",
                "detached-child dashboard",
            ],
            event_families: vec![
                EventFamily::AgentPool,
                EventFamily::Subagent,
                EventFamily::ChildLifecycle,
            ],
            event_kinds: vec![
                EventKind::SubagentStarted,
                EventKind::RunMessageDelivered,
                EventKind::SubagentEventWrapped,
                EventKind::SubagentUsageRolledUp,
            ],
            journal_records: vec![
                JournalRecordKind::Subagent,
                JournalRecordKind::AgentPool,
                JournalRecordKind::RunMessage,
                JournalRecordKind::Wake,
                JournalRecordKind::ChildLifecycle,
            ],
            effect_kinds: vec![EffectKind::ChildAgentStart, EffectKind::RunMessageDelivery],
            steps: vec![
                ScenarioStep::new(
                    StepKind::JournalAppend,
                    "child agent start intent journaled with parent causal refs",
                    FakePort::InMemoryJournal,
                )
                .effect(EffectKind::ChildAgentStart),
                ScenarioStep::new(
                    StepKind::SdkDecision,
                    "child runtime package strips recursive subagent tools",
                    FakePort::InMemoryJournal,
                ),
                ScenarioStep::new(
                    StepKind::FakePortCall,
                    "child run starts through fake subagent runtime",
                    FakePort::FakeSubagentRuntime,
                )
                .effect(EffectKind::ChildAgentStart),
                ScenarioStep::new(
                    StepKind::JournalAppend,
                    "run message delivery intent journaled",
                    FakePort::InMemoryJournal,
                )
                .effect(EffectKind::RunMessageDelivery),
                ScenarioStep::new(
                    StepKind::FakePortCall,
                    "child event wrapped into parent stream",
                    FakePort::InMemoryEventBus,
                )
                .effect(EffectKind::RunMessageDelivery),
                ScenarioStep::new(
                    StepKind::HostBoundary,
                    "host display remains read-only and does not create a chat tab",
                    FakePort::InMemoryEventBus,
                )
                .host_owned(),
            ],
        },
        Scenario {
            workflow: "extension_action_boundary",
            source_kind: SourceKind::Extension,
            destination_kind: DestinationKind::Host,
            sdk_primitives: vec![
                "CoreExtensionCapabilities",
                "CapabilityCatalogSnapshot",
                "ApprovalBroker",
                "EffectIntent",
                "EffectResult",
                "ExtensionActionRecord",
            ],
            host_owned_boundaries: vec![
                "host manifest",
                "extension runtime",
                "trust store",
                "host action adapter",
            ],
            event_families: vec![EventFamily::Extension, EventFamily::Approval],
            event_kinds: vec![
                EventKind::ExtensionActionSubmitted,
                EventKind::ApprovalRequested,
                EventKind::ExtensionActionStarted,
                EventKind::ExtensionActionCompleted,
            ],
            journal_records: vec![
                JournalRecordKind::ExtensionAction,
                JournalRecordKind::Approval,
                JournalRecordKind::EffectIntent,
                JournalRecordKind::EffectResult,
                JournalRecordKind::Recovery,
            ],
            effect_kinds: vec![EffectKind::ApprovalDispatch, EffectKind::ExtensionAction],
            steps: vec![
                ScenarioStep::new(
                    StepKind::HostBoundary,
                    "host validates manifest and trust outside core",
                    FakePort::ScriptedExtensionActionExecutor,
                )
                .host_owned(),
                ScenarioStep::new(
                    StepKind::JournalAppend,
                    "extension approval dispatch intent journaled",
                    FakePort::InMemoryJournal,
                )
                .effect(EffectKind::ApprovalDispatch),
                ScenarioStep::new(
                    StepKind::Approval,
                    "extension cannot self-approve action",
                    FakePort::ScriptedApprovalDispatcher,
                )
                .effect(EffectKind::ApprovalDispatch),
                ScenarioStep::new(
                    StepKind::JournalAppend,
                    "extension action intent journaled after host approval",
                    FakePort::InMemoryJournal,
                )
                .effect(EffectKind::ExtensionAction),
                ScenarioStep::new(
                    StepKind::FakePortCall,
                    "host action route executed by scripted extension adapter",
                    FakePort::ScriptedExtensionActionExecutor,
                )
                .effect(EffectKind::ExtensionAction),
                ScenarioStep::new(
                    StepKind::JournalAppend,
                    "extension action terminal effect result journaled",
                    FakePort::InMemoryJournal,
                ),
            ],
        },
        Scenario {
            workflow: "live_vs_durable_event_flow",
            source_kind: SourceKind::Sdk,
            destination_kind: DestinationKind::Telemetry,
            sdk_primitives: vec![
                "AgentEventBus",
                "EventFrame",
                "EventCursor",
                "RunJournal",
                "TelemetrySink",
                "EventArchive",
            ],
            host_owned_boundaries: vec![
                "display event bridge",
                "bounded display event store",
                "trace store",
                "UI selectors",
            ],
            event_families: vec![
                EventFamily::Tool,
                EventFamily::Telemetry,
                EventFamily::Recovery,
            ],
            event_kinds: vec![
                EventKind::ToolStarted,
                EventKind::ToolCompleted,
                EventKind::UsageRecorded,
                EventKind::ReplayCompleted,
            ],
            journal_records: vec![
                JournalRecordKind::Tool,
                JournalRecordKind::Telemetry,
                JournalRecordKind::Recovery,
            ],
            effect_kinds: vec![EffectKind::ToolExecution],
            steps: vec![
                ScenarioStep::new(
                    StepKind::JournalAppend,
                    "tool started journal record is durable truth",
                    FakePort::InMemoryJournal,
                )
                .effect(EffectKind::ToolExecution),
                ScenarioStep::new(
                    StepKind::LiveEmit,
                    "live SDK event frame emitted with event cursor",
                    FakePort::InMemoryEventBus,
                ),
                ScenarioStep::new(
                    StepKind::HostBoundary,
                    "host display event drops from bounded store",
                    FakePort::InMemoryEventBus,
                )
                .host_owned(),
                ScenarioStep::new(
                    StepKind::DurableExport,
                    "trace export rebuilds from journal and telemetry cursor",
                    FakePort::ScriptedTelemetrySink,
                ),
                ScenarioStep::new(
                    StepKind::Recovery,
                    "UI reconnect uses journal cursor instead of display history",
                    FakePort::InMemoryJournal,
                ),
            ],
        },
    ]
}

impl Scenario {
    fn executed_effect_kinds(&self) -> Vec<EffectKind> {
        let mut effects = self
            .steps
            .iter()
            .filter(|step| step.requires_journaled_intent())
            .filter_map(|step| step.effect_kind.clone())
            .collect::<Vec<_>>();
        effects.sort_by_key(|effect| format!("{effect:?}"));
        effects.dedup();
        effects
    }
}

impl ScenarioStep {
    fn requires_journaled_intent(&self) -> bool {
        matches!(self.kind, StepKind::FakePortCall | StepKind::Approval)
            && self.effect_kind.is_some()
    }
}

fn matrix_row(scenario: &Scenario) -> Value {
    normalize_json_value(json!({
        "workflow": scenario.workflow,
        "source_kind": scenario.source_kind,
        "destination_kind": scenario.destination_kind,
        "sdk_primitives": scenario.sdk_primitives,
        "host_owned_boundaries": scenario.host_owned_boundaries,
        "event_families": scenario.event_families,
        "event_kinds": scenario.event_kinds,
        "journal_records": scenario.journal_records,
        "effect_kinds": scenario.effect_kinds,
        "fake_ports": scenario.steps.iter().map(|step| step.port.as_str()).collect::<Vec<_>>(),
    }))
}

fn scenario<'a>(workflow: &str, scenarios: &'a [Scenario]) -> &'a Scenario {
    scenarios
        .iter()
        .find(|scenario| scenario.workflow == workflow)
        .unwrap_or_else(|| panic!("missing scenario {workflow}"))
}

fn position(scenario: &Scenario, kind: StepKind) -> usize {
    scenario
        .steps
        .iter()
        .position(|step| step.kind == kind)
        .unwrap_or_else(|| panic!("{} missing step {:?}", scenario.workflow, kind))
}

fn assert_before(
    scenarios: &[Scenario],
    workflow: &str,
    before: StepKind,
    after: StepKind,
    after_effect: Option<EffectKind>,
    reason: &str,
) {
    let scenario = scenario(workflow, scenarios);
    let after_position = scenario
        .steps
        .iter()
        .position(|step| {
            step.kind == after
                && after_effect
                    .as_ref()
                    .is_none_or(|effect| step.effect_kind.as_ref() == Some(effect))
        })
        .unwrap_or_else(|| panic!("{workflow} missing step {after:?} {after_effect:?}"));
    assert!(
        position(scenario, before) < after_position,
        "{}: {}",
        workflow,
        reason
    );
}

fn assert_no_forbidden_product_term(
    workflow: &str,
    surface: &str,
    value: &str,
    forbidden: &[&str],
) {
    let lower = value.to_ascii_lowercase();
    for term in forbidden {
        assert!(
            !lower.contains(term),
            "{workflow} {surface} contains product-specific term {term}: {value}"
        );
    }
}
