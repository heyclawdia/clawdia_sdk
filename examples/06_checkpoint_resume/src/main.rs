use std::time::{SystemTime, UNIX_EPOCH};

use clawdia_sdk::{
    core::{JournalRecord, JournalRecordBase, ReplayMode, ReplayReducer, RunCheckpoint},
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
    let store_root = temp_root("checkpoint-resume");
    let stores = AgentAppStores::file(&store_root);
    let agent = Agent::builder()
        .id(AgentId::new("agent.example.checkpoint_resume"))
        .name("checkpoint resume")
        .build()?;
    let app = AgentApp::builder(agent)
        .provider(
            "provider.fake",
            clawdia_sdk::testing::FakeProvider::with_responses(["checkpoint ready"]),
        )?
        .stores(stores.clone())
        .policy(AllowRunPolicy)
        .build()?;

    let run_id = RunId::new("run.example.checkpoint_resume");
    let result = app.run_text(run_id.clone(), "produce checkpoint evidence")?;
    let evidence = app.run_evidence(&run_id)?;
    let latest_journal_seq = evidence
        .journal_records
        .iter()
        .map(|record| record.journal_seq)
        .max()
        .ok_or_else(|| AgentError::contract_violation("run produced no journal records"))?;
    let checkpoint = RunCheckpoint {
        checkpoint_id: "checkpoint.example.resume.ready".to_string(),
        run_id: run_id.clone(),
        checkpoint_seq: 1,
        covers_journal_seq: latest_journal_seq,
        loop_state: "terminal:completed".to_string(),
        turn_id: None,
        attempt_id: None,
        runtime_package_fingerprint: "runtime.package.fingerprint.example.resume".to_string(),
        pending_side_effects: Vec::new(),
        pending_approvals: Vec::new(),
        content_ref_manifest: Vec::new(),
        state_hash: "state.hash.example.resume".to_string(),
        created_at_millis: latest_journal_seq,
        writer_id: "writer.example.resume".to_string(),
    };
    stores
        .checkpoint
        .as_ref()
        .expect("file stores include checkpoint store")
        .save(checkpoint.clone(), latest_journal_seq)?;
    let checkpoint_record = JournalRecord::checkpoint(
        JournalRecordBase::new(
            latest_journal_seq + 1,
            "journal.example.resume.checkpoint",
            run_id.clone(),
            app.agent().id().clone(),
            SourceRef::with_kind(SourceKind::Sdk, "source.example.resume"),
        ),
        checkpoint.clone(),
    );
    stores.journal.append(checkpoint_record.clone())?;

    let durable_records = app.journal_records_for_run(&run_id)?;
    let durable_checkpoint_record = durable_records
        .iter()
        .find(|record| record.record_id == checkpoint_record.record_id)
        .cloned()
        .ok_or_else(|| {
            AgentError::contract_violation("checkpoint record was not readable from journal")
        })?;
    let mut replay = ReplayReducer::new(ReplayMode::ResumeReplay);
    replay.apply(durable_checkpoint_record)?;
    let replay_result = replay.finish()?;
    let loaded_checkpoint = app
        .latest_checkpoint(&run_id)?
        .expect("checkpoint accelerator is present");

    println!(
        "output={}; records={}; resume_allowed={}; next_state={}; checkpoint={}",
        result.output,
        durable_records.len(),
        replay_result.resume_allowed,
        replay_result.next_loop_state.unwrap_or_default(),
        loaded_checkpoint.checkpoint_id
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
