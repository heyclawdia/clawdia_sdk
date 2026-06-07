use clawdia_sdk::{
    core::{RunId, RunTrace},
    eval::{RunReport, StaticRateTable},
};

fn main() -> Result<(), clawdia_sdk::core::AgentError> {
    let run_id = RunId::new("run.example.report");
    let trace = RunTrace {
        run_id: Some(run_id.clone()),
        session_id: None,
        turn_traces: Vec::new(),
        records: Vec::new(),
    };
    let rates = StaticRateTable::new("USD", 1_000_000, 2_000_000, 100);
    let report = RunReport::from_run_trace(&trace, Some(&rates))?;
    println!(
        "{} {}",
        report.run_id.as_str(),
        report.limitations.items.join("; ")
    );
    Ok(())
}
