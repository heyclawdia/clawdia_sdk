use std::{
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use clawdia_sdk::{
    core::{
        AllowToolPolicy, CanonicalToolName, ProviderCapabilities, ProviderRequest,
        ProviderResponse, ProviderStreamChunk, ProviderToolCall, ProviderUsage, ToolCallId,
    },
    prelude::*,
    tools::{FunctionTool, ToolArgs, ToolOutput, ToolResult, serde_json::Value},
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
struct ReadFileInput {
    path: String,
}

impl ToolArgs for ReadFileInput {
    const SCHEMA_ID: &'static str = "schema.example.read_file.input";
    const SCHEMA_VERSION: SchemaVersion = SchemaVersion::new(1, 0, 0);

    fn schema() -> Value {
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

#[derive(Clone, Debug, Serialize)]
struct ReadFileOutput {
    text: String,
}

impl ToolOutput for ReadFileOutput {
    fn redacted_summary(&self) -> String {
        format!("read {} bytes", self.text.len())
    }
}

fn read_file_executor(input: ReadFileInput) -> ToolResult<ReadFileOutput> {
    Ok(ReadFileOutput {
        text: format!("fake content for {}", input.path),
    })
}

#[derive(Clone)]
struct ToolLoopProvider {
    responses: Arc<Mutex<Vec<ProviderResponse>>>,
}

impl ToolLoopProvider {
    fn new(args_ref: clawdia_sdk::core::domain::ContentRef) -> Self {
        let tool_call = ProviderToolCall::new(
            ToolCallId::new("tool.call.example.workspace_read"),
            CanonicalToolName::new("workspace_read"),
            "read README.md",
        )
        .with_args_ref(args_ref);
        Self {
            responses: Arc::new(Mutex::new(vec![
                ProviderResponse::text("typed tool builder complete").with_usage(ProviderUsage {
                    input_tokens: Some(6),
                    output_tokens: Some(4),
                    total_tokens: Some(10),
                }),
                ProviderResponse::tool_use([tool_call]).with_usage(ProviderUsage {
                    input_tokens: Some(5),
                    output_tokens: Some(2),
                    total_tokens: Some(7),
                }),
            ])),
        }
    }
}

impl ProviderAdapter for ToolLoopProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::text_only("provider.example.tool_builder")
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
    let store_root = temp_root("typed-tool-builder");
    let stores = AgentAppStores::file(&store_root);
    let args_ref = stores
        .provider_arguments
        .store_provider_arguments(
            "provider.example.tool_builder",
            "tool.call.example.workspace_read",
            &CanonicalToolName::new("workspace_read"),
            r#"{"path":"README.md"}"#,
        )?
        .expect("provider arguments ref");
    let read_file = FunctionTool::builder("workspace_read")
        .description("Read a file from the workspace")
        .input_schema(ReadFileInput::schema())
        .executor(read_file_executor)
        .build()?;
    let agent = Agent::builder()
        .id(AgentId::new("agent.example.typed_tool_builder"))
        .name("typed tool builder")
        .build()?;
    let app = AgentApp::builder(agent)
        .provider("provider.fake", ToolLoopProvider::new(args_ref))?
        .stores(stores.clone())
        .policy(AllowRunPolicy)
        .tool_policy(AllowToolPolicy)
        .typed_tool(read_file)?
        .build()?;

    let run_id = RunId::new("run.example.typed_tool_builder");
    let result = app.run_text(run_id.clone(), "read README.md")?;
    let evidence = app.run_evidence(&run_id)?;
    let report = app.run_report_from_evidence(&evidence, None)?;
    println!(
        "output={}; records={}; tool_calls={}; provider_calls={}",
        result.output,
        evidence.journal_records.len(),
        report.usage.tool_call_count,
        report.usage.provider_call_count
    );

    drop(std::fs::remove_dir_all(store_root));
    Ok(())
}

fn temp_root(slug: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("clawdia-sdk-example-{slug}-{nanos}"))
}
