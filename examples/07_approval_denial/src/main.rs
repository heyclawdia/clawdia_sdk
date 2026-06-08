use std::{
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use clawdia_sdk::{
    core::{
        AllowToolPolicy, ApprovalDecision, ApprovalDispatchResponse, ApprovalLifecycleStatus,
        ApprovalRecord, CanonicalToolName, JournalRecordPayload, ProviderCapabilities,
        ProviderRequest, ProviderResponse, ProviderStreamChunk, ProviderToolCall, ProviderUsage,
        ToolCallId,
    },
    prelude::*,
    testing::ScriptedApprovalDispatcher,
    tools::{ToolArgs, ToolIdentity, ToolOutput, ToolResult, TypedTool, serde_json::Value},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
struct WriteArgs {
    path: String,
}

impl ToolArgs for WriteArgs {
    const SCHEMA_ID: &'static str = "schema.example.write.args";
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
struct WriteOutput {
    summary: String,
}

impl ToolOutput for WriteOutput {
    fn redacted_summary(&self) -> String {
        self.summary.clone()
    }
}

#[derive(Clone)]
struct DeniedToolProvider {
    responses: Arc<Mutex<Vec<ProviderResponse>>>,
}

impl DeniedToolProvider {
    fn new(args_ref: clawdia_sdk::core::domain::ContentRef) -> Self {
        let tool_call = ProviderToolCall::new(
            ToolCallId::new("tool.call.example.denied_write"),
            CanonicalToolName::new("write_note"),
            "write docs/start-here.md",
        )
        .with_args_ref(args_ref);
        Self {
            responses: Arc::new(Mutex::new(vec![
                ProviderResponse::text("approval denial handled").with_usage(ProviderUsage {
                    input_tokens: Some(8),
                    output_tokens: Some(4),
                    total_tokens: Some(12),
                }),
                ProviderResponse::tool_use([tool_call]),
            ])),
        }
    }
}

impl ProviderAdapter for DeniedToolProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::text_only("provider.example.denied_tool")
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

fn main() -> Result<(), AgentError> {
    let store_root = temp_root("approval-denial");
    let stores = AgentAppStores::file(&store_root);
    let args_ref = stores
        .provider_arguments
        .store_provider_arguments(
            "provider.example.denied_tool",
            "tool.call.example.denied_write",
            &CanonicalToolName::new("write_note"),
            r#"{"path":"docs/start-here.md"}"#,
        )?
        .expect("provider arguments are stored by ref");
    let tool = TypedTool::builder(ToolIdentity::new("write_note", "v1")?)
        .write_effect()
        .sync_handler(|_args: WriteArgs, _context| -> ToolResult<WriteOutput> {
            panic!("denied approval must not execute the tool")
        })
        .build()?
        .require_approval();
    let agent = Agent::builder()
        .id(AgentId::new("agent.example.approval_denial"))
        .name("approval denial")
        .build()?;
    let app = AgentApp::builder(agent)
        .provider("provider.fake", DeniedToolProvider::new(args_ref))?
        .stores(stores)
        .policy(AllowRunPolicy)
        .tool_policy(AllowToolPolicy)
        .approval_dispatcher(ScriptedApprovalDispatcher::new(
            ApprovalDispatchResponse::decision(ApprovalDecision::denied("approval.example.denied")),
        ))
        .typed_tool(tool)?
        .build()?;

    let run_id = RunId::new("run.example.approval_denial");
    let run_result = app.run_text(run_id.clone(), "attempt the gated write");
    let events = app.event_frames_for_run(run_id.clone(), None)?;
    let records = app.journal_records_for_run(&run_id)?;
    let approval_denials = records
        .iter()
        .filter(|record| {
            matches!(
                &record.payload,
                JournalRecordPayload::Approval(ApprovalRecord::Denied { .. })
                    | JournalRecordPayload::Approval(ApprovalRecord::DispatchResult {
                        lifecycle_status: ApprovalLifecycleStatus::Denied,
                        ..
                    })
            )
        })
        .count();
    let tool_records = records
        .iter()
        .filter(|record| matches!(&record.payload, JournalRecordPayload::Tool(_)))
        .count();
    let report = app.run_report_from_stores(&run_id, None)?;
    let outcome = match &run_result {
        Ok(result) => format!("completed:{:?}", result.status),
        Err(error) => format!("closed:{:?}", error.kind()),
    };
    let message = match &run_result {
        Ok(result) => result.output.clone(),
        Err(error) => error.context().message.clone(),
    };

    println!(
        "outcome={}; message={}; approval_denials={}; tool_records={}; events={}; report_records={}",
        outcome,
        message,
        approval_denials,
        tool_records,
        events.len(),
        report.usage.record_count
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
