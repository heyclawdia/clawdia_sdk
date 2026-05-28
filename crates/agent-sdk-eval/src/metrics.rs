//! Deterministic metrics derived from core traces and journal records.

use serde::{Deserialize, Serialize};

use agent_sdk_core::{
    AgentError, EffectId, EffectKind, EntityKind, EntityRef, JournalRecord, JournalRecordPayload,
    RunTrace, SessionTimeline, ToolCallId, ToolCallRecord, ToolCallRecordStatus, TurnTrace,
};

use crate::{EvaluationMetricDelta, EvaluationScope};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Deterministic metrics for a turn, run, session, or custom trace scope.
pub struct TraceMetrics {
    /// Scope these metrics describe.
    pub scope: EvaluationScope,
    /// Earliest non-zero journal timestamp observed for this scope.
    pub started_at_millis: Option<u64>,
    /// Latest non-zero journal timestamp observed for this scope.
    pub ended_at_millis: Option<u64>,
    /// `ended_at_millis - started_at_millis` when both are available.
    pub elapsed_ms: Option<u64>,
    /// Number of journal records inspected for this scope.
    pub record_count: usize,
    /// Number of distinct runs represented in this scope.
    pub run_count: usize,
    /// Number of distinct turns represented in this scope.
    pub turn_count: usize,
    /// Number of completed provider attempts.
    pub provider_call_count: u64,
    /// Sum of provider-reported input tokens.
    pub provider_input_tokens: u64,
    /// Sum of provider-reported output tokens.
    pub provider_output_tokens: u64,
    /// Sum of provider-reported total tokens.
    pub provider_total_tokens: u64,
    /// Number of distinct tool calls represented in this scope.
    pub tool_call_count: u64,
    /// Number of tools whose latest terminal status is completed.
    pub tool_completed_count: u64,
    /// Number of tools whose latest terminal status is failed.
    pub tool_failed_count: u64,
    /// Number of tools whose latest terminal status is timed out.
    pub tool_timed_out_count: u64,
    /// Number of tools whose latest terminal status is cancelled.
    pub tool_cancelled_count: u64,
    /// Number of tools denied before execution.
    pub tool_denied_count: u64,
    /// Number of tools whose result was rewritten by a hook.
    pub tool_rewritten_count: u64,
    /// Number of tools whose latest terminal status is unknown.
    pub tool_unknown_count: u64,
    /// Number of tools that require recovery.
    pub tool_recovery_required_count: u64,
    /// Sum of per-tool elapsed milliseconds when at least one tool has timing.
    pub tool_total_elapsed_ms: Option<u64>,
    /// Per-tool metrics, one row per distinct tool call id.
    pub tools: Vec<ToolTraceMetric>,
}

impl TraceMetrics {
    /// Builds deterministic metrics from a turn trace.
    pub fn from_turn_trace(trace: &TurnTrace) -> Result<Self, AgentError> {
        let turn_id = trace.turn_id.clone().ok_or_else(|| {
            AgentError::contract_violation("turn trace is missing turn id for metrics")
        })?;
        let run_count = if trace.run_ids.is_empty() {
            unique_run_count(&trace.records)
        } else {
            trace.run_ids.len()
        };
        Ok(Self::from_records(
            EvaluationScope::Turn {
                session_id: trace.session_id.clone(),
                turn_id,
            },
            run_count,
            1,
            &trace.records,
        ))
    }

    /// Builds deterministic metrics from a run trace.
    pub fn from_run_trace(trace: &RunTrace) -> Result<Self, AgentError> {
        let run_id = trace.run_id.clone().ok_or_else(|| {
            AgentError::contract_violation("run trace is missing run id for metrics")
        })?;
        let records = if trace.records.is_empty() {
            trace
                .turn_traces
                .iter()
                .flat_map(|turn| turn.records.iter().cloned())
                .collect::<Vec<_>>()
        } else {
            trace.records.clone()
        };
        let turn_count = if trace.turn_traces.is_empty() {
            unique_turn_count(&records)
        } else {
            trace.turn_traces.len()
        };
        Ok(Self::from_records(
            EvaluationScope::Run { run_id },
            1,
            turn_count,
            &records,
        ))
    }

    /// Builds deterministic metrics from a session timeline.
    pub fn from_session_timeline(timeline: &SessionTimeline) -> Self {
        let records = timeline
            .turns
            .iter()
            .flat_map(|turn| turn.records.iter().cloned())
            .collect::<Vec<_>>();
        let run_count = timeline
            .turns
            .iter()
            .flat_map(|turn| turn.run_ids.iter().cloned())
            .fold(Vec::new(), |mut runs, run_id| {
                push_unique(&mut runs, run_id);
                runs
            })
            .len()
            .max(unique_run_count(&records));
        Self::from_records(
            EvaluationScope::Session {
                session_id: timeline.session_id.clone(),
            },
            run_count,
            timeline.turns.len(),
            &records,
        )
    }

    fn from_records(
        scope: EvaluationScope,
        run_count: usize,
        turn_count: usize,
        records: &[JournalRecord],
    ) -> Self {
        let started_at_millis = records
            .iter()
            .filter_map(|record| non_zero_timestamp(record.timestamp_millis))
            .min();
        let ended_at_millis = records
            .iter()
            .filter_map(|record| non_zero_timestamp(record.timestamp_millis))
            .max();
        let elapsed_ms = started_at_millis
            .zip(ended_at_millis)
            .and_then(|(started_at, ended_at)| ended_at.checked_sub(started_at));
        let mut provider_call_count = 0;
        let mut provider_input_tokens = 0;
        let mut provider_output_tokens = 0;
        let mut provider_total_tokens = 0;

        for record in records {
            if let JournalRecordPayload::ModelAttempt(attempt) = &record.payload
                && attempt.stop_reason.is_some()
            {
                provider_call_count += 1;
                if let Some(usage) = &attempt.usage {
                    provider_input_tokens += u64::from(usage.input_tokens.unwrap_or_default());
                    provider_output_tokens += u64::from(usage.output_tokens.unwrap_or_default());
                    provider_total_tokens += u64::from(usage.total_tokens.unwrap_or_default());
                }
            }
        }

        let tools = tool_metrics(records);
        let tool_total_elapsed_ms = tools
            .iter()
            .filter_map(|tool| tool.elapsed_ms)
            .reduce(|left, right| left.saturating_add(right));
        let mut metrics = Self {
            scope,
            started_at_millis,
            ended_at_millis,
            elapsed_ms,
            record_count: records.len(),
            run_count,
            turn_count,
            provider_call_count,
            provider_input_tokens,
            provider_output_tokens,
            provider_total_tokens,
            tool_call_count: tools.len() as u64,
            tool_completed_count: 0,
            tool_failed_count: 0,
            tool_timed_out_count: 0,
            tool_cancelled_count: 0,
            tool_denied_count: 0,
            tool_rewritten_count: 0,
            tool_unknown_count: 0,
            tool_recovery_required_count: 0,
            tool_total_elapsed_ms,
            tools,
        };
        metrics.count_tool_statuses();
        metrics
    }

    fn count_tool_statuses(&mut self) {
        for tool in &self.tools {
            match tool.status {
                ToolCallRecordStatus::Completed => self.tool_completed_count += 1,
                ToolCallRecordStatus::Failed => self.tool_failed_count += 1,
                ToolCallRecordStatus::TimedOut => self.tool_timed_out_count += 1,
                ToolCallRecordStatus::Cancelled => self.tool_cancelled_count += 1,
                ToolCallRecordStatus::DeniedBeforeExecution => self.tool_denied_count += 1,
                ToolCallRecordStatus::ResultRewritten => self.tool_rewritten_count += 1,
                ToolCallRecordStatus::Unknown => self.tool_unknown_count += 1,
                ToolCallRecordStatus::RecoveryRequired => self.tool_recovery_required_count += 1,
                ToolCallRecordStatus::Requested
                | ToolCallRecordStatus::RequestModified
                | ToolCallRecordStatus::IntentRecorded => {}
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Deterministic metrics for one tool call.
pub struct ToolTraceMetric {
    /// Tool call ref represented by this row.
    pub tool_call_ref: EntityRef,
    /// Canonical tool name recorded by the SDK.
    pub canonical_tool_name: String,
    /// Latest observed tool status.
    pub status: ToolCallRecordStatus,
    /// Timestamp of the tool-execution intent record when available.
    pub started_at_millis: Option<u64>,
    /// Timestamp of the matching terminal result record when available.
    pub ended_at_millis: Option<u64>,
    /// `ended_at_millis - started_at_millis` when durable evidence supports it.
    pub elapsed_ms: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Deterministic comparison between two trace metric sets.
pub struct TraceMetricsComparison {
    /// Observed metrics.
    pub observed: TraceMetrics,
    /// Baseline or comparison metrics.
    pub baseline: TraceMetrics,
    /// Deterministic metric deltas computed as observed minus baseline.
    pub metric_deltas: Vec<EvaluationMetricDelta>,
}

impl TraceMetricsComparison {
    /// Compares two session timelines.
    pub fn sessions(observed: &SessionTimeline, baseline: &SessionTimeline) -> Self {
        Self::from_metrics(
            TraceMetrics::from_session_timeline(observed),
            TraceMetrics::from_session_timeline(baseline),
        )
    }

    /// Compares two already computed metric sets.
    pub fn from_metrics(observed: TraceMetrics, baseline: TraceMetrics) -> Self {
        let metric_deltas = metric_deltas(&observed, &baseline);
        Self {
            observed,
            baseline,
            metric_deltas,
        }
    }
}

#[derive(Clone)]
struct ToolMetricState {
    tool_call_id: ToolCallId,
    entries: Vec<ToolJournalEntry>,
}

#[derive(Clone)]
struct ToolJournalEntry {
    journal_seq: u64,
    timestamp_millis: u64,
    record: ToolCallRecord,
}

fn tool_metrics(records: &[JournalRecord]) -> Vec<ToolTraceMetric> {
    let mut states = Vec::<ToolMetricState>::new();
    for record in records {
        let JournalRecordPayload::Tool(tool_record) = &record.payload else {
            continue;
        };
        if let Some(state) = states
            .iter_mut()
            .find(|state| state.tool_call_id == tool_record.tool_call_id)
        {
            state.entries.push(ToolJournalEntry {
                journal_seq: record.journal_seq,
                timestamp_millis: record.timestamp_millis,
                record: tool_record.clone(),
            });
        } else {
            states.push(ToolMetricState {
                tool_call_id: tool_record.tool_call_id.clone(),
                entries: vec![ToolJournalEntry {
                    journal_seq: record.journal_seq,
                    timestamp_millis: record.timestamp_millis,
                    record: tool_record.clone(),
                }],
            });
        }
    }

    states
        .into_iter()
        .filter_map(|state| tool_metric(state.entries))
        .collect()
}

fn tool_metric(mut entries: Vec<ToolJournalEntry>) -> Option<ToolTraceMetric> {
    entries.sort_by_key(|entry| entry.journal_seq);
    let latest = entries.last()?.record.clone();
    let start = entries.iter().find_map(tool_start);
    let end = start
        .as_ref()
        .and_then(|start| matching_tool_end(&entries, start));
    let elapsed_ms = start
        .as_ref()
        .zip(end.as_ref())
        .and_then(|(start, end)| end.timestamp_millis.checked_sub(start.timestamp_millis));

    Some(ToolTraceMetric {
        tool_call_ref: EntityRef::new(EntityKind::ToolCall, latest.tool_call_id),
        canonical_tool_name: latest.canonical_tool_name.as_str().to_string(),
        status: latest.status,
        started_at_millis: start.map(|start| start.timestamp_millis),
        ended_at_millis: end.map(|end| end.timestamp_millis),
        elapsed_ms,
    })
}

struct ToolStart {
    effect_id: EffectId,
    timestamp_millis: u64,
}

struct ToolEnd {
    timestamp_millis: u64,
}

fn tool_start(entry: &ToolJournalEntry) -> Option<ToolStart> {
    if entry.timestamp_millis == 0 || entry.record.status != ToolCallRecordStatus::IntentRecorded {
        return None;
    }
    let intent = entry.record.effect_intent.as_ref()?;
    if intent.kind != EffectKind::ToolExecution {
        return None;
    }
    Some(ToolStart {
        effect_id: intent.effect_id.clone(),
        timestamp_millis: entry.timestamp_millis,
    })
}

fn matching_tool_end(entries: &[ToolJournalEntry], start: &ToolStart) -> Option<ToolEnd> {
    entries
        .iter()
        .filter(|entry| {
            entry.timestamp_millis > start.timestamp_millis && is_terminal_status(&entry.record)
        })
        .filter_map(|entry| {
            let result = entry.record.effect_result.as_ref()?;
            (result.effect_id == start.effect_id).then_some(ToolEnd {
                timestamp_millis: entry.timestamp_millis,
            })
        })
        .max_by_key(|end| end.timestamp_millis)
}

fn is_terminal_status(record: &ToolCallRecord) -> bool {
    matches!(
        record.status,
        ToolCallRecordStatus::Completed
            | ToolCallRecordStatus::Failed
            | ToolCallRecordStatus::TimedOut
            | ToolCallRecordStatus::Cancelled
            | ToolCallRecordStatus::DeniedBeforeExecution
            | ToolCallRecordStatus::ResultRewritten
            | ToolCallRecordStatus::Unknown
            | ToolCallRecordStatus::RecoveryRequired
    )
}

fn metric_deltas(observed: &TraceMetrics, baseline: &TraceMetrics) -> Vec<EvaluationMetricDelta> {
    let mut deltas = Vec::new();
    push_count_delta(
        &mut deltas,
        "trace.elapsed_ms",
        observed.elapsed_ms,
        baseline.elapsed_ms,
    );
    push_delta(
        &mut deltas,
        "trace.provider_call_count",
        observed.provider_call_count,
        baseline.provider_call_count,
    );
    push_delta(
        &mut deltas,
        "trace.provider_input_tokens",
        observed.provider_input_tokens,
        baseline.provider_input_tokens,
    );
    push_delta(
        &mut deltas,
        "trace.provider_output_tokens",
        observed.provider_output_tokens,
        baseline.provider_output_tokens,
    );
    push_delta(
        &mut deltas,
        "trace.provider_total_tokens",
        observed.provider_total_tokens,
        baseline.provider_total_tokens,
    );
    push_delta(
        &mut deltas,
        "trace.tool_call_count",
        observed.tool_call_count,
        baseline.tool_call_count,
    );
    push_delta(
        &mut deltas,
        "trace.tool_completed_count",
        observed.tool_completed_count,
        baseline.tool_completed_count,
    );
    push_delta(
        &mut deltas,
        "trace.tool_failed_count",
        observed.tool_failed_count,
        baseline.tool_failed_count,
    );
    push_delta(
        &mut deltas,
        "trace.tool_denied_count",
        observed.tool_denied_count,
        baseline.tool_denied_count,
    );
    push_count_delta(
        &mut deltas,
        "trace.tool_total_elapsed_ms",
        observed.tool_total_elapsed_ms,
        baseline.tool_total_elapsed_ms,
    );
    deltas
}

fn push_delta(
    deltas: &mut Vec<EvaluationMetricDelta>,
    metric_ref: &'static str,
    observed: u64,
    baseline: u64,
) {
    deltas.push(EvaluationMetricDelta::new(
        metric_ref,
        signed_delta(observed, baseline),
        format!("{metric_ref}: observed={observed}, baseline={baseline}"),
    ));
}

fn push_count_delta(
    deltas: &mut Vec<EvaluationMetricDelta>,
    metric_ref: &'static str,
    observed: Option<u64>,
    baseline: Option<u64>,
) {
    if let Some((observed, baseline)) = observed.zip(baseline) {
        push_delta(deltas, metric_ref, observed, baseline);
    }
}

fn signed_delta(observed: u64, baseline: u64) -> String {
    let delta = i128::from(observed) - i128::from(baseline);
    if delta >= 0 {
        format!("+{delta}")
    } else {
        delta.to_string()
    }
}

fn unique_run_count(records: &[JournalRecord]) -> usize {
    records
        .iter()
        .fold(Vec::new(), |mut run_ids, record| {
            push_unique(&mut run_ids, record.run_id.clone());
            run_ids
        })
        .len()
}

fn unique_turn_count(records: &[JournalRecord]) -> usize {
    records
        .iter()
        .filter_map(|record| record.turn_id.clone())
        .fold(Vec::new(), |mut turn_ids, turn_id| {
            push_unique(&mut turn_ids, turn_id);
            turn_ids
        })
        .len()
}

fn non_zero_timestamp(timestamp_millis: u64) -> Option<u64> {
    (timestamp_millis > 0).then_some(timestamp_millis)
}

fn push_unique<T: Eq>(items: &mut Vec<T>, value: T) {
    if !items.contains(&value) {
        items.push(value);
    }
}
