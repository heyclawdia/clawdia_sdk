use std::time::{SystemTime, UNIX_EPOCH};

use clawdia_sdk::{
    core::RunTrace,
    eval::{RunReport, StaticRateTable, UsageReport},
    prelude::*,
};

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
    let store_root = temp_root("token-tracking-costs");
    let stores = AgentAppStores::file(&store_root);
    let agent = Agent::builder()
        .id(AgentId::new("agent.example.token_cost"))
        .name("token cost")
        .build()?;
    let app = AgentApp::builder(agent)
        .provider(
            "provider.fake",
            clawdia_sdk::testing::FakeProvider::with_responses(["cost evidence ready"]),
        )?
        .stores(stores)
        .policy(AllowRunPolicy)
        .build()?;

    let run_id = RunId::new("run.example.token_cost");
    let result = app.run_text(run_id.clone(), "produce token evidence")?;
    let records = app.journal_records_for_run(&run_id)?;
    let trace = RunTrace::from_records(&run_id, records.iter());
    let usage = UsageReport::from_run_trace(&trace)?;
    let rates = StaticRateTable::new("USD", 1_000_000, 2_000_000, 100);
    let report = RunReport::from_run_trace(&trace, Some(&rates))?;
    let cost = report
        .cost
        .ok_or_else(|| AgentError::contract_violation("cost report missing"))?;

    println!(
        "output={}; records={}; provider_tokens={}; input_tokens={}; output_tokens={}; cost_micros={}",
        result.output,
        usage.record_count,
        usage.provider_total_tokens,
        usage.provider_input_tokens,
        usage.provider_output_tokens,
        cost.total_cost_micros
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
