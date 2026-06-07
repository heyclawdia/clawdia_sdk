//! Run-level report helpers.

use serde::{Deserialize, Serialize};

use agent_sdk_core::{AgentError, RunId, RunTrace};

use crate::{CostPolicy, CostReport, UsageReport};

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
/// Limitations attached to a run report.
pub struct RunReportLimitations {
    /// Bounded limitations safe for logs and report views.
    pub items: Vec<String>,
}

impl RunReportLimitations {
    /// Creates limitations from report parts.
    pub fn from_parts(usage: &UsageReport, cost: Option<&CostReport>) -> Self {
        let mut items = usage.limitations.clone();
        if let Some(cost) = cost {
            items.extend(cost.limitations.clone());
        } else {
            items.push("cost report was not requested".to_string());
        }
        items.sort();
        items.dedup();
        Self { items }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Run report combining usage, optional cost, and limitations.
pub struct RunReport {
    /// Run id this report describes.
    pub run_id: RunId,
    /// Usage evidence derived from durable trace records.
    pub usage: UsageReport,
    /// Optional cost estimate.
    pub cost: Option<CostReport>,
    /// Report limitations and caveats.
    pub limitations: RunReportLimitations,
}

impl RunReport {
    /// Builds a run report from a run trace and optional cost policy.
    pub fn from_run_trace(
        trace: &RunTrace,
        cost_policy: Option<&dyn CostPolicy>,
    ) -> Result<Self, AgentError> {
        let run_id = trace.run_id.clone().ok_or_else(|| {
            AgentError::contract_violation("run report requires a run trace with run_id")
        })?;
        let usage = UsageReport::from_run_trace(trace)?;
        let cost = cost_policy.map(|policy| policy.estimate_cost(&usage));
        let limitations = RunReportLimitations::from_parts(&usage, cost.as_ref());
        Ok(Self {
            run_id,
            usage,
            cost,
            limitations,
        })
    }
}
