use std::time::{SystemTime, UNIX_EPOCH};

use clawdia_sdk::{prelude::*, providers::OpenAiResponsesAdapter};

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
    let store_root = temp_root("live-provider-text-run");
    let stores = AgentAppStores::file(&store_root);
    let agent = Agent::builder()
        .id(AgentId::new("agent.example.live_provider_text"))
        .name("live provider text")
        .build()?;
    let builder = AgentApp::builder(agent)
        .stores(stores)
        .policy(AllowRunPolicy);
    let app = if std::env::var_os("OPENAI_API_KEY").is_some() {
        let model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4.1-mini".to_string());
        builder
            .provider_route("provider.openai.responses", model.clone())
            .provider(
                "provider.openai.responses",
                OpenAiResponsesAdapter::from_env(model)?,
            )?
            .build()?
    } else {
        builder
            .provider(
                "provider.fake",
                clawdia_sdk::testing::FakeProvider::with_responses(["fake provider text run"]),
            )?
            .build()?
    };

    let run_id = RunId::new("run.example.live_provider_text");
    let result = app.run_text(run_id.clone(), "reply with one short sentence")?;
    let records = app.journal_records_for_run(&run_id)?;
    println!(
        "output={}; status={:?}; records={}",
        result.output,
        result.status,
        records.len()
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
