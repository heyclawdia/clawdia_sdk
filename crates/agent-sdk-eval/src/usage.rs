//! Deterministic usage reports derived from trace metrics.

use serde::{Deserialize, Serialize};

use agent_sdk_core::{AgentError, RunTrace};

use crate::{EvaluationScope, TraceMetrics};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Provider/tool usage report derived from durable trace evidence.
pub struct UsageReport {
    /// Scope this usage report describes.
    pub scope: EvaluationScope,
    /// Number of journal records inspected.
    pub record_count: usize,
    /// Number of runs represented in the source trace.
    pub run_count: usize,
    /// Number of turns represented in the source trace.
    pub turn_count: usize,
    /// Number of completed provider calls.
    pub provider_call_count: u64,
    /// Sum of provider input tokens.
    pub provider_input_tokens: u64,
    /// Sum of provider output tokens.
    pub provider_output_tokens: u64,
    /// Sum of provider total tokens.
    pub provider_total_tokens: u64,
    /// Number of distinct tool calls.
    pub tool_call_count: u64,
    /// Number of completed tool calls.
    pub tool_completed_count: u64,
    /// Number of failed, timed out, cancelled, denied, unknown, or recovery-required tool calls.
    pub tool_non_success_count: u64,
    /// Elapsed time across the scope when durable timestamps support it.
    pub elapsed_ms: Option<u64>,
    /// Sum of per-tool elapsed milliseconds when available.
    pub tool_total_elapsed_ms: Option<u64>,
    /// Limitations found while deriving usage.
    pub limitations: Vec<String>,
}

impl UsageReport {
    /// Builds a usage report from a run trace.
    pub fn from_run_trace(trace: &RunTrace) -> Result<Self, AgentError> {
        Self::from_trace_metrics(TraceMetrics::from_run_trace(trace)?)
    }

    /// Builds a usage report from precomputed trace metrics.
    pub fn from_trace_metrics(metrics: TraceMetrics) -> Result<Self, AgentError> {
        let mut limitations = Vec::new();
        if metrics.record_count == 0 {
            limitations.push("usage report has no journal records".to_string());
        }
        if metrics.provider_call_count > 0 && metrics.provider_total_tokens == 0 {
            limitations.push("provider usage did not include token counts".to_string());
        }
        if metrics.elapsed_ms.is_none() {
            limitations
                .push("scope elapsed time is unavailable from durable timestamps".to_string());
        }
        Ok(Self {
            scope: metrics.scope,
            record_count: metrics.record_count,
            run_count: metrics.run_count,
            turn_count: metrics.turn_count,
            provider_call_count: metrics.provider_call_count,
            provider_input_tokens: metrics.provider_input_tokens,
            provider_output_tokens: metrics.provider_output_tokens,
            provider_total_tokens: metrics.provider_total_tokens,
            tool_call_count: metrics.tool_call_count,
            tool_completed_count: metrics.tool_completed_count,
            tool_non_success_count: metrics
                .tool_failed_count
                .saturating_add(metrics.tool_timed_out_count)
                .saturating_add(metrics.tool_cancelled_count)
                .saturating_add(metrics.tool_denied_count)
                .saturating_add(metrics.tool_unknown_count)
                .saturating_add(metrics.tool_recovery_required_count),
            elapsed_ms: metrics.elapsed_ms,
            tool_total_elapsed_ms: metrics.tool_total_elapsed_ms,
            limitations,
        })
    }
}
