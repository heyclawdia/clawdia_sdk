//! Application-layer coordination over core primitives. Use these services to lower
//! helpers, drive runs, validate output, coordinate tools, approvals, delivery,
//! isolation, telemetry, and feature layers. Methods in this layer may call
//! configured ports, mutate in-memory stores, append journals, or publish events as
//! documented. This file contains the realtime portion of that contract.
//!
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::{
    domain::{
        AgentError, AgentErrorKind, AgentId, DestinationKind, DestinationRef, EntityKind,
        EntityRef, RetryClassification, RunId, SourceRef,
    },
    journal::{
        JournalCursor, JournalRecord, JournalRecordBase, JournalRecordKind, JournalRecordPayload,
    },
    journal_ports::RunJournal,
    ports::realtime::{RealtimeConnectRequest, RealtimeProviderAdapter},
    realtime_records::{
        RealtimeBackpressureState, RealtimeCloseReason, RealtimeConnectionId, RealtimeInputFrame,
        RealtimeMediaKind, RealtimeResponseId, RealtimeSessionId, RealtimeSessionRecord,
        RealtimeSessionRecordKind, RealtimeSessionState, RealtimeSessionStatus,
    },
    stream_records::{
        StreamChannel, StreamCursor, StreamCursorPrecision, StreamDirection, safe_id_fragment,
    },
};

#[derive(Clone)]
/// Holds realtime session controller application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct RealtimeSessionController {
    sidecar: crate::package::realtime::RealtimeSessionSidecar,
    adapter: Arc<dyn RealtimeProviderAdapter>,
    journal: Arc<dyn RunJournal>,
    run_id: RunId,
    agent_id: AgentId,
    source: SourceRef,
    runtime_package_fingerprint: String,
    next_journal_seq: u64,
    state: Option<RealtimeSessionState>,
}

impl RealtimeSessionController {
    /// Creates a new application::realtime value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new(
        sidecar: crate::package::realtime::RealtimeSessionSidecar,
        adapter: Arc<dyn RealtimeProviderAdapter>,
        journal: Arc<dyn RunJournal>,
        run_id: RunId,
        agent_id: AgentId,
        source: SourceRef,
        runtime_package_fingerprint: impl Into<String>,
    ) -> Self {
        Self {
            sidecar,
            adapter,
            journal,
            run_id,
            agent_id,
            source,
            runtime_package_fingerprint: runtime_package_fingerprint.into(),
            next_journal_seq: 1,
            state: None,
        }
    }

    /// Connect.
    /// This appends realtime connection intent/result records through the journal path and
    /// calls the configured realtime adapter to open the session.
    pub fn connect(&mut self) -> Result<RealtimeSessionRecord, AgentError> {
        self.sidecar.validate()?;
        let session_id = RealtimeSessionId::new(format!(
            "realtime.session.{}",
            safe_id_fragment(self.run_id.as_str())
        ));
        let backpressure_state = RealtimeBackpressureState::bounded(
            self.sidecar.queue_capacity,
            self.sidecar.backpressure_policy_ref.clone(),
        );
        let requested_state = RealtimeSessionState {
            session_id: session_id.clone(),
            connection_id: RealtimeConnectionId::new(format!(
                "realtime.connection.pending.{}",
                safe_id_fragment(self.run_id.as_str())
            )),
            provider_route_ref: self.sidecar.provider_route_ref.clone(),
            send_cursor: StreamCursor::chunk(0),
            receive_cursor: StreamCursor::chunk(0),
            restart_count: 0,
            backpressure_state: backpressure_state.clone(),
            lifecycle_status: RealtimeSessionStatus::Connecting,
            policy_refs: self.sidecar.policy_refs(),
        };
        let requested = self.record(
            &requested_state,
            RealtimeSessionRecordKind::ConnectRequested,
            "realtime connect requested before adapter call",
        );
        self.append_realtime_record(requested)?;

        let response = self.adapter.connect(RealtimeConnectRequest {
            session_id: session_id.clone(),
            provider_route_ref: self.sidecar.provider_route_ref.clone(),
            realtime_capability_ref: self.sidecar.realtime_capability_ref.clone(),
        })?;
        let state = RealtimeSessionState {
            session_id,
            connection_id: response.connection_id,
            provider_route_ref: self.sidecar.provider_route_ref.clone(),
            send_cursor: StreamCursor::chunk(0),
            receive_cursor: StreamCursor::chunk(0),
            restart_count: 0,
            backpressure_state,
            lifecycle_status: RealtimeSessionStatus::Connected,
            policy_refs: self.sidecar.policy_refs(),
        };
        let record = self.record(
            &state,
            RealtimeSessionRecordKind::Connected,
            "realtime session connected",
        );
        self.append_realtime_record(record.clone())?;
        self.state = Some(state);
        Ok(record)
    }

    /// Send.
    /// This journals the realtime send path and forwards one frame to the configured realtime
    /// adapter.
    pub fn send(&mut self, frame: RealtimeInputFrame) -> Result<RealtimeSessionRecord, AgentError> {
        if self
            .state
            .as_ref()
            .is_some_and(|state| state.lifecycle_status == RealtimeSessionStatus::RestartStarted)
        {
            return self.apply_backpressure(frame);
        }

        let state = self.connected_state()?.clone();
        let mut requested = self.record(
            &state,
            RealtimeSessionRecordKind::InputSendRequested,
            frame.redacted_summary.clone(),
        );
        requested.channel = StreamChannel::RealtimeMedia;
        requested.direction = Some(StreamDirection::InputToProvider);
        requested.media_kind = frame.media_kind;
        requested.content_refs = frame.content_refs.clone();
        requested.privacy = frame.privacy.clone();
        requested.retention = frame.retention.clone();
        self.append_realtime_record(requested)?;

        self.adapter.send(&state.session_id, frame.clone())?;
        let mut state = state;
        state.send_cursor = StreamCursor {
            chunk_sequence: state.send_cursor.chunk_sequence + 1,
            byte_offset: 0,
            precision: StreamCursorPrecision::ChunkSequenceOnly,
            label: Some("send".to_string()),
        };
        state.lifecycle_status = RealtimeSessionStatus::InputSent;
        let mut record = self.record(
            &state,
            RealtimeSessionRecordKind::InputSent,
            frame.redacted_summary.clone(),
        );
        record.channel = StreamChannel::RealtimeMedia;
        record.direction = Some(StreamDirection::InputToProvider);
        record.media_kind = frame.media_kind;
        record.content_refs = frame.content_refs;
        record.privacy = frame.privacy;
        record.retention = frame.retention;
        self.append_realtime_record(record.clone())?;
        self.state = Some(state);
        Ok(record)
    }

    /// Receives one realtime output frame through the configured adapter.
    /// This records a receive request, calls the adapter, appends a received record when output is
    /// available, and updates the in-memory session cursor.
    pub fn receive(&mut self) -> Result<Option<RealtimeSessionRecord>, AgentError> {
        let state = self.connected_state()?.clone();
        let requested = self.record(
            &state,
            RealtimeSessionRecordKind::OutputReceiveRequested,
            "realtime receive requested before adapter call",
        );
        self.append_realtime_record(requested)?;
        let Some(frame) = self.adapter.receive(&state.session_id)? else {
            return Ok(None);
        };
        let mut state = state;
        state.receive_cursor = StreamCursor {
            chunk_sequence: state.receive_cursor.chunk_sequence + 1,
            byte_offset: 0,
            precision: StreamCursorPrecision::ChunkSequenceOnly,
            label: Some("receive".to_string()),
        };
        state.lifecycle_status = RealtimeSessionStatus::OutputReceived;
        let mut record = self.record(
            &state,
            RealtimeSessionRecordKind::OutputReceived,
            frame.redacted_summary,
        );
        record.channel = StreamChannel::RealtimeTranscript;
        record.direction = Some(StreamDirection::OutputFromProvider);
        record.media_kind = frame.media_kind;
        record.response_id = Some(frame.response_id);
        record.content_refs = frame.content_refs;
        record.privacy = frame.privacy;
        record.retention = frame.retention;
        self.append_realtime_record(record.clone())?;
        self.state = Some(state);
        Ok(Some(record))
    }

    /// Interrupt.
    /// This records the interrupt path and sends the configured realtime interruption frame to
    /// the adapter session.
    pub fn interrupt(
        &mut self,
        response_id: impl Into<String>,
    ) -> Result<RealtimeSessionRecord, AgentError> {
        let response_id = RealtimeResponseId::new(response_id);
        let state = self.connected_state()?.clone();
        let mut requested = self.record(
            &state,
            RealtimeSessionRecordKind::InterruptRequested,
            "realtime interrupt requested before adapter call",
        );
        requested.response_id = Some(response_id.clone());
        self.append_realtime_record(requested)?;
        let mut record = self.record(
            &state,
            RealtimeSessionRecordKind::Interrupted,
            "realtime interruption acknowledged by adapter",
        );
        record.status = RealtimeSessionStatus::Interrupted;
        record.response_id = Some(response_id.clone());
        self.adapter.interrupt(&state.session_id, &response_id)?;
        let mut next = state.clone();
        next.lifecycle_status = RealtimeSessionStatus::Interrupted;
        self.append_realtime_record(record.clone())?;
        self.state = Some(next);
        Ok(record)
    }

    /// Marks the active realtime session as beginning a restart.
    /// This appends restart-requested and restart-started records and updates session state; the
    /// adapter restart call happens in `complete_restart`.
    pub fn begin_restart(&mut self) -> Result<Vec<RealtimeSessionRecord>, AgentError> {
        let state = self.connected_state()?.clone();
        let mut requested = self.record(
            &state,
            RealtimeSessionRecordKind::RestartRequested,
            "realtime restart requested",
        );
        requested.status = RealtimeSessionStatus::RestartRequested;

        let mut started_state = state;
        started_state.lifecycle_status = RealtimeSessionStatus::RestartStarted;
        let mut started = self.record(
            &started_state,
            RealtimeSessionRecordKind::RestartStarted,
            "realtime restart started; outbound frames gated",
        );
        started.status = RealtimeSessionStatus::RestartStarted;
        self.append_realtime_record(requested.clone())?;
        self.append_realtime_record(started.clone())?;
        self.state = Some(started_state);
        Ok(vec![requested, started])
    }

    /// Complete restart.
    /// This records restart completion and updates session state after the adapter reports
    /// success.
    pub fn complete_restart(&mut self) -> Result<Vec<RealtimeSessionRecord>, AgentError> {
        let state = self.connected_state()?.clone();
        let response = match self
            .adapter
            .restart(&state.session_id, &state.connection_id)
        {
            Ok(response) => response,
            Err(error) => {
                let mut failed_state = state.clone();
                failed_state.lifecycle_status = RealtimeSessionStatus::RestartFailed;
                let mut failed = self.record(
                    &failed_state,
                    RealtimeSessionRecordKind::RestartFailed,
                    error.context().message,
                );
                failed.status = RealtimeSessionStatus::RestartFailed;
                self.append_realtime_record(failed.clone())?;
                self.state = Some(failed_state);
                return Ok(vec![failed]);
            }
        };

        let mut completed_state = state;
        completed_state.connection_id = response.connection_id;
        completed_state.restart_count += 1;
        completed_state.lifecycle_status = RealtimeSessionStatus::RestartCompleted;
        let mut completed = self.record(
            &completed_state,
            RealtimeSessionRecordKind::RestartCompleted,
            "realtime restart completed",
        );
        completed.status = RealtimeSessionStatus::RestartCompleted;
        self.append_realtime_record(completed.clone())?;
        self.state = Some(completed_state);
        Ok(vec![completed])
    }

    /// Close.
    /// This journals close intent/result and calls the realtime adapter to close the active
    /// session.
    pub fn close(
        &mut self,
        reason: RealtimeCloseReason,
    ) -> Result<RealtimeSessionRecord, AgentError> {
        let state = self.connected_state()?.clone();
        let mut requested = self.record(
            &state,
            RealtimeSessionRecordKind::CloseRequested,
            "realtime close requested before adapter call",
        );
        requested.close_reason = Some(reason);
        self.append_realtime_record(requested)?;
        self.adapter.close(&state.session_id, reason)?;
        let mut closed_state = state;
        closed_state.lifecycle_status = RealtimeSessionStatus::Closed;
        let mut record = self.record(
            &closed_state,
            RealtimeSessionRecordKind::Closed,
            "realtime session closed",
        );
        record.status = RealtimeSessionStatus::Closed;
        record.close_reason = Some(reason);
        self.append_realtime_record(record.clone())?;
        self.state = Some(closed_state);
        Ok(record)
    }

    fn apply_backpressure(
        &mut self,
        frame: RealtimeInputFrame,
    ) -> Result<RealtimeSessionRecord, AgentError> {
        let state = self.connected_state()?.clone();
        let mut gated_state = state;
        gated_state.backpressure_state = gated_state.backpressure_state.clone().gate();
        gated_state.lifecycle_status = RealtimeSessionStatus::BackpressureApplied;
        let mut record = self.record(
            &gated_state,
            RealtimeSessionRecordKind::BackpressureApplied,
            "outbound realtime frame gated during restart",
        );
        record.channel = StreamChannel::RealtimeMedia;
        record.direction = Some(StreamDirection::InputToProvider);
        record.media_kind = frame.media_kind;
        record.content_refs = frame.content_refs;
        record.privacy = frame.privacy;
        record.retention = frame.retention;
        self.append_realtime_record(record.clone())?;
        self.state = Some(gated_state);
        Ok(record)
    }

    fn connected_state(&self) -> Result<&RealtimeSessionState, AgentError> {
        self.state.as_ref().ok_or_else(|| {
            AgentError::contract_violation("realtime session must connect before use")
        })
    }

    fn record(
        &self,
        state: &RealtimeSessionState,
        kind: RealtimeSessionRecordKind,
        redacted_summary: impl Into<String>,
    ) -> RealtimeSessionRecord {
        let _ = (&self.source, &self.runtime_package_fingerprint);
        RealtimeSessionRecord {
            kind,
            session_id: state.session_id.clone(),
            connection_id: Some(state.connection_id.clone()),
            response_id: None,
            run_id: self.run_id.clone(),
            agent_id: self.agent_id.clone(),
            provider_route_ref: state.provider_route_ref.clone(),
            send_cursor: state.send_cursor.clone(),
            receive_cursor: state.receive_cursor.clone(),
            restart_count: state.restart_count,
            backpressure_state: state.backpressure_state.clone(),
            status: state.lifecycle_status,
            close_reason: None,
            channel: StreamChannel::RealtimeTranscript,
            direction: None,
            media_kind: RealtimeMediaKind::Transcript,
            content_refs: Vec::new(),
            policy_refs: state.policy_refs.clone(),
            privacy: crate::domain::PrivacyClass::ContentRefsOnly,
            retention: crate::domain::RetentionClass::RunScoped,
            redacted_summary: redacted_summary.into(),
            effect_intent_ref: None,
            effect_result_ref: None,
            effect_intent: None,
            effect_result: None,
        }
    }

    fn append_realtime_record(
        &mut self,
        record: RealtimeSessionRecord,
    ) -> Result<JournalCursor, AgentError> {
        let mut base = JournalRecordBase::new(
            self.next_journal_seq,
            format!(
                "journal.record.{}",
                record.event_kind_name().replace('_', ".")
            ),
            self.run_id.clone(),
            self.agent_id.clone(),
            self.source.clone(),
        );
        self.next_journal_seq += 1;
        base.destination = Some(DestinationRef::with_kind(
            DestinationKind::Provider,
            record.provider_route_ref.clone(),
        ));
        base.runtime_package_fingerprint = self.runtime_package_fingerprint.clone();
        base.privacy = record.privacy.clone();
        base.redaction_policy_id = record
            .policy_refs
            .first()
            .map(|policy| policy.as_str().to_string())
            .unwrap_or_else(|| "policy.redaction.realtime.default".to_string());
        base.tags = vec!["feature:realtime".to_string()];
        let subject_ref = EntityRef::new(EntityKind::RealtimeSession, record.session_id.as_str());
        self.journal
            .append(JournalRecord::feature_record(
                base,
                JournalRecordKind::RealtimeSession,
                "realtime",
                record.event_kind_name(),
                subject_ref,
                Vec::new(),
                record.content_refs.clone(),
                JournalRecordPayload::RealtimeSession(record),
            ))
            .map_err(journal_failure)
    }
}

fn journal_failure(error: AgentError) -> AgentError {
    AgentError::new(
        AgentErrorKind::JournalFailure,
        RetryClassification::RepairNeeded,
        error.context().message,
    )
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
/// Holds realtime completion gate application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct RealtimeCompletionGate {
    /// Whether final visible output seen is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub final_visible_output_seen: bool,
    /// Whether terminal event replayable is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub terminal_event_replayable: bool,
    /// Whether stream-intervention processing has reached its terminal completion gate.
    /// Run completion should wait for this when stream rules can mask, abort, or retry output.
    pub stream_interventions_terminal: bool,
    /// Whether realtime sessions terminal is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub realtime_sessions_terminal: bool,
    /// Output delivery setting or policy.
    /// Delivery coordinators use it to decide sink mode, dedupe, and required evidence.
    pub output_delivery_terminal: bool,
    /// Whether approvals terminal is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub approvals_terminal: bool,
    /// Whether journal terminal is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub journal_terminal: bool,
}

impl RealtimeCompletionGate {
    /// Mark final visible output.
    /// This marks the in-memory completion gate for final visible output and does not publish
    /// events.
    pub fn mark_final_visible_output(&mut self) {
        self.final_visible_output_seen = true;
    }

    /// Mark terminal event replayable.
    /// This flips the in-memory completion gate for replayable terminal events.
    pub fn mark_terminal_event_replayable(&mut self) {
        self.terminal_event_replayable = true;
    }

    /// Mark stream interventions terminal.
    /// This operates on realtime session or completion-gate state only.
    pub fn mark_stream_interventions_terminal(&mut self) {
        self.stream_interventions_terminal = true;
    }

    /// Mark realtime sessions terminal.
    /// This flips the in-memory completion gate for terminal realtime sessions.
    pub fn mark_realtime_sessions_terminal(&mut self) {
        self.realtime_sessions_terminal = true;
    }

    /// Mark output delivery terminal.
    /// This flips the in-memory completion gate for terminal output delivery.
    pub fn mark_output_delivery_terminal(&mut self) {
        self.output_delivery_terminal = true;
    }

    /// Mark approvals terminal.
    /// This marks the in-memory completion gate for terminal approvals and does not publish
    /// events.
    pub fn mark_approvals_terminal(&mut self) {
        self.approvals_terminal = true;
    }

    /// Mark journal terminal.
    /// This marks the in-memory completion gate for terminal journal state and does not append
    /// a record.
    pub fn mark_journal_terminal(&mut self) {
        self.journal_terminal = true;
    }

    /// Returns whether can complete run applies for this contract.
    /// This reads the realtime completion gates and does not mutate state.
    pub fn can_complete_run(&self) -> bool {
        self.final_visible_output_seen
            && self.terminal_event_replayable
            && self.stream_interventions_terminal
            && self.realtime_sessions_terminal
            && self.output_delivery_terminal
            && self.approvals_terminal
            && self.journal_terminal
    }
}
