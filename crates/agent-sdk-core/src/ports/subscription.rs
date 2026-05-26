//! Host adapter boundaries for the SDK core. Use these traits and registries when
//! hosts provide providers, journals, sinks, tools, isolation, extensions, telemetry,
//! or subscriptions. Implementations may perform external side effects and must honor
//! policy, redaction, idempotency, and replay contracts. This file contains the
//! subscription portion of that contract.
//!
use std::sync::{Arc, Mutex};

use crate::{
    domain::{
        AgentError, AgentId, DestinationKind, DestinationRef, EntityRef, EventId, JournalCursor,
        RunId, SourceKind, SourceRef, SpanId, TraceId,
    },
    event::{
        AgentEvent, CompiledEventFilter, ContentCaptureMode, EVENT_SCHEMA_VERSION, EventCursor,
        EventDeliverySemantics, EventEnvelope, EventFamily, EventFilter, EventFrame, EventKind,
        EventStreamScope, cursor_compatible,
    },
    event_bus::AgentEventStream,
};

/// Port or behavior contract for run subscription source. Implementors
/// should preserve policy, redaction, idempotency, and replay
/// expectations from the surrounding module. Implementations may
/// perform side effects only as described by the trait methods.
pub trait RunSubscriptionSource: Send + Sync {
    /// Creates a read-only event stream for all visible events.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    fn subscribe_all(&self, cursor: Option<EventCursor>) -> Result<AgentEventStream, AgentError>;

    /// Creates a read-only event stream scoped to one run.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    fn subscribe_run(
        &self,
        run_id: RunId,
        cursor: Option<EventCursor>,
    ) -> Result<AgentEventStream, AgentError>;

    /// Creates a read-only event stream scoped to one agent.
    /// Implementations create a read-only subscription or replay stream; the call must not
    /// start runs, publish events, or append journal records.
    fn subscribe_agent(
        &self,
        agent_id: AgentId,
        cursor: Option<EventCursor>,
    ) -> Result<AgentEventStream, AgentError>;

    /// Creates a read-only event stream matching a compiled filter.
    /// Implementations create a read-only subscription or replay stream; the call must not
    /// start runs, publish events, or append journal records.
    fn subscribe_events(
        &self,
        filter: CompiledEventFilter,
        cursor: Option<EventCursor>,
    ) -> Result<AgentEventStream, AgentError>;

    /// Replays one run from durable cursor state.
    /// Implementations create a read-only subscription or replay stream; the call must not
    /// start runs, publish events, or append journal records.
    fn replay_run_from_cursor(
        &self,
        run_id: RunId,
        cursor: JournalCursor,
    ) -> Result<AgentEventStream, AgentError>;

    /// Reads the latest terminal event for one run, when available.
    /// Implementations read subscription state for the latest terminal frame; the lookup must
    /// not publish events or alter run state.
    fn latest_terminal_event(&self, run_id: &RunId) -> Result<Option<EventFrame>, AgentError>;
}

#[derive(Clone, Debug, Default)]
/// Carries in memory subscription hub data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct InMemorySubscriptionHub {
    frames: Arc<Mutex<Vec<EventFrame>>>,
    live_floor_seq: Arc<Mutex<u64>>,
}

impl InMemorySubscriptionHub {
    /// Mutates the in-memory event/subscription state and may wake local
    /// subscribers. It does not persist durable journal truth or call network
    /// sinks.
    pub fn publish(&self, frame: EventFrame) -> Result<(), AgentError> {
        self.frames
            .lock()
            .map_err(|_| AgentError::contract_violation("subscription hub lock poisoned"))?
            .push(frame);
        Ok(())
    }

    /// Mutates the in-memory event/subscription state and may wake local
    /// subscribers. It does not persist durable journal truth or call network
    /// sinks.
    pub fn publish_all(
        &self,
        frames: impl IntoIterator<Item = EventFrame>,
    ) -> Result<(), AgentError> {
        let mut locked = self
            .frames
            .lock()
            .map_err(|_| AgentError::contract_violation("subscription hub lock poisoned"))?;
        locked.extend(frames);
        Ok(())
    }

    /// Mutates the in-memory event/subscription state and may wake local
    /// subscribers. It does not persist durable journal truth or call network
    /// sinks.
    pub fn expire_live_before(&self, event_seq: u64) -> Result<(), AgentError> {
        *self
            .live_floor_seq
            .lock()
            .map_err(|_| AgentError::contract_violation("subscription hub lock poisoned"))? =
            event_seq;
        Ok(())
    }

    /// Builds the frames value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn frames(&self) -> Result<Vec<EventFrame>, AgentError> {
        Ok(self
            .frames
            .lock()
            .map_err(|_| AgentError::contract_violation("subscription hub lock poisoned"))?
            .clone())
    }

    fn subscribe_scope(
        &self,
        requested_scope: EventStreamScope,
        filter: CompiledEventFilter,
        cursor: Option<EventCursor>,
    ) -> Result<AgentEventStream, AgentError> {
        cursor_compatible(&requested_scope, cursor.as_ref())?;

        if let Some(cursor) = cursor.as_ref() {
            if self.cursor_expired(cursor)? {
                return self.resume_expired_cursor(requested_scope, filter, cursor.clone());
            }
        }

        let start_after = cursor.as_ref().map(|cursor| cursor.event_seq);
        let live_floor = self.live_floor()?;
        Ok(AgentEventStream::new(
            self.frames()?
                .into_iter()
                .filter(|frame| frame.event.envelope.event_seq >= live_floor)
                .filter(|frame| {
                    start_after.is_none_or(|event_seq| frame.event.envelope.event_seq > event_seq)
                })
                .filter(|frame| filter.matches_envelope(&frame.event.envelope))
                .map(|frame| frame_for_scope(frame, requested_scope.clone())),
        ))
    }

    fn resume_expired_cursor(
        &self,
        requested_scope: EventStreamScope,
        filter: CompiledEventFilter,
        cursor: EventCursor,
    ) -> Result<AgentEventStream, AgentError> {
        match (&requested_scope, cursor.journal_cursor.as_ref()) {
            (EventStreamScope::Run(run_id), Some(journal_cursor)) => {
                let live_floor = self.live_floor()?;
                let mut frames = self
                    .journal_backed_run_frames_after(run_id, journal_cursor)?
                    .into_iter()
                    .filter(|frame| frame.event.envelope.event_seq < live_floor)
                    .map(|frame| derived_replay_frame(frame, requested_scope.clone()))
                    .collect::<Vec<_>>();
                frames.extend(
                    self.frames()?
                        .into_iter()
                        .filter(|frame| frame.event.envelope.event_seq >= live_floor)
                        .filter(|frame| frame.event.envelope.event_seq > cursor.event_seq)
                        .filter(|frame| filter.matches_envelope(&frame.event.envelope))
                        .map(|frame| frame_for_scope(frame, requested_scope.clone())),
                );
                Ok(AgentEventStream::new(frames))
            }
            (EventStreamScope::Run(run_id), None) => {
                Ok(AgentEventStream::new([self.gap_diagnostic_frame(
                    run_id.clone(),
                    requested_scope,
                    cursor,
                )?]))
            }
            (_, Some(_)) => Err(AgentError::host_configuration_needed(
                "host archive required for expired non-run event cursor replay",
            )),
            (_, None) => Err(AgentError::host_configuration_needed(
                "expired non-run event cursor has no durable archive cursor",
            )),
        }
    }

    fn replay_run(
        &self,
        run_id: &RunId,
        cursor: &JournalCursor,
    ) -> Result<Vec<EventFrame>, AgentError> {
        self.journal_backed_run_frames_after(run_id, cursor)
            .map(|frames| {
                frames
                    .into_iter()
                    .map(|frame| derived_replay_frame(frame, EventStreamScope::Run(run_id.clone())))
                    .collect()
            })
    }

    fn journal_backed_run_frames_after(
        &self,
        run_id: &RunId,
        cursor: &JournalCursor,
    ) -> Result<Vec<EventFrame>, AgentError> {
        let cursor_seq = journal_cursor_seq(cursor);
        Ok(self
            .frames()?
            .into_iter()
            .filter(|frame| &frame.event.envelope.run_id == run_id)
            .filter(|frame| {
                frame
                    .event
                    .envelope
                    .journal_cursor
                    .as_ref()
                    .is_some_and(|journal_cursor| journal_cursor_seq(journal_cursor) > cursor_seq)
            })
            .collect())
    }

    fn cursor_expired(&self, cursor: &EventCursor) -> Result<bool, AgentError> {
        Ok(cursor.event_seq < self.live_floor()?)
    }

    fn live_floor(&self) -> Result<u64, AgentError> {
        Ok(*self
            .live_floor_seq
            .lock()
            .map_err(|_| AgentError::contract_violation("subscription hub lock poisoned"))?)
    }

    fn gap_diagnostic_frame(
        &self,
        run_id: RunId,
        requested_scope: EventStreamScope,
        cursor: EventCursor,
    ) -> Result<EventFrame, AgentError> {
        let agent_id = self
            .frames()?
            .into_iter()
            .rev()
            .find(|frame| frame.event.envelope.run_id == run_id)
            .map(|frame| frame.event.envelope.agent_id)
            .unwrap_or_else(|| AgentId::new("agent.replay.unknown"));
        let next_seq = cursor.event_seq.saturating_add(1);
        let event = AgentEvent::with_redacted_summary(
            EventEnvelope {
                schema_version: EVENT_SCHEMA_VERSION,
                event_id: EventId::new(format!(
                    "event.replay_failed.{}.{}",
                    run_id.as_str(),
                    next_seq
                )),
                event_seq: next_seq,
                event_family: EventFamily::Recovery,
                event_kind: EventKind::ReplayFailed,
                payload_schema_version: 1,
                timestamp: "1970-01-01T00:00:00Z".to_string(),
                recorded_at: "1970-01-01T00:00:00Z".to_string(),
                run_id: run_id.clone(),
                session_id: None,
                agent_id,
                turn_id: None,
                attempt_id: None,
                message_id: None,
                context_item_id: None,
                trace_id: TraceId::new(format!("trace.replay_failed.{}", run_id.as_str())),
                span_id: SpanId::new(format!("span.replay_failed.{next_seq}")),
                parent_event_id: Some(cursor.event_id),
                caused_by: None,
                subject_ref: EntityRef::run(run_id),
                related_refs: Vec::new(),
                causal_refs: Vec::new(),
                correlation: Default::default(),
                tags: Vec::new(),
                source: SourceRef::with_kind(SourceKind::Replay, "source.replay.subscription"),
                destination: Some(DestinationRef::with_kind(
                    DestinationKind::EventStream,
                    "destination.event_stream.subscription",
                )),
                policy_refs: Vec::new(),
                journal_cursor: None,
                state_before: None,
                state_after: None,
                delivery_semantics: EventDeliverySemantics::DiagnosticOnly,
                privacy: crate::domain::PrivacyClass::ContentRefsOnly,
                content_capture: ContentCaptureMode::Off,
                redaction_policy_id: "policy.redaction.default".to_string(),
                runtime_package_fingerprint: "runtime.package.fingerprint.subscription".to_string(),
            },
            "live cursor expired and no journal cursor was available for run replay",
        );
        Ok(frame_for_scope(
            EventFrame {
                cursor: event.envelope.cursor(requested_scope.clone()),
                event,
                archive_cursor: None,
                overflow: None,
            },
            requested_scope,
        ))
    }
}

impl RunSubscriptionSource for InMemorySubscriptionHub {
    fn subscribe_all(&self, cursor: Option<EventCursor>) -> Result<AgentEventStream, AgentError> {
        self.subscribe_scope(
            EventStreamScope::All,
            EventFilter::default().compile()?,
            cursor,
        )
    }

    fn subscribe_run(
        &self,
        run_id: RunId,
        cursor: Option<EventCursor>,
    ) -> Result<AgentEventStream, AgentError> {
        self.subscribe_scope(
            EventStreamScope::Run(run_id.clone()),
            EventFilter::run(run_id).compile()?,
            cursor,
        )
    }

    fn subscribe_agent(
        &self,
        agent_id: AgentId,
        cursor: Option<EventCursor>,
    ) -> Result<AgentEventStream, AgentError> {
        self.subscribe_scope(
            EventStreamScope::Agent(agent_id.clone()),
            EventFilter::agent(agent_id).compile()?,
            cursor,
        )
    }

    fn subscribe_events(
        &self,
        filter: CompiledEventFilter,
        cursor: Option<EventCursor>,
    ) -> Result<AgentEventStream, AgentError> {
        self.subscribe_scope(filter.cursor_scope(), filter, cursor)
    }

    fn replay_run_from_cursor(
        &self,
        run_id: RunId,
        cursor: JournalCursor,
    ) -> Result<AgentEventStream, AgentError> {
        Ok(AgentEventStream::new(self.replay_run(&run_id, &cursor)?))
    }

    fn latest_terminal_event(&self, run_id: &RunId) -> Result<Option<EventFrame>, AgentError> {
        Ok(self.frames()?.into_iter().rev().find(|frame| {
            &frame.event.envelope.run_id == run_id
                && matches!(
                    frame.event.envelope.event_kind,
                    EventKind::RunCompleted | EventKind::RunFailed | EventKind::RunCancelled
                )
        }))
    }
}

fn frame_for_scope(mut frame: EventFrame, scope: EventStreamScope) -> EventFrame {
    frame.cursor = frame.event.envelope.cursor(scope);
    frame
}

fn derived_replay_frame(mut frame: EventFrame, scope: EventStreamScope) -> EventFrame {
    frame.event.envelope.delivery_semantics = EventDeliverySemantics::DerivedReplay;
    frame.cursor = frame.event.envelope.cursor(scope);
    frame
}

fn journal_cursor_seq(cursor: &JournalCursor) -> u64 {
    cursor
        .as_str()
        .rsplit_once('.')
        .and_then(|(_, seq)| seq.parse::<u64>().ok())
        .unwrap_or(0)
}
