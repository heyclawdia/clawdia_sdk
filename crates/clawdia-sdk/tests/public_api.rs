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
}

#[cfg(feature = "test-support")]
#[test]
fn test_support_feature_exports_core_testing_helpers() {
    let journal = clawdia_sdk::testing::FakeJournalStore::default();

    assert!(journal.records().is_empty());
}
