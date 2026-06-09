use std::sync::Arc;

use agent_sdk_core::{
    AgentId, AllowToolPolicy, JournalRecordPayload, PolicyKind, PolicyRef, ProviderRouteSnapshot,
    RunId, RuntimePackage, RuntimePackageId, SourceKind, SourceRef, ToolCallId, ToolCallRequest,
    ToolExecutionContext, ToolExecutionCoordinator, ToolExecutorRegistry, ToolRegistrySnapshot,
    ToolRouter,
    domain::ContentRef as ContentRefId,
    testing::FakeJournalStore,
    tool_records::{CanonicalToolName, ToolCallRecordStatus},
};
use agent_sdk_toolkit::{
    FunctionTool, InMemoryJsonArgumentStore, InMemoryToolkitContentStore, ToolArgs, ToolIdentity,
    ToolOutput, ToolResult, TypedTool,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Debug, Deserialize, Serialize)]
struct LookupArgs {
    query: String,
}

impl ToolArgs for LookupArgs {
    const SCHEMA_ID: &'static str = "schema.lookup.args";
    const SCHEMA_VERSION: agent_sdk_core::SchemaVersion =
        agent_sdk_core::SchemaVersion::new(1, 0, 0);

    fn schema() -> Value {
        json!({
            "type": "object",
            "required": ["query"],
            "properties": {
                "query": { "type": "string" }
            },
            "additionalProperties": false
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct LookupOutput {
    answer: String,
}

impl ToolOutput for LookupOutput {
    fn redacted_summary(&self) -> String {
        format!("lookup returned {}", self.answer)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ReadFileInput {
    path: String,
}

impl ToolArgs for ReadFileInput {
    const SCHEMA_ID: &'static str = "schema.read_file.input";
    const SCHEMA_VERSION: agent_sdk_core::SchemaVersion =
        agent_sdk_core::SchemaVersion::new(1, 0, 0);

    fn schema() -> Value {
        json!({
            "type": "object",
            "required": ["path"],
            "properties": {
                "path": { "type": "string" }
            },
            "additionalProperties": false
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct ReadFileOutput {
    text: String,
}

impl ToolOutput for ReadFileOutput {
    fn redacted_summary(&self) -> String {
        format!("read {} bytes", self.text.len())
    }
}

#[test]
fn typed_tool_schema_snapshot_is_deterministic_and_package_visible() {
    let tool = lookup_tool();

    assert_eq!(
        tool.schema_snapshot().schema_ref.content_hash.as_deref(),
        Some(tool.schema_snapshot().content_hash.as_str())
    );
    assert_eq!(
        tool.schema_snapshot().redacted_schema["properties"]["query"]["type"],
        "string"
    );

    let bundle = tool
        .pack_bundle(SourceRef::with_kind(
            SourceKind::Sdk,
            "source.typed_tool.test",
        ))
        .expect("bundle builds");
    let package = bundle
        .install_into(base_package_builder())
        .build()
        .expect("package builds");

    assert_eq!(package.provider_tool_specs().unwrap().len(), 1);
    let sidecar_payload = bundle
        .sidecar
        .redacted_payload
        .as_ref()
        .expect("tool pack sidecar carries redacted payload");
    assert_eq!(
        sidecar_payload["tools"][0]["redacted_schema"]["properties"]["query"]["type"],
        "string"
    );
    assert_eq!(
        sidecar_payload["tools"][0]["redacted_schema"]["additionalProperties"],
        false
    );
    assert_eq!(bundle.routes[0].canonical_tool_name.as_str(), "lookup_docs");
    assert!(!bundle.routes[0].requires_approval);
}

#[test]
fn typed_tool_require_approval_sets_route_execution_gate() {
    let tool = lookup_tool().require_approval();
    let bundle = tool
        .pack_bundle(SourceRef::with_kind(
            SourceKind::Sdk,
            "source.typed_tool.test",
        ))
        .expect("bundle builds");

    assert!(bundle.routes[0].requires_approval);
    assert!(
        bundle.routes[0]
            .policy_refs
            .iter()
            .any(|policy| policy.kind == PolicyKind::Approval)
    );
}

#[test]
fn typed_tool_executor_runs_through_core_coordinator_and_stores_output() {
    let tool = lookup_tool();
    let args_store = Arc::new(InMemoryJsonArgumentStore::default());
    let output_store = Arc::new(InMemoryToolkitContentStore::default());
    let args_ref = ContentRefId::new("content.args.lookup");
    args_store
        .insert(
            args_ref.clone(),
            &LookupArgs {
                query: "README.md".to_string(),
            },
        )
        .expect("args insert");

    let outcome = execute_tool(tool, args_store, output_store.clone(), args_ref);

    assert_eq!(outcome.record.status, ToolCallRecordStatus::Completed);
    assert_eq!(
        outcome.record.redacted_result_summary.as_deref(),
        Some("lookup returned result for README.md")
    );
    let result_ref = outcome.record.result_content_refs[0].clone();
    let stored: LookupOutput = output_store.get(&result_ref).expect("stored output");
    assert_eq!(
        stored,
        LookupOutput {
            answer: "result for README.md".to_string()
        }
    );
}

#[test]
fn function_tool_builder_lowers_to_typed_tool_package_and_core_execution() {
    let tool = FunctionTool::builder("workspace_read")
        .description("Read a file from the workspace")
        .input_schema(ReadFileInput::schema())
        .executor(read_file_executor)
        .build()
        .expect("function tool builds");

    assert_eq!(
        tool.schema_snapshot().schema_ref.content_hash.as_deref(),
        Some(tool.schema_snapshot().content_hash.as_str())
    );
    assert_eq!(
        tool.schema_snapshot().redacted_schema["properties"]["path"]["type"],
        "string"
    );

    let bundle = tool
        .pack_bundle(SourceRef::with_kind(
            SourceKind::Sdk,
            "source.function_tool.test",
        ))
        .expect("bundle builds");
    assert_eq!(
        bundle.sidecar.redacted_payload.as_ref().expect("payload")["tools"][0]["description"],
        "Read a file from the workspace"
    );
    assert_eq!(
        bundle.routes[0].description.as_deref(),
        Some("Read a file from the workspace")
    );

    let package = bundle
        .install_into(base_package_builder())
        .build()
        .expect("package builds");
    assert_eq!(
        package.provider_tool_specs().expect("provider tools").len(),
        1
    );

    let args_store = Arc::new(InMemoryJsonArgumentStore::default());
    let output_store = Arc::new(InMemoryToolkitContentStore::default());
    let args_ref = ContentRefId::new("content.args.workspace_read");
    args_store
        .insert(
            args_ref.clone(),
            &ReadFileInput {
                path: "README.md".to_string(),
            },
        )
        .expect("args insert");

    let outcome = execute_tool_with_request(
        tool,
        args_store,
        output_store.clone(),
        args_ref,
        "workspace_read",
        "tool.call.workspace_read",
    );
    assert_eq!(outcome.record.status, ToolCallRecordStatus::Completed);
    assert_eq!(
        outcome.record.redacted_result_summary.as_deref(),
        Some("read 14 bytes")
    );
    let stored: ReadFileOutput = output_store
        .get(&outcome.record.result_content_refs[0])
        .expect("stored output");
    assert_eq!(
        stored,
        ReadFileOutput {
            text: "fake:README.md".to_string()
        }
    );
}

#[test]
fn typed_tool_invalid_arguments_return_terminal_failed_record() {
    let tool = lookup_tool();
    let args_store = Arc::new(InMemoryJsonArgumentStore::default());
    let output_store = Arc::new(InMemoryToolkitContentStore::default());
    let args_ref = ContentRefId::new("content.args.invalid");
    args_store
        .insert(args_ref.clone(), &json!({"query": 5}))
        .expect("args insert");

    let outcome = execute_tool(tool, args_store, output_store, args_ref);

    assert_eq!(outcome.record.status, ToolCallRecordStatus::Failed);
    assert_eq!(
        outcome.record.redacted_result_summary.as_deref(),
        Some("typed tool arguments failed schema decoding")
    );
}

fn lookup_tool() -> TypedTool<LookupArgs, LookupOutput> {
    TypedTool::builder(ToolIdentity::new("lookup_docs", "v1").expect("identity"))
        .read_only()
        .policy_ref(PolicyRef::with_kind(
            PolicyKind::Approval,
            "policy.approval.lookup_docs",
        ))
        .sync_handler(|args: LookupArgs, _context| {
            Ok(LookupOutput {
                answer: format!("result for {}", args.query),
            })
        })
        .build()
        .expect("typed tool builds")
}

fn read_file_executor(input: ReadFileInput) -> ToolResult<ReadFileOutput> {
    Ok(ReadFileOutput {
        text: format!("fake:{}", input.path),
    })
}

fn execute_tool(
    tool: TypedTool<LookupArgs, LookupOutput>,
    args_store: Arc<InMemoryJsonArgumentStore>,
    output_store: Arc<InMemoryToolkitContentStore>,
    args_ref: ContentRefId,
) -> agent_sdk_core::tool_execution::ToolExecutionOutcome {
    execute_tool_with_request(
        tool,
        args_store,
        output_store,
        args_ref,
        "lookup_docs",
        "tool.call.lookup",
    )
}

fn execute_tool_with_request<A, R>(
    tool: TypedTool<A, R>,
    args_store: Arc<InMemoryJsonArgumentStore>,
    output_store: Arc<InMemoryToolkitContentStore>,
    args_ref: ContentRefId,
    canonical_tool_name: &str,
    tool_call_id: &str,
) -> agent_sdk_core::tool_execution::ToolExecutionOutcome
where
    A: ToolArgs,
    R: ToolOutput,
{
    let bundle = tool
        .pack_bundle(SourceRef::with_kind(
            SourceKind::Sdk,
            "source.typed_tool.test",
        ))
        .expect("bundle builds");
    let package = bundle
        .install_into(base_package_builder())
        .build()
        .expect("package builds");
    let snapshot = ToolRegistrySnapshot::from_runtime_package(&package, bundle.routes.clone())
        .expect("snapshot builds");
    let mut executors = ToolExecutorRegistry::new();
    executors
        .register(tool.executor(args_store, output_store))
        .expect("executor registers");
    let coordinator = ToolExecutionCoordinator::new(ToolRouter::new(snapshot), executors)
        .with_policy(Arc::new(AllowToolPolicy));
    let journal = FakeJournalStore::default();
    let fingerprint = package.fingerprint().expect("fingerprint");
    let context = ToolExecutionContext::new(
        RunId::new("run.typed_tool"),
        AgentId::new("agent.typed_tool"),
        SourceRef::with_kind(SourceKind::Sdk, "source.typed_tool.executor"),
        fingerprint.as_str(),
    );

    let outcome = coordinator
        .execute(
            &journal,
            ToolCallRequest {
                tool_call_id: ToolCallId::new(tool_call_id),
                canonical_tool_name: CanonicalToolName::new(canonical_tool_name),
                source: SourceRef::with_kind(SourceKind::Sdk, "source.provider.tool_call"),
                requested_args_refs: vec![args_ref],
                redacted_args_summary: "lookup docs".to_string(),
                idempotency_key: None,
                dedupe_key: None,
            },
            context,
        )
        .expect("tool executes");

    let records = journal.records();
    assert_eq!(records.len(), 2);
    assert!(matches!(records[0].payload, JournalRecordPayload::Tool(_)));
    outcome
}

fn base_package_builder() -> agent_sdk_core::RuntimePackageBuilder {
    RuntimePackage::builder(RuntimePackageId::new("package.typed_tool"))
        .agent(agent_sdk_core::AgentSnapshot {
            agent_id: AgentId::new("agent.typed_tool"),
            name: "typed tool".to_string(),
            default_behavior_refs: Vec::new(),
        })
        .provider_route(ProviderRouteSnapshot::new("provider.fake", "model.fake"))
        .policy(PolicyRef::with_kind(
            PolicyKind::RuntimePackage,
            "policy.package.typed_tool",
        ))
}
