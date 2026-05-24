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
pub struct TelemetryFanoutConfig {
    pub queue_capacity: NonZeroUsize,
    pub terminal_reserve: NonZeroUsize,
    pub overflow: TelemetryOverflowPolicy,
    pub sink_isolation: TelemetrySinkIsolationPolicy,
}

impl TelemetryFanoutConfig {
    pub fn safe_defaults() -> Self {
        Self {
            queue_capacity: NonZeroUsize::new(64).expect("nonzero queue capacity"),
            terminal_reserve: NonZeroUsize::new(4).expect("nonzero terminal reserve"),
            overflow: TelemetryOverflowPolicy::DropNonTerminalProgress,
            sink_isolation: TelemetrySinkIsolationPolicy::IsolateEachSink,
        }
    }

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
pub enum TelemetryOverflowPolicy {
    DropNonTerminalProgress,
    CoalesceProgressByRun,
    FailSinkNotRun,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TelemetrySinkIsolationPolicy {
    IsolateEachSink,
}

#[derive(Default)]
pub struct TelemetryFanout {
    config: TelemetryFanoutConfig,
    sinks: BTreeMap<TelemetrySinkId, TelemetrySinkState>,
}

impl TelemetryFanout {
    pub fn new(config: TelemetryFanoutConfig) -> Self {
        Self {
            config,
            sinks: BTreeMap::new(),
        }
    }

    pub fn safe_defaults() -> Self {
        Self::new(TelemetryFanoutConfig::safe_defaults())
    }

    pub fn register_sink(&mut self, sink: Arc<dyn TelemetrySink>) -> Result<(), AgentError> {
        let spec = sink.spec().clone();
        if spec.sink_id.as_str().is_empty() {
            return Err(AgentError::missing_required_field("telemetry.sink_id"));
        }
        self.sinks
            .insert(spec.sink_id.clone(), TelemetrySinkState::new(sink));
        Ok(())
    }

    pub fn sink_queue_len(&self, sink_id: &TelemetrySinkId) -> Option<usize> {
        self.sinks.get(sink_id).map(|state| state.queue.len())
    }

    pub fn queued_for_sink(&self, sink_id: &TelemetrySinkId) -> Vec<TelemetryProjection> {
        self.sinks
            .get(sink_id)
            .map(|state| state.queue.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn try_record(&mut self, projection: TelemetryProjection) -> TelemetryFanoutReport {
        let mut report = TelemetryFanoutReport::default();
        for state in self.sinks.values_mut() {
            let projection = apply_sink_content_boundary(&projection, state.sink.spec());
            state.enqueue(&self.config, projection, &mut report);
        }
        report
    }

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
pub struct TelemetryFanoutReport {
    pub enqueued: u64,
    pub dropped: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub records: Vec<TelemetryRecord>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct TelemetryDrainReport {
    pub exported: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub records: Vec<TelemetryRecord>,
}

pub struct TelemetryUsageExtractionInput {
    pub event: AgentEvent,
    pub event_cursor: Option<EventCursor>,
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
    pub usage: UsageUnits,
}

pub struct TelemetryUsageExtractor;

impl TelemetryUsageExtractor {
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
pub struct TelemetryContentCaptureRequest {
    pub policy: ContentCapturePolicy,
    pub sink: TelemetrySinkSpec,
    pub requested_mode: TelemetryContentCaptureMode,
    pub source_permits_content: bool,
    pub retention_active: bool,
    pub deterministic_sample_included: bool,
    pub requested_bytes: u64,
    pub redaction_policy_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TelemetryContentCaptureDecision {
    pub allowed: bool,
    pub requested_mode: TelemetryContentCaptureMode,
    pub effective_mode: TelemetryContentCaptureMode,
    pub reason: String,
    pub redaction_policy_id: String,
    pub policy_refs: Vec<PolicyRef>,
}

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
pub struct TelemetryAuthorityBoundary {
    pub can_decide_run_state: bool,
    pub can_decide_policy_outcome: bool,
    pub can_decide_output_delivery: bool,
    pub can_decide_side_effect_status: bool,
}

pub const fn telemetry_authority_boundary() -> TelemetryAuthorityBoundary {
    TelemetryAuthorityBoundary {
        can_decide_run_state: false,
        can_decide_policy_outcome: false,
        can_decide_output_delivery: false,
        can_decide_side_effect_status: false,
    }
}

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
