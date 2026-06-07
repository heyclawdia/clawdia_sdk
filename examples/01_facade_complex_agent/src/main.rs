use std::{
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use clawdia_sdk::{
    core::{
        AllowToolPolicy, ApprovalDecision, ApprovalDispatchResponse, CanonicalToolName,
        ProviderCapabilities, ProviderRequest, ProviderResponse, ProviderStreamChunk,
        ProviderToolCall, ProviderUsage, ToolCallId,
    },
    prelude::*,
    testing::ScriptedApprovalDispatcher,
    tools::{ToolArgs, ToolIdentity, ToolOutput, TypedTool, serde_json::Value},
};
use serde::{Deserialize, Serialize};

struct AllowRunPolicy;

impl RuntimePolicyPort for AllowRunPolicy {
    fn evaluate_run_start(
        &self,
        _request: &RunRequest,
        _package: &RuntimePackage,
    ) -> Result<PolicyOutcome, AgentError> {
        Ok(PolicyOutcome {
            stage: PolicyStage::Input,
            decision: PolicyDecision::allow("policy.example.allow"),
            subject: None,
            source: None,
            destination: None,
            policy_refs: Vec::new(),
            privacy: PrivacyClass::ContentRefsOnly,
            retention: RetentionClass::RunScoped,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct LookupArgs {
    query: String,
}

impl ToolArgs for LookupArgs {
    const SCHEMA_ID: &'static str = "schema.example.lookup.args";
    const SCHEMA_VERSION: SchemaVersion = SchemaVersion::new(1, 0, 0);

    fn schema() -> Value {
        clawdia_sdk::tools::serde_json::json!({
            "type": "object",
            "required": ["query"],
            "properties": {
                "query": { "type": "string" }
            },
            "additionalProperties": false
        })
    }
}

#[derive(Clone, Debug, Serialize)]
struct LookupOutput {
    answer: String,
}

impl ToolOutput for LookupOutput {
    fn redacted_summary(&self) -> String {
        format!("lookup returned {}", self.answer)
    }
}

#[derive(Clone)]
struct ToolLoopProvider {
    responses: Arc<Mutex<Vec<ProviderResponse>>>,
}

impl ToolLoopProvider {
    fn new(args_ref: clawdia_sdk::core::domain::ContentRef) -> Self {
        let tool_call = ProviderToolCall::new(
            ToolCallId::new("tool.call.example.lookup"),
            CanonicalToolName::new("lookup_docs"),
            "lookup docs/start-here.md",
        )
        .with_args_ref(args_ref);
        Self {
            responses: Arc::new(Mutex::new(vec![
                ProviderResponse::text("facade example completed").with_usage(ProviderUsage {
                    input_tokens: Some(4),
                    output_tokens: Some(3),
                    total_tokens: Some(7),
                }),
                ProviderResponse::tool_use([tool_call]),
            ])),
        }
    }
}

impl ProviderAdapter for ToolLoopProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::text_only("provider.example.tool_loop")
    }

    fn complete(&self, _request: &ProviderRequest) -> Result<ProviderResponse, AgentError> {
        self.responses
            .lock()
            .expect("scripted provider lock")
            .pop()
            .ok_or_else(|| AgentError::contract_violation("scripted provider exhausted"))
    }

    fn stream(&self, _request: &ProviderRequest) -> Result<Vec<ProviderStreamChunk>, AgentError> {
        Err(AgentError::host_configuration_needed(
            "example provider is sync-only",
        ))
    }
}

fn main() -> Result<(), AgentError> {
    let store_root = temp_root();
    let stores = AgentAppStores::file(&store_root);
    let args_ref = stores
        .provider_arguments
        .store_provider_arguments(
            "provider.example.tool_loop",
            "tool.call.example.lookup",
            &CanonicalToolName::new("lookup_docs"),
            r#"{"query":"docs/start-here.md"}"#,
        )?
        .expect("provider arguments are stored by ref");
    let tool = TypedTool::builder(ToolIdentity::new("lookup_docs", "v1")?)
        .read_only()
        .sync_handler(|args: LookupArgs, _context| {
            Ok(LookupOutput {
                answer: format!("result for {}", args.query),
            })
        })
        .build()?
        .require_approval();
    let agent = Agent::builder()
        .id(AgentId::new("agent.example.facade"))
        .name("facade example")
        .build()?;
    let event_bus = clawdia_sdk::core::InMemoryAgentEventBus::default();
    let app = AgentApp::builder(agent)
        .provider("provider.fake", ToolLoopProvider::new(args_ref))?
        .stores(stores.clone())
        .event_bus(event_bus)
        .policy(AllowRunPolicy)
        .tool_policy(AllowToolPolicy)
        .approval_dispatcher(ScriptedApprovalDispatcher::new(
            ApprovalDispatchResponse::decision(ApprovalDecision::approved("actor.example.user")),
        ))
        .typed_tool(tool)?
        .build()?;

    let run_id = RunId::new("run.example.facade");
    let result = app.run_text(run_id.clone(), "complete the task")?;
    let records = stores.journal_reader.records_for_run(&run_id)?;
    let report = app.run_report(&run_id, records.iter(), None)?;
    let event_count = app.subscribe_run(run_id, None)?.collect::<Vec<_>>().len();

    println!(
        "{}; records={}; events={}; usage_total_tokens={}",
        result.output,
        records.len(),
        event_count,
        report.usage.provider_total_tokens
    );
    drop(std::fs::remove_dir_all(store_root));
    Ok(())
}

fn temp_root() -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "clawdia-sdk-example-facade-{}-{nanos}",
        std::process::id()
    ))
}
