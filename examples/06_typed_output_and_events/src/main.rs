use std::time::{SystemTime, UNIX_EPOCH};

use clawdia_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Debug, Deserialize, Serialize)]
struct TodoExtraction {
    title: String,
    priority: String,
}

impl TypedOutputModel for TodoExtraction {
    const SCHEMA_ID: &'static str = "schema.example.todo_extraction";
    const SCHEMA_VERSION: SchemaVersion = SchemaVersion::new(1, 0, 0);

    fn schema_ref() -> OutputSchemaRef {
        OutputContract::inline_json_schema(
            OutputSchemaId::new(Self::SCHEMA_ID),
            Self::SCHEMA_VERSION,
            todo_schema(),
        )
        .schema
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
    let store_root = temp_root("typed-output-events");
    let stores = AgentAppStores::file(&store_root);
    let agent = Agent::builder()
        .id(AgentId::new("agent.example.typed_output_events"))
        .name("typed output and events")
        .build()?;
    let app = AgentApp::builder(agent)
        .provider(
            "provider.fake",
            clawdia_sdk::testing::FakeProvider::with_responses([todo_json(
                "Review Phase 16",
                "high",
            )]),
        )?
        .stores(stores)
        .policy(AllowRunPolicy)
        .build()?;

    let run_id = RunId::new("run.example.typed_output_events");
    let result = app.run_typed::<TodoExtraction>(run_id.clone(), "extract one todo")?;
    let typed = serde_json::from_str::<TodoExtraction>(&result.output).map_err(|error| {
        AgentError::contract_violation(format!("example typed output decode failed: {error}"))
    })?;
    let evidence = app.run_evidence(&run_id)?;
    let report = app.run_report_from_evidence(&evidence, None)?;
    let validation_reports = result
        .structured_output
        .as_ref()
        .map_or(0, |artifacts| artifacts.validation_reports.len());

    println!(
        "typed_title={}; priority={}; validation_reports={}; events={}; records={}; report_records={}",
        typed.title,
        typed.priority,
        validation_reports,
        evidence.live_event_frames.len(),
        evidence.journal_records.len(),
        report.usage.record_count
    );

    drop(std::fs::remove_dir_all(store_root));
    Ok(())
}

fn todo_json(title: &str, priority: &str) -> String {
    json!({ "title": title, "priority": priority }).to_string()
}

fn todo_schema() -> Value {
    json!({
        "type": "object",
        "required": ["title", "priority"],
        "properties": {
            "title": { "type": "string" },
            "priority": { "type": "string" }
        },
        "additionalProperties": false
    })
}

fn temp_root(slug: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("clawdia-sdk-example-{slug}-{nanos}"))
}
