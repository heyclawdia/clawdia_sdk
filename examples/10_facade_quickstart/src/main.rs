use std::time::{SystemTime, UNIX_EPOCH};

use clawdia_sdk::prelude::*;

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
    let store_root = temp_root("facade-quickstart");
    let stores = AgentAppStores::file(&store_root);
    let agent = Agent::builder()
        .id(AgentId::new("agent.example.quickstart"))
        .name("quickstart")
        .build()?;
    let app = AgentApp::builder(agent)
        .provider(
            "provider.fake",
            clawdia_sdk::testing::FakeProvider::with_responses(["quickstart complete"]),
        )?
        .stores(stores)
        .policy(AllowRunPolicy)
        .build()?;

    let run_id = RunId::new("run.example.quickstart");
    let result = app.run_text(run_id.clone(), "say hello")?;
    let evidence = app.run_evidence(&run_id)?;
    let report = app.run_report_from_evidence(&evidence, None)?;

    println!(
        "output={}; status={:?}; records={}; events={}; provider_calls={}",
        result.output,
        result.status,
        evidence.journal_records.len(),
        evidence.live_event_frames.len(),
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
