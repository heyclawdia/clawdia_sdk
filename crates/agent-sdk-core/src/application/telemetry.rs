//! Application-layer coordination over core primitives. Use these services to lower
//! helpers, drive runs, validate output, coordinate tools, approvals, delivery,
//! isolation, telemetry, and feature layers. Methods in this layer may call
//! configured ports, mutate in-memory stores, append journals, or publish events as
//! documented. This file contains the telemetry portion of that contract.
//!
use std::{
    collections::{BTreeMap, VecDeque},
    num::NonZeroUsize,
    sync::Arc,
};

use serde::{Deserialize, Serialize};

use crate::{
    domain::{
        AgentError, AgentErrorKind, DestinationKind, DestinationRef, PolicyRef, RetentionClass,
        RetryClassification,
    },
    event::{AgentEvent, EventCursor, EventFamily, EventKind, EventStreamScope},
    policy::{ContentCaptureMode as PolicyContentCaptureMode, ContentCapturePolicy},
    telemetry_ports::{TelemetrySink, TelemetrySinkError, TelemetrySinkSpec},
    telemetry_records::{
        TELEMETRY_SCHEMA_VERSION, TelemetryContentCaptureMode, TelemetryExportCursor,
        TelemetryProjection, TelemetryProjectionId, TelemetryProjectionKind, TelemetryRecord,
        TelemetryRecordId, TelemetrySinkFailureKind, TelemetrySinkFailureRecord,
        TelemetrySinkHealth, TelemetrySinkHealthState, TelemetrySinkId, TelemetrySinkKind,
        TelemetrySinkRecoveryRecord, TelemetrySourceCursor, TelemetrySourceRecord,
        TelemetryTerminalStatus, TelemetryUsageRecordId, UsageUnits,
    },
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Holds telemetry fanout config application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct TelemetryFanoutConfig {
    /// Queue capacity used by this record or request.
    pub queue_capacity: NonZeroUsize,
    /// Queue slots reserved for terminal frames.
    /// This keeps important terminal events available even when non-terminal frames overflow.
    pub terminal_reserve: NonZeroUsize,
    /// Overflow policy applied when a subscriber queue reaches capacity.
    /// It decides whether to drop, summarize, backpressure, or fail the subscriber.
    pub overflow: TelemetryOverflowPolicy,
    /// Sink isolation used by this record or request.
    pub sink_isolation: TelemetrySinkIsolationPolicy,
}

impl TelemetryFanoutConfig {
    /// Returns an updated value with safe defaults configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn safe_defaults() -> Self {
        Self {
            queue_capacity: NonZeroUsize::new(64).expect("nonzero queue capacity"),
            terminal_reserve: NonZeroUsize::new(4).expect("nonzero terminal reserve"),
            overflow: TelemetryOverflowPolicy::DropNonTerminalProgress,
            sink_isolation: TelemetrySinkIsolationPolicy::IsolateEachSink,
        }
    }

    /// Builds the tiny for tests value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn tiny_for_tests() -> Self {
        Self {
            queue_capacity: NonZeroUsize::new(2).expect("nonzero queue capacity"),
            terminal_reserve: NonZeroUsize::new(1).expect("nonzero terminal reserve"),
            overflow: TelemetryOverflowPolicy::DropNonTerminalProgress,
            sink_isolation: TelemetrySinkIsolationPolicy::IsolateEachSink,
        }
    }
}

impl Default for TelemetryFanoutConfig {
    fn default() -> Self {
        Self::safe_defaults()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite telemetry overflow policy cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum TelemetryOverflowPolicy {
    /// Use this variant when the contract needs to represent drop non terminal progress; selecting it has no side effect by itself.
    DropNonTerminalProgress,
    /// Use this variant when the contract needs to represent coalesce progress by run; selecting it has no side effect by itself.
    CoalesceProgressByRun,
    /// Use this variant when the contract needs to represent fail sink not run; selecting it has no side effect by itself.
    FailSinkNotRun,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite telemetry sink isolation policy cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum TelemetrySinkIsolationPolicy {
    /// Use this variant when the contract needs to represent isolate each sink; selecting it has no side effect by itself.
    IsolateEachSink,
}

#[derive(Default)]
/// Holds telemetry fanout application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct TelemetryFanout {
    config: TelemetryFanoutConfig,
    sinks: BTreeMap<TelemetrySinkId, TelemetrySinkState>,
}

impl TelemetryFanout {
    /// Creates a new application::telemetry value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new(config: TelemetryFanoutConfig) -> Self {
        Self {
            config,
            sinks: BTreeMap::new(),
        }
    }

    /// Returns an updated value with safe defaults configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn safe_defaults() -> Self {
        Self::new(TelemetryFanoutConfig::safe_defaults())
    }

    /// Register sink.
    /// This adds the sink to telemetry fanout state and initializes its bounded in-memory
    /// queue.
    pub fn register_sink(&mut self, sink: Arc<dyn TelemetrySink>) -> Result<(), AgentError> {
        let spec = sink.spec().clone();
        if spec.sink_id.as_str().is_empty() {
            return Err(AgentError::missing_required_field("telemetry.sink_id"));
        }
        self.sinks
            .insert(spec.sink_id.clone(), TelemetrySinkState::new(sink));
        Ok(())
    }

    /// Returns sink queue len for callers that need to inspect the contract state.
    /// This reads the in-memory queue length for one sink and does not drain or export
    /// telemetry.
    pub fn sink_queue_len(&self, sink_id: &TelemetrySinkId) -> Option<usize> {
        self.sinks.get(sink_id).map(|state| state.queue.len())
    }

    /// Returns queued for sink for callers that need to inspect the contract state.
    /// This clones queued in-memory telemetry projections for inspection and does not drain or
    /// export them.
    pub fn queued_for_sink(&self, sink_id: &TelemetrySinkId) -> Vec<TelemetryProjection> {
        self.sinks
            .get(sink_id)
            .map(|state| state.queue.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Sets try record on the value and returns it.
    /// This enqueues a telemetry projection for eligible sinks and reports overflow or drop
    /// outcomes.
    pub fn try_record(&mut self, projection: TelemetryProjection) -> TelemetryFanoutReport {
        let mut report = TelemetryFanoutReport::default();
        for state in self.sinks.values_mut() {
            let projection = apply_sink_content_boundary(&projection, state.sink.spec());
            state.enqueue(&self.config, projection, &mut report);
        }
        report
    }

    /// Drain sink.
    /// This drains queued projections for one sink so tests or adapters can export them.
    pub fn drain_sink(
        &mut self,
        sink_id: &TelemetrySinkId,
    ) -> Result<TelemetryDrainReport, AgentError> {
        let Some(state) = self.sinks.get_mut(sink_id) else {
            return Err(AgentError::host_configuration_needed(
                "telemetry sink is not registered",
            ));
        };
        Ok(state.drain())
    }
}

struct TelemetrySinkState {
    sink: Arc<dyn TelemetrySink>,
    queue: VecDeque<TelemetryProjection>,
    cursor: TelemetryExportCursor,
    failed: bool,
    dropped_count: u64,
    next_record_seq: u64,
}

impl TelemetrySinkState {
    fn new(sink: Arc<dyn TelemetrySink>) -> Self {
        let sink_id = sink.spec().sink_id.clone();
        Self {
            sink,
            queue: VecDeque::new(),
            cursor: TelemetryExportCursor::new(sink_id),
            failed: false,
            dropped_count: 0,
            next_record_seq: 0,
        }
    }

    fn enqueue(
        &mut self,
        config: &TelemetryFanoutConfig,
        projection: TelemetryProjection,
        report: &mut TelemetryFanoutReport,
    ) {
        if self.has_room_for(config, &projection) {
            self.queue.push_back(projection);
            report.enqueued += 1;
            return;
        }

        if projection.is_terminal_preserved() {
            while self.queue.len() >= self.capacity(config) && self.drop_oldest_nonterminal() {
                report.dropped += 1;
            }
            if self.queue.len() < self.capacity(config) {
                self.queue.push_back(projection.clone());
                report.enqueued += 1;
                report.records.push(self.failure_record(
                    &projection,
                    TelemetrySinkFailureKind::Overflow,
                    true,
                    projection.source_record.source_cursor.clone(),
                    "telemetry terminal projection preserved by dropping non-terminal progress",
                ));
                return;
            }
            self.dropped_count += 1;
            report.dropped += 1;
            report.records.push(self.failure_record(
                &projection,
                TelemetrySinkFailureKind::Overflow,
                false,
                projection.source_record.source_cursor.clone(),
                "telemetry terminal projection could not enter the bounded sink queue",
            ));
            return;
        }

        match config.overflow {
            TelemetryOverflowPolicy::DropNonTerminalProgress => {
                self.dropped_count += 1;
                report.dropped += 1;
                report.records.push(self.failure_record(
                    &projection,
                    TelemetrySinkFailureKind::Overflow,
                    true,
                    projection.source_record.source_cursor.clone(),
                    "telemetry non-terminal progress dropped under sink backpressure",
                ));
            }
            TelemetryOverflowPolicy::CoalesceProgressByRun => {
                if let Some(index) = self.queue.iter().position(|queued| {
                    !queued.is_terminal_preserved() && queued.run_id == projection.run_id
                }) {
                    self.queue.remove(index);
                    self.dropped_count += 1;
                    report.dropped += 1;
                }
                if self.has_room_for(config, &projection) {
                    self.queue.push_back(projection);
                    report.enqueued += 1;
                } else {
                    self.dropped_count += 1;
                    report.dropped += 1;
                }
            }
            TelemetryOverflowPolicy::FailSinkNotRun => {
                self.failed = true;
                self.dropped_count += 1;
                report.dropped += 1;
                report.records.push(self.failure_record(
                    &projection,
                    TelemetrySinkFailureKind::Overflow,
                    true,
                    projection.source_record.source_cursor.clone(),
                    "telemetry sink marked failed by overflow; run state is unaffected",
                ));
            }
        }
    }

    fn drain(&mut self) -> TelemetryDrainReport {
        let mut report = TelemetryDrainReport::default();
        while let Some(projection) = self.queue.front().cloned() {
            let attempted = self
                .cursor
                .clone()
                .attempted(projection.source_record.source_cursor.clone());
            match self.sink.export(&projection, &attempted) {
                Ok(ack) => {
                    self.cursor = ack.cursor;
                    self.queue.pop_front();
                    report.exported += 1;
                    if self.failed {
                        self.failed = false;
                        report.records.push(self.recovery_record(&projection));
                    }
                }
                Err(error) => {
                    self.failed = true;
                    report
                        .records
                        .push(self.export_failure_record(&projection, error));
                    break;
                }
            }
        }
        report
    }

    fn has_room_for(
        &self,
        config: &TelemetryFanoutConfig,
        projection: &TelemetryProjection,
    ) -> bool {
        if projection.is_terminal_preserved() {
            return self.queue.len() < self.capacity(config);
        }
        self.queue.len() < self.capacity(config)
            && self.nonterminal_count() < self.normal_capacity(config)
    }

    fn capacity(&self, config: &TelemetryFanoutConfig) -> usize {
        self.sink
            .spec()
            .queue_capacity
            .get()
            .min(config.queue_capacity.get())
    }

    fn normal_capacity(&self, config: &TelemetryFanoutConfig) -> usize {
        let terminal_reserve = self
            .sink
            .spec()
            .terminal_reserve
            .get()
            .max(config.terminal_reserve.get())
            .min(self.capacity(config));
        self.capacity(config).saturating_sub(terminal_reserve)
    }

    fn nonterminal_count(&self) -> usize {
        self.queue
            .iter()
            .filter(|projection| !projection.is_terminal_preserved())
            .count()
    }

    fn drop_oldest_nonterminal(&mut self) -> bool {
        let Some(index) = self
            .queue
            .iter()
            .position(|projection| !projection.is_terminal_preserved())
        else {
            return false;
        };
        self.queue.remove(index);
        self.dropped_count += 1;
        true
    }

    fn export_failure_record(
        &mut self,
        projection: &TelemetryProjection,
        error: TelemetrySinkError,
    ) -> TelemetryRecord {
        self.failure_record(
            projection,
            error.failure_kind,
            projection.is_terminal_preserved(),
            projection.source_record.source_cursor.clone(),
            error.redacted_summary,
        )
    }

    fn failure_record(
        &mut self,
        projection: &TelemetryProjection,
        failure_kind: TelemetrySinkFailureKind,
        terminal_preserved: bool,
        repair_cursor: Option<TelemetrySourceCursor>,
        summary: impl Into<String>,
    ) -> TelemetryRecord {
        let sink_spec = self.sink.spec();
        let failure = TelemetrySinkFailureRecord {
            sink_id: sink_spec.sink_id.clone(),
            sink_kind: sink_spec.sink_kind.clone(),
            failure_kind,
            terminal_preserved,
            dropped_count: self.dropped_count,
            last_acknowledged_cursor: Some(self.cursor.clone()),
            repair_cursor,
            unsafe_pending_reason: (!sink_spec.requires_idempotent_replay)
                .then(|| "sink cannot prove idempotent repair replay".to_string()),
            redacted_summary: summary.into(),
        };
        TelemetryRecord::sink_failed(self.next_record_id("sink_failed"), projection, failure)
    }

    fn recovery_record(&mut self, projection: &TelemetryProjection) -> TelemetryRecord {
        let sink_spec = self.sink.spec();
        let recovery = TelemetrySinkRecoveryRecord {
            sink_id: sink_spec.sink_id.clone(),
            sink_kind: sink_spec.sink_kind.clone(),
            export_cursor: self.cursor.clone(),
            redacted_summary: "telemetry sink recovered after successful export".to_string(),
        };
        TelemetryRecord::sink_recovered(self.next_record_id("sink_recovered"), projection, recovery)
    }

    fn next_record_id(&mut self, label: &str) -> TelemetryRecordId {
        self.next_record_seq += 1;
        TelemetryRecordId::new(format!(
            "telemetry.{}.{}.{}",
            self.sink.spec().sink_id.as_str(),
            label,
            self.next_record_seq
        ))
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
/// Holds telemetry fanout report application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct TelemetryFanoutReport {
    /// Enqueued used by this record or request.
    pub enqueued: u64,
    /// Dropped used by this record or request.
    pub dropped: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Bounded records included in this record. Limits and truncation are
    /// represented by companion metadata when applicable.
    pub records: Vec<TelemetryRecord>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
/// Holds telemetry drain report application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct TelemetryDrainReport {
    /// Exported used by this record or request.
    pub exported: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Bounded records included in this record. Limits and truncation are
    /// represented by companion metadata when applicable.
    pub records: Vec<TelemetryRecord>,
}

/// Holds telemetry usage extraction input application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct TelemetryUsageExtractionInput {
    /// Event used by this record or request.
    pub event: AgentEvent,
    /// Cursor identifying a replay, export, or subscription position.
    /// Use it to resume without widening the original scope.
    pub event_cursor: Option<EventCursor>,
    /// Stable provider id used for typed lineage, lookup, or dedupe.
    pub provider_id: Option<String>,
    /// Stable model id used for typed lineage, lookup, or dedupe.
    pub model_id: Option<String>,
    /// Usage used by this record or request.
    pub usage: UsageUnits,
}

/// Holds telemetry usage extractor application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct TelemetryUsageExtractor;

impl TelemetryUsageExtractor {
    /// Returns extract from event derived from the supplied state.
    /// This uses only local coordinator state and performs no hidden host work.
    pub fn extract_from_event(
        input: TelemetryUsageExtractionInput,
    ) -> Result<TelemetryProjection, AgentError> {
        let envelope = input.event.envelope;
        if !matches!(
            envelope.event_family,
            EventFamily::Model | EventFamily::Run | EventFamily::Subagent
        ) {
            return Err(AgentError::new(
                AgentErrorKind::TelemetryFailure,
                RetryClassification::RepairNeeded,
                "usage telemetry must derive from model, run, or subagent facts",
            ));
        }

        Ok(TelemetryProjection {
            schema_version: TELEMETRY_SCHEMA_VERSION,
            projection_id: TelemetryProjectionId::new(format!(
                "telemetry.usage.{}",
                envelope.event_id.as_str()
            )),
            projection_kind: TelemetryProjectionKind::Usage,
            source_record: TelemetrySourceRecord {
                event_family: envelope.event_family.clone(),
                event_kind: envelope.event_kind.clone(),
                event_cursor: input.event_cursor.clone(),
                source_cursor: envelope
                    .journal_cursor
                    .clone()
                    .map(TelemetrySourceCursor::Journal)
                    .or_else(|| input.event_cursor.clone().map(TelemetrySourceCursor::Event)),
            },
            run_id: envelope.run_id,
            agent_id: envelope.agent_id,
            turn_id: envelope.turn_id,
            attempt_id: envelope.attempt_id,
            event_id: Some(envelope.event_id),
            journal_cursor: envelope.journal_cursor,
            trace_id: Some(envelope.trace_id),
            span_id: Some(envelope.span_id),
            runtime_package_fingerprint: envelope.runtime_package_fingerprint,
            source: envelope.source,
            destination: Some(DestinationRef::with_kind(
                DestinationKind::Telemetry,
                "destination.telemetry.usage",
            )),
            subject_ref: envelope.subject_ref,
            policy_refs: envelope.policy_refs,
            privacy: envelope.privacy,
            retention: RetentionClass::RunScoped,
            content_capture: TelemetryContentCaptureMode::Off,
            redaction_policy_id: envelope.redaction_policy_id,
            provider_id: input.provider_id,
            model_id: input.model_id,
            tool_name: None,
            usage: Some(input.usage),
            cost: None,
            terminal_status: Some(TelemetryTerminalStatus::Completed),
            sink_health: None,
            redacted_summary: "usage telemetry derived without raw prompt, tool, or model content"
                .to_string(),
            raw_content: None,
        })
    }

    /// Builds the usage record record for this contract.
    /// This builds a telemetry usage record from already redacted accounting data without
    /// exporting it.
    pub fn usage_record(
        projection: &TelemetryProjection,
        usage_record_id: impl Into<String>,
    ) -> TelemetryRecord {
        TelemetryRecord::usage(
            TelemetryRecordId::new(format!(
                "telemetry.record.{}",
                projection.projection_id.as_str()
            )),
            projection,
            TelemetryUsageRecordId::new(usage_record_id),
        )
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Holds telemetry content capture request application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct TelemetryContentCaptureRequest {
    /// Policy used by this record or request.
    pub policy: ContentCapturePolicy,
    /// Sink used by this record or request.
    pub sink: TelemetrySinkSpec,
    /// Requested mode used by this record or request.
    pub requested_mode: TelemetryContentCaptureMode,
    /// Whether source permits content is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub source_permits_content: bool,
    /// Retention class for referenced content or records.
    /// Stores and telemetry sinks use it to decide how long evidence may be kept.
    pub retention_active: bool,
    /// Whether deterministic telemetry sampling included this projection after policy gates.
    /// False means the projection should be dropped for sampled sinks even if retention is
    /// active.
    pub deterministic_sample_included: bool,
    /// requested bytes used for bounds checks, summaries, or truncation
    /// evidence.
    pub requested_bytes: u64,
    /// Stable redaction policy id used for typed lineage, lookup, or dedupe.
    pub redaction_policy_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Holds telemetry content capture decision application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct TelemetryContentCaptureDecision {
    /// Allowlist for this policy or contract.
    /// Validation uses it to reject undeclared or policy-denied values.
    pub allowed: bool,
    /// Requested mode used by this record or request.
    pub requested_mode: TelemetryContentCaptureMode,
    /// Effective mode used by this record or request.
    pub effective_mode: TelemetryContentCaptureMode,
    /// Redacted explanation for a denial, failure, status, or package delta.
    pub reason: String,
    /// Stable redaction policy id used for typed lineage, lookup, or dedupe.
    pub redaction_policy_id: String,
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
}

/// Evaluate content capture.
/// This evaluates policy for telemetry content capture and performs no capture or sink export
/// by itself.
pub fn evaluate_content_capture(
    request: &TelemetryContentCaptureRequest,
) -> TelemetryContentCaptureDecision {
    let policy_raw_mode = matches!(request.policy.mode, PolicyContentCaptureMode::RawContent);
    let sink_raw_mode = request.sink.content_capture.captures_raw_content();
    let byte_limit_allows =
        request.requested_bytes > 0 && request.requested_bytes <= request.policy.byte_limit;
    let all_raw_gates_pass = policy_raw_mode
        && request.policy.allows_raw_content()
        && request.source_permits_content
        && sink_raw_mode
        && request.retention_active
        && request.deterministic_sample_included
        && byte_limit_allows;

    if !request.requested_mode.captures_raw_content() {
        return TelemetryContentCaptureDecision {
            allowed: true,
            requested_mode: request.requested_mode.clone(),
            effective_mode: request.requested_mode.clone(),
            reason: "telemetry metadata or redacted capture does not request raw content"
                .to_string(),
            redaction_policy_id: request.redaction_policy_id.clone(),
            policy_refs: vec![request.policy.policy_ref.clone()],
        };
    }

    if all_raw_gates_pass {
        TelemetryContentCaptureDecision {
            allowed: true,
            requested_mode: request.requested_mode.clone(),
            effective_mode: TelemetryContentCaptureMode::RawContent,
            reason: "raw telemetry content capture allowed by source, sink, redaction, retention, sampling, and limits".to_string(),
            redaction_policy_id: request.redaction_policy_id.clone(),
            policy_refs: vec![request.policy.policy_ref.clone()],
        }
    } else {
        TelemetryContentCaptureDecision {
            allowed: false,
            requested_mode: request.requested_mode.clone(),
            effective_mode: TelemetryContentCaptureMode::RedactedSummary,
            reason: "raw telemetry content capture denied by source, sink, redaction, retention, sampling, or byte-limit policy".to_string(),
            redaction_policy_id: request.redaction_policy_id.clone(),
            policy_refs: vec![request.policy.policy_ref.clone()],
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Holds telemetry authority boundary application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct TelemetryAuthorityBoundary {
    /// Boolean policy/capability flag for whether can decide run state is
    /// enabled.
    pub can_decide_run_state: bool,
    /// Boolean policy/capability flag for whether can decide policy outcome
    /// is enabled.
    pub can_decide_policy_outcome: bool,
    /// Boolean policy/capability flag for whether can decide output delivery
    /// is enabled.
    pub can_decide_output_delivery: bool,
    /// Whether this telemetry surface is allowed to affect side-effect status.
    /// Observability-only telemetry should leave this false so telemetry cannot drive run
    /// control.
    pub can_decide_side_effect_status: bool,
}

/// Constant value for the application::telemetry contract. Use it to
/// keep SDK records and tests aligned on the same stable value.
pub const fn telemetry_authority_boundary() -> TelemetryAuthorityBoundary {
    TelemetryAuthorityBoundary {
        can_decide_run_state: false,
        can_decide_policy_outcome: false,
        can_decide_output_delivery: false,
        can_decide_side_effect_status: false,
    }
}

/// Returns terminal run projection from event derived from the supplied state.
/// This derives SDK state locally and does not call host adapters.
pub fn terminal_run_projection_from_event(event: AgentEvent) -> TelemetryProjection {
    let envelope = event.envelope;
    let terminal_status = match envelope.event_kind {
        EventKind::RunCompleted => TelemetryTerminalStatus::Completed,
        EventKind::RunCancelled => TelemetryTerminalStatus::Cancelled,
        EventKind::RunFailed => TelemetryTerminalStatus::Failed,
        _ => TelemetryTerminalStatus::Unknown,
    };
    let source_cursor = envelope
        .journal_cursor
        .clone()
        .map(TelemetrySourceCursor::Journal);
    TelemetryProjection {
        schema_version: TELEMETRY_SCHEMA_VERSION,
        projection_id: TelemetryProjectionId::new(format!(
            "telemetry.terminal.{}",
            envelope.event_id.as_str()
        )),
        projection_kind: TelemetryProjectionKind::RunTerminal,
        source_record: TelemetrySourceRecord {
            event_family: envelope.event_family.clone(),
            event_kind: envelope.event_kind.clone(),
            event_cursor: Some(envelope.cursor(EventStreamScope::Run(envelope.run_id.clone()))),
            source_cursor,
        },
        run_id: envelope.run_id,
        agent_id: envelope.agent_id,
        turn_id: envelope.turn_id,
        attempt_id: envelope.attempt_id,
        event_id: Some(envelope.event_id),
        journal_cursor: envelope.journal_cursor,
        trace_id: Some(envelope.trace_id),
        span_id: Some(envelope.span_id),
        runtime_package_fingerprint: envelope.runtime_package_fingerprint,
        source: envelope.source,
        destination: Some(DestinationRef::with_kind(
            DestinationKind::Telemetry,
            "destination.telemetry.terminal",
        )),
        subject_ref: envelope.subject_ref,
        policy_refs: envelope.policy_refs,
        privacy: envelope.privacy,
        retention: RetentionClass::RunScoped,
        content_capture: TelemetryContentCaptureMode::Off,
        redaction_policy_id: envelope.redaction_policy_id,
        provider_id: None,
        model_id: None,
        tool_name: None,
        usage: None,
        cost: None,
        terminal_status: Some(terminal_status),
        sink_health: None,
        redacted_summary: "terminal run telemetry derived from journal-backed event".to_string(),
        raw_content: None,
    }
}

fn apply_sink_content_boundary(
    projection: &TelemetryProjection,
    sink: &TelemetrySinkSpec,
) -> TelemetryProjection {
    if projection.raw_content.is_none() {
        return projection.clone();
    }
    if sink.content_capture.captures_raw_content()
        && projection.content_capture.captures_raw_content()
    {
        return projection.clone();
    }
    projection.clone().without_raw_content()
}

/// Returns sink health projection derived from the supplied state.
/// This derives SDK state locally and does not call host adapters.
pub fn sink_health_projection(
    base: &TelemetryProjection,
    sink_id: TelemetrySinkId,
    sink_kind: TelemetrySinkKind,
    state: TelemetrySinkHealthState,
) -> TelemetryProjection {
    let mut projection = base.clone().without_raw_content();
    projection.projection_id =
        TelemetryProjectionId::new(format!("telemetry.sink_health.{}", sink_id.as_str()));
    projection.projection_kind = TelemetryProjectionKind::SinkHealth;
    projection.sink_health = Some(TelemetrySinkHealth {
        sink_id,
        sink_kind,
        state,
        failure_kind: None,
        terminal_preserved: true,
        dropped_count: 0,
        export_cursor: None,
        unsafe_pending_reason: None,
    });
    projection
}
