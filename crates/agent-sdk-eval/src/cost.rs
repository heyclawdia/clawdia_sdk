//! Cost report helpers over deterministic usage reports.

use serde::{Deserialize, Serialize};

use crate::UsageReport;

/// Policy contract for estimating cost from usage evidence.
pub trait CostPolicy: Send + Sync {
    /// Estimates cost from one usage report.
    fn estimate_cost(&self, usage: &UsageReport) -> CostReport;
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Static provider-neutral rate table.
///
/// Token rates are expressed in micros of `currency` per million tokens.
/// Tool rates are expressed in micros of `currency` per tool call.
pub struct StaticRateTable {
    /// Currency code or accounting unit.
    pub currency: String,
    /// Input-token rate per one million input tokens, in micros.
    pub input_token_micros_per_million: u64,
    /// Output-token rate per one million output tokens, in micros.
    pub output_token_micros_per_million: u64,
    /// Tool-call rate per call, in micros.
    pub tool_call_micros: u64,
}

impl StaticRateTable {
    /// Creates a static rate table.
    pub fn new(
        currency: impl Into<String>,
        input_token_micros_per_million: u64,
        output_token_micros_per_million: u64,
        tool_call_micros: u64,
    ) -> Self {
        Self {
            currency: currency.into(),
            input_token_micros_per_million,
            output_token_micros_per_million,
            tool_call_micros,
        }
    }
}

impl CostPolicy for StaticRateTable {
    fn estimate_cost(&self, usage: &UsageReport) -> CostReport {
        let input_cost_micros = usage
            .provider_input_tokens
            .saturating_mul(self.input_token_micros_per_million)
            / 1_000_000;
        let output_cost_micros = usage
            .provider_output_tokens
            .saturating_mul(self.output_token_micros_per_million)
            / 1_000_000;
        let tool_cost_micros = usage.tool_call_count.saturating_mul(self.tool_call_micros);
        let total_cost_micros = input_cost_micros
            .saturating_add(output_cost_micros)
            .saturating_add(tool_cost_micros);
        let mut limitations = Vec::new();
        if usage.provider_call_count == 0 && usage.tool_call_count == 0 {
            limitations.push("cost report has no provider or tool usage".to_string());
        }
        CostReport {
            currency: self.currency.clone(),
            input_cost_micros,
            output_cost_micros,
            tool_cost_micros,
            total_cost_micros,
            limitations,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Deterministic cost estimate from a usage report and cost policy.
pub struct CostReport {
    /// Currency code or accounting unit.
    pub currency: String,
    /// Input token cost in micros.
    pub input_cost_micros: u64,
    /// Output token cost in micros.
    pub output_cost_micros: u64,
    /// Tool call cost in micros.
    pub tool_cost_micros: u64,
    /// Total estimated cost in micros.
    pub total_cost_micros: u64,
    /// Limitations found while estimating cost.
    pub limitations: Vec<String>,
}
