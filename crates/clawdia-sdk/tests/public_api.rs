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
