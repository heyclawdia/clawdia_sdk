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
            clawdia_sdk::testing::FakeProvider::with_responses(["evidence ready"]),
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

    let live_frames = app
        .event_frames_for_run(run_id.clone(), None)
        .expect("live event frames");
    assert!(!live_frames.is_empty(), "run should publish live frames");

    assert!(
        app.archived_event_frames(None)
            .expect("archive reader is configured")
            .is_empty(),
        "live event frames must not imply archived event frames"
    );

    let archive = clawdia_sdk::stores::FileEventArchive::new(&root);
    for frame in live_frames.clone() {
        archive.append_frame(frame).expect("archive append");
    }
    let archived_frames = app
        .archived_event_frames(None)
        .expect("archived frames are read through archive reader");
    assert_eq!(archived_frames.len(), live_frames.len());

    let records = app
        .journal_records_for_run(&run_id)
        .expect("journal records");
    assert!(!records.is_empty(), "run should append journal records");

    let report = app
        .run_report_from_stores(&run_id, None)
        .expect("report from journal records");
    assert_eq!(report.usage.record_count, records.len());

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
