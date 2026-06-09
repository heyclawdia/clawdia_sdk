use std::any::type_name;

use clawdia_sdk::prelude::*;

#[test]
fn facade_prelude_lowers_to_core_request_types() {
    let agent = Agent::builder()
        .id(AgentId::new("agent.facade.public_api"))
        .name("facade public api")
        .build()
        .expect("agent builds");
    let source = SourceRef::with_kind(SourceKind::Host, "source.facade.public_api");

    let request = RunRequest::text(
        RunId::new("run.facade.public_api"),
        agent.id().clone(),
        source,
        "hello",
    );
    let package = RuntimePackage::builder(clawdia_sdk::core::RuntimePackageId::new(
        "package.facade.public_api",
    ))
    .agent(agent.snapshot())
    .build()
    .expect("package builds");

    assert_eq!(request.agent_id, agent.id().clone());
    assert_eq!(package.agent.agent_id, agent.id().clone());
    assert!(
        type_name::<AgentRuntime>().contains("agent_sdk_core"),
        "facade prelude must re-export core runtime, not define a second runtime"
    );
}

#[test]
fn facade_core_namespace_exposes_advanced_core_surface() {
    let agent = clawdia_sdk::core::Agent::builder()
        .id(clawdia_sdk::core::AgentId::new("agent.facade.core"))
        .name("facade core")
        .build()
        .expect("agent builds");

    assert_eq!(agent.name(), "facade core");
    assert!(type_name::<dyn clawdia_sdk::core::RunJournal>().contains("agent_sdk_core"));
}

#[cfg(feature = "providers")]
#[test]
fn providers_feature_exports_provider_adapters() {
    assert!(
        type_name::<clawdia_sdk::providers::OpenAiResponsesAdapter>()
            .contains("agent_sdk_provider")
    );
    assert!(
        type_name::<clawdia_sdk::providers::OpenAiCompatibleResponsesAdapter>()
            .contains("agent_sdk_provider")
    );
}

#[cfg(feature = "workspace-tools")]
#[test]
fn workspace_tools_feature_exports_toolkit_helpers() {
    assert!(type_name::<clawdia_sdk::tools::Tool>().contains("agent_sdk_toolkit"));
    assert!(type_name::<clawdia_sdk::tools::ToolPackBuilder>().contains("agent_sdk_toolkit"));
    assert!(type_name::<clawdia_sdk::tools::WorkspaceReadExecutor>().contains("agent_sdk_toolkit"));
}

#[cfg(feature = "evals")]
#[test]
fn evals_feature_exports_eval_helpers() {
    assert!(type_name::<clawdia_sdk::eval::TraceMetrics>().contains("agent_sdk_eval"));
    assert!(type_name::<clawdia_sdk::eval::EvaluationReport>().contains("agent_sdk_eval"));
    assert!(type_name::<clawdia_sdk::eval::UsageReport>().contains("agent_sdk_eval"));
    assert!(type_name::<clawdia_sdk::eval::RunReport>().contains("agent_sdk_eval"));
}

#[cfg(feature = "file-store")]
#[test]
fn file_store_feature_exports_file_store_adapters() {
    assert!(type_name::<clawdia_sdk::stores::FileStoreBundle>().contains("agent_sdk_store_file"));
    assert!(type_name::<clawdia_sdk::stores::FileRunJournal>().contains("agent_sdk_store_file"));
    let root = std::env::temp_dir().join(format!(
        "clawdia-sdk-public-api-file-store-{}",
        std::process::id()
    ));
    let stores = AgentAppStores::file(&root);
    assert!(
        stores
            .journal_reader
            .records_for_run(&RunId::new("run.facade.public_api.file_store"))
            .expect("file store journal reader is available")
            .is_empty()
    );
    drop(std::fs::remove_dir_all(root));
}

#[cfg(feature = "supabase-store")]
#[test]
fn supabase_store_feature_exports_supabase_store_adapters() {
    assert!(
        type_name::<clawdia_sdk::stores::SupabaseStoreConfig>()
            .contains("agent_sdk_store_supabase")
    );
    assert!(
        type_name::<clawdia_sdk::stores::SupabaseRunJournal>().contains("agent_sdk_store_supabase")
    );
}

#[cfg(feature = "macros")]
#[test]
fn macros_feature_exports_tool_macros_through_tools_namespace() {
    #[derive(
        serde::Serialize,
        serde::Deserialize,
        clawdia_sdk::tools::ToolArgs,
        clawdia_sdk::tools::ToolOutput,
    )]
    struct FacadeMacroArgs {
        value: String,
    }

    fn assert_tool_args<T: clawdia_sdk::tools::ToolArgs>() {}
    fn assert_tool_output<T: clawdia_sdk::tools::ToolOutput>() {}

    assert_tool_args::<FacadeMacroArgs>();
    assert_tool_output::<FacadeMacroArgs>();
}

#[cfg(feature = "test-support")]
#[test]
fn test_support_feature_exports_core_testing_helpers() {
    let journal = clawdia_sdk::testing::FakeJournalStore::default();

    assert!(journal.records().is_empty());
}

#[cfg(feature = "test-support")]
#[test]
fn agent_app_runs_text_through_canonical_runtime() {
    let agent = Agent::builder()
        .id(AgentId::new("agent.facade.app"))
        .name("facade app")
        .build()
        .expect("agent builds");
    let event_bus = clawdia_sdk::core::InMemoryAgentEventBus::default();
    let app = AgentApp::builder(agent)
        .provider(
            "provider.fake",
            clawdia_sdk::testing::FakeProvider::with_responses(["hello from AgentApp"]),
        )
        .expect("provider registers")
        .journal(clawdia_sdk::testing::FakeJournalStore::default())
        .event_bus(event_bus.clone())
        .content(clawdia_sdk::testing::FakeContentResolver::default())
        .policy(AllowFacadePolicy)
        .build()
        .expect("app builds");

    let result = app
        .run_text(RunId::new("run.facade.app"), "say hello")
        .expect("run succeeds");

    assert_eq!(result.output, "hello from AgentApp");
    assert_eq!(result.status, RunStatus::Completed);
    let frames = app
        .subscribe_run(RunId::new("run.facade.app"), None)
        .expect("subscribe run")
        .collect::<Vec<_>>();
    assert!(!frames.is_empty());
    assert!(
        event_bus
            .subscribe_run(RunId::new("run.facade.app"), None)
            .expect("event bus has same events")
            .next()
            .is_some()
    );
}

#[cfg(feature = "test-support")]
#[test]
fn agent_app_read_helpers_require_configured_stores() {
    let agent = Agent::builder()
        .id(AgentId::new("agent.facade.no_stores"))
        .name("facade no stores")
        .build()
        .expect("agent builds");
    let app = AgentApp::builder(agent)
        .provider(
            "provider.fake",
            clawdia_sdk::testing::FakeProvider::with_responses(["unused"]),
        )
        .expect("provider registers")
        .journal(clawdia_sdk::testing::FakeJournalStore::default())
        .content(clawdia_sdk::testing::FakeContentResolver::default())
        .policy(AllowFacadePolicy)
        .build()
        .expect("app builds");

    let error = app
        .journal_records_for_run(&RunId::new("run.facade.no_stores"))
        .expect_err("missing stores must be diagnostic");

    assert_eq!(
        error.kind(),
        clawdia_sdk::core::AgentErrorKind::HostConfigurationNeeded
    );
    assert!(
        error
            .context()
            .message
            .contains("AgentApp journal_records_for_run requires AgentAppStores")
    );

    let error = app
        .run_evidence(&RunId::new("run.facade.no_stores"))
        .expect_err("run evidence must require durable stores");

    assert_eq!(
        error.kind(),
        clawdia_sdk::core::AgentErrorKind::HostConfigurationNeeded
    );
    assert!(
        error
            .context()
            .message
            .contains("AgentApp run_evidence requires AgentAppStores")
    );
}

#[cfg(all(feature = "file-store", feature = "test-support"))]
#[test]
fn agent_app_run_evidence_tolerates_missing_optional_archive_and_checkpoint_ports() {
    let root = temp_file_store_root("run-evidence-optional-ports");
    let mut stores = AgentAppStores::file(&root);
    stores.event_archive = None;
    stores.checkpoint = None;
    let agent = Agent::builder()
        .id(AgentId::new("agent.facade.optional_evidence"))
        .name("facade optional evidence")
        .build()
        .expect("agent builds");
    let app = AgentApp::builder(agent)
        .provider(
            "provider.fake",
            clawdia_sdk::testing::FakeProvider::with_responses(["optional evidence ready"]),
        )
        .expect("provider registers")
        .stores(stores)
        .policy(AllowFacadePolicy)
        .build()
        .expect("app builds");

    let run_id = RunId::new("run.facade.optional_evidence");
    let result = app
        .run_text(run_id.clone(), "collect optional evidence")
        .expect("run succeeds");
    assert_eq!(result.status, RunStatus::Completed);

    let evidence = app.run_evidence(&run_id).expect("run evidence");

    assert!(!evidence.live_event_frames.is_empty());
    assert!(!evidence.journal_records.is_empty());
    assert!(
        evidence.archived_event_frames.is_empty(),
        "missing optional archive reader should not fail run evidence"
    );
    assert!(
        evidence.latest_checkpoint.is_none(),
        "missing optional checkpoint store should not fail run evidence"
    );

    drop(std::fs::remove_dir_all(root));
}

#[cfg(all(feature = "evals", feature = "file-store", feature = "test-support"))]
#[test]
fn agent_app_read_helpers_keep_observation_archive_journal_report_and_checkpoint_separate() {
    let root = temp_file_store_root("read-helpers");
    let stores = AgentAppStores::file(&root);
    let agent = Agent::builder()
        .id(AgentId::new("agent.facade.evidence"))
        .name("facade evidence")
        .build()
        .expect("agent builds");
    let app = AgentApp::builder(agent)
        .provider(
            "provider.fake",
            clawdia_sdk::testing::FakeProvider::with_responses([
                "evidence ready",
                "other evidence ready",
            ]),
        )
        .expect("provider registers")
        .stores(stores.clone())
        .policy(AllowFacadePolicy)
        .build()
        .expect("app builds");

    let run_id = RunId::new("run.facade.evidence");
    let result = app
        .run_text(run_id.clone(), "collect evidence")
        .expect("run succeeds");
    assert_eq!(result.status, RunStatus::Completed);
    let other_run_id = RunId::new("run.facade.other_evidence");
    app.run_text(other_run_id.clone(), "collect other evidence")
        .expect("second run succeeds");

    let live_frames = app
        .event_frames_for_run(run_id.clone(), None)
        .expect("live event frames");
    assert!(!live_frames.is_empty(), "run should publish live frames");
    let other_live_frames = app
        .event_frames_for_run(other_run_id.clone(), None)
        .expect("other live event frames");
    assert!(
        !other_live_frames.is_empty(),
        "second run should publish live frames"
    );

    assert!(
        app.archived_event_frames(None)
            .expect("archive reader is configured")
            .is_empty(),
        "live event frames must not imply archived event frames"
    );
    let initial_evidence = app.run_evidence(&run_id).expect("initial run evidence");
    assert_eq!(initial_evidence.run_id, run_id);
    assert_eq!(initial_evidence.live_event_frames, live_frames);
    assert!(
        initial_evidence.archived_event_frames.is_empty(),
        "run evidence must not treat live frames as archived frames"
    );
    assert!(
        initial_evidence.latest_checkpoint.is_none(),
        "journal records do not create checkpoint accelerator entries"
    );

    let archive = clawdia_sdk::stores::FileEventArchive::new(&root);
    for frame in live_frames
        .clone()
        .into_iter()
        .chain(other_live_frames.clone())
    {
        archive.append_frame(frame).expect("archive append");
    }
    let archived_frames = app
        .archived_event_frames(None)
        .expect("archived frames are read through archive reader");
    assert_eq!(
        archived_frames.len(),
        live_frames.len() + other_live_frames.len()
    );

    let evidence = app.run_evidence(&run_id).expect("run evidence");
    assert_eq!(evidence.archived_event_frames.len(), live_frames.len());
    assert!(
        evidence
            .archived_event_frames
            .iter()
            .all(|frame| frame.event.envelope.run_id == run_id),
        "run evidence must filter the global archive to the requested run"
    );

    let records = &evidence.journal_records;
    assert!(!records.is_empty(), "run should append journal records");

    let report = app
        .run_report_from_stores(&run_id, None)
        .expect("report from journal records");
    assert_eq!(report.usage.record_count, records.len());
    let evidence_report = app
        .run_report_from_evidence(&evidence, None)
        .expect("report from run evidence");
    assert_eq!(evidence_report.usage.record_count, records.len());

    assert!(
        app.latest_checkpoint(&run_id)
            .expect("checkpoint store configured")
            .is_none(),
        "journal records do not create checkpoint accelerator entries"
    );

    let latest_journal_seq = records
        .iter()
        .map(|record| record.journal_seq)
        .max()
        .expect("records have journal seq");
    let checkpoint = clawdia_sdk::core::RunCheckpoint {
        checkpoint_id: "checkpoint.facade.evidence.terminal".to_string(),
        run_id: run_id.clone(),
        checkpoint_seq: 1,
        covers_journal_seq: latest_journal_seq,
        loop_state: "completed".to_string(),
        turn_id: None,
        attempt_id: None,
        runtime_package_fingerprint: "runtime.package.fingerprint.facade.evidence".to_string(),
        pending_side_effects: Vec::new(),
        pending_approvals: Vec::new(),
        content_ref_manifest: Vec::new(),
        state_hash: "state.hash.facade.evidence".to_string(),
        created_at_millis: latest_journal_seq,
        writer_id: "writer.facade.test".to_string(),
    };
    stores
        .checkpoint
        .as_ref()
        .expect("file stores include checkpoint")
        .save(checkpoint.clone(), latest_journal_seq)
        .expect("checkpoint save");

    let latest_checkpoint = app
        .latest_checkpoint(&run_id)
        .expect("latest checkpoint")
        .expect("checkpoint saved");
    assert_eq!(latest_checkpoint.checkpoint_id, checkpoint.checkpoint_id);
    let checkpoint_evidence = app.run_evidence(&run_id).expect("checkpoint evidence");
    assert_eq!(
        checkpoint_evidence
            .latest_checkpoint
            .expect("checkpoint is present in run evidence")
            .checkpoint_id,
        checkpoint.checkpoint_id
    );

    drop(std::fs::remove_dir_all(root));
}

#[cfg(all(
    feature = "evals",
    feature = "file-store",
    feature = "test-support",
    feature = "workspace-tools"
))]
#[test]
fn agent_app_builder_lowers_function_tool_into_canonical_runtime_path() {
    let root = temp_file_store_root("function-tool-builder");
    let stores = AgentAppStores::file(&root);
    let args_ref = stores
        .provider_arguments
        .store_provider_arguments(
            "provider.facade.builder",
            "tool.call.facade.builder",
            &clawdia_sdk::core::CanonicalToolName::new("workspace_read"),
            r#"{"path":"README.md"}"#,
        )
        .expect("provider args store")
        .expect("provider args ref");
    let tool = clawdia_sdk::tools::FunctionTool::builder("workspace_read")
        .description("Read a file from the workspace")
        .input_schema(<BuilderReadInput as clawdia_sdk::tools::ToolArgs>::schema())
        .executor(builder_read_file)
        .build()
        .expect("function tool builds");
    let agent = Agent::builder()
        .id(AgentId::new("agent.facade.builder"))
        .name("facade builder")
        .build()
        .expect("agent builds");
    let event_bus = clawdia_sdk::core::InMemoryAgentEventBus::default();
    let app = AgentApp::builder(agent)
        .provider("provider.fake", BuilderToolLoopProvider::new(args_ref))
        .expect("provider registers")
        .stores(stores.clone())
        .event_bus(event_bus.clone())
        .policy(AllowFacadePolicy)
        .tool_policy(clawdia_sdk::core::AllowToolPolicy)
        .typed_tool(tool)
        .expect("typed tool registers")
        .build()
        .expect("app builds");

    let run_id = RunId::new("run.facade.builder");
    let result = app
        .run_typed::<BuilderSummary>(run_id.clone(), "read README and summarize")
        .expect("typed run succeeds");

    assert_eq!(result.status, RunStatus::Completed);
    assert!(result.structured_output.is_some());
    assert_eq!(result.output, r#"{"status":"done"}"#);
    let evidence = app.run_evidence(&run_id).expect("run evidence");
    assert!(evidence.live_event_frames.len() >= 2);
    assert!(
        event_bus
            .subscribe_run(run_id.clone(), None)
            .expect("event bus")
            .next()
            .is_some()
    );
    assert!(evidence.journal_records.iter().any(|record| matches!(
        record.payload,
        clawdia_sdk::core::JournalRecordPayload::Tool(_)
    )));
    let report = app
        .run_report_from_evidence(
            &evidence,
            Some(&clawdia_sdk::eval::StaticRateTable::new(
                "USD", 1_000_000, 2_000_000, 100,
            )),
        )
        .expect("report from evidence");
    assert_eq!(report.usage.tool_call_count, 1);
    assert!(report.usage.provider_call_count >= 1);
    assert!(report.cost.expect("cost report").total_cost_micros > 0);

    drop(std::fs::remove_dir_all(root));
}

#[cfg(feature = "test-support")]
struct AllowFacadePolicy;

#[cfg(feature = "test-support")]
impl RuntimePolicyPort for AllowFacadePolicy {
    fn evaluate_run_start(
        &self,
        _request: &RunRequest,
        _package: &RuntimePackage,
    ) -> Result<PolicyOutcome, AgentError> {
        Ok(PolicyOutcome {
            stage: PolicyStage::Input,
            decision: PolicyDecision::allow("policy.facade.allow"),
            subject: None,
            source: None,
            destination: None,
            policy_refs: Vec::new(),
            privacy: PrivacyClass::ContentRefsOnly,
            retention: RetentionClass::RunScoped,
        })
    }
}

#[cfg(feature = "file-store")]
fn temp_file_store_root(slug: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "clawdia-sdk-public-api-{slug}-{}",
        std::process::id()
    ))
}

#[cfg(all(
    feature = "evals",
    feature = "file-store",
    feature = "test-support",
    feature = "workspace-tools"
))]
#[derive(Clone, serde::Deserialize, serde::Serialize)]
struct BuilderReadInput {
    path: String,
}

#[cfg(all(
    feature = "evals",
    feature = "file-store",
    feature = "test-support",
    feature = "workspace-tools"
))]
impl clawdia_sdk::tools::ToolArgs for BuilderReadInput {
    const SCHEMA_ID: &'static str = "schema.facade.builder.read_input";
    const SCHEMA_VERSION: SchemaVersion = SchemaVersion::new(1, 0, 0);

    fn schema() -> clawdia_sdk::tools::serde_json::Value {
        clawdia_sdk::tools::serde_json::json!({
            "type": "object",
            "required": ["path"],
            "properties": {
                "path": { "type": "string" }
            },
            "additionalProperties": false
        })
    }
}

#[cfg(all(
    feature = "evals",
    feature = "file-store",
    feature = "test-support",
    feature = "workspace-tools"
))]
#[derive(Clone, serde::Serialize)]
struct BuilderReadOutput {
    text: String,
}

#[cfg(all(
    feature = "evals",
    feature = "file-store",
    feature = "test-support",
    feature = "workspace-tools"
))]
impl clawdia_sdk::tools::ToolOutput for BuilderReadOutput {
    fn redacted_summary(&self) -> String {
        format!("read {} bytes", self.text.len())
    }
}

#[cfg(all(
    feature = "evals",
    feature = "file-store",
    feature = "test-support",
    feature = "workspace-tools"
))]
fn builder_read_file(input: BuilderReadInput) -> clawdia_sdk::tools::ToolResult<BuilderReadOutput> {
    Ok(BuilderReadOutput {
        text: format!("fake content for {}", input.path),
    })
}

#[cfg(all(
    feature = "evals",
    feature = "file-store",
    feature = "test-support",
    feature = "workspace-tools"
))]
#[derive(Clone, serde::Deserialize, serde::Serialize)]
struct BuilderSummary {
    status: String,
}

#[cfg(all(
    feature = "evals",
    feature = "file-store",
    feature = "test-support",
    feature = "workspace-tools"
))]
impl TypedOutputModel for BuilderSummary {
    const SCHEMA_ID: &'static str = "schema.facade.builder.summary";
    const SCHEMA_VERSION: SchemaVersion = SchemaVersion::new(1, 0, 0);

    fn schema_ref() -> OutputSchemaRef {
        OutputContract::inline_json_schema(
            OutputSchemaId::new(Self::SCHEMA_ID),
            Self::SCHEMA_VERSION,
            clawdia_sdk::tools::serde_json::json!({
                "type": "object",
                "required": ["status"],
                "properties": {
                    "status": { "type": "string" }
                },
                "additionalProperties": false
            }),
        )
        .schema
    }
}

#[cfg(all(
    feature = "evals",
    feature = "file-store",
    feature = "test-support",
    feature = "workspace-tools"
))]
#[derive(Clone)]
struct BuilderToolLoopProvider {
    responses: std::sync::Arc<std::sync::Mutex<Vec<clawdia_sdk::core::ProviderResponse>>>,
}

#[cfg(all(
    feature = "evals",
    feature = "file-store",
    feature = "test-support",
    feature = "workspace-tools"
))]
impl BuilderToolLoopProvider {
    fn new(args_ref: clawdia_sdk::core::domain::ContentRef) -> Self {
        let tool_call = clawdia_sdk::core::ProviderToolCall::new(
            clawdia_sdk::core::ToolCallId::new("tool.call.facade.builder"),
            clawdia_sdk::core::CanonicalToolName::new("workspace_read"),
            "read README",
        )
        .with_args_ref(args_ref);
        Self {
            responses: std::sync::Arc::new(std::sync::Mutex::new(vec![
                clawdia_sdk::core::ProviderResponse::text(r#"{"status":"done"}"#).with_usage(
                    clawdia_sdk::core::ProviderUsage {
                        input_tokens: Some(8),
                        output_tokens: Some(4),
                        total_tokens: Some(12),
                    },
                ),
                clawdia_sdk::core::ProviderResponse::tool_use([tool_call]).with_usage(
                    clawdia_sdk::core::ProviderUsage {
                        input_tokens: Some(5),
                        output_tokens: Some(2),
                        total_tokens: Some(7),
                    },
                ),
            ])),
        }
    }
}

#[cfg(all(
    feature = "evals",
    feature = "file-store",
    feature = "test-support",
    feature = "workspace-tools"
))]
impl ProviderAdapter for BuilderToolLoopProvider {
    fn capabilities(&self) -> clawdia_sdk::core::ProviderCapabilities {
        clawdia_sdk::core::ProviderCapabilities::text_only("provider.facade.builder")
    }

    fn complete(
        &self,
        _request: &clawdia_sdk::core::ProviderRequest,
    ) -> Result<clawdia_sdk::core::ProviderResponse, AgentError> {
        self.responses
            .lock()
            .expect("scripted provider lock")
            .pop()
            .ok_or_else(|| AgentError::contract_violation("scripted provider exhausted"))
    }

    fn stream(
        &self,
        _request: &clawdia_sdk::core::ProviderRequest,
    ) -> Result<Vec<clawdia_sdk::core::ProviderStreamChunk>, AgentError> {
        Err(AgentError::host_configuration_needed(
            "facade builder test provider is sync-only",
        ))
    }
}
