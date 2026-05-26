//! Derived trace views over durable journal records.
//! These helpers answer lineage questions without introducing a trace store,
//! payload parser, or host-owned conversation database.

use crate::{
    domain::{
        AttemptId, ContextProjectionId, EffectId, MessageId, RunId, SessionId, ToolCallId, TurnId,
    },
    journal::{EventIndexProjection, JournalRecord, JournalRecordPayload},
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
/// Derived evidence for one turn. Build this from journal records when a host
/// needs to inspect the actions causally associated with a user message.
pub struct TurnTrace {
    /// Optional host-provided session identifier for grouping related turns.
    pub session_id: Option<SessionId>,
    /// Turn identifier for one loop turn within a run.
    pub turn_id: Option<TurnId>,
    /// Run identifiers causally associated with this trace.
    pub run_ids: Vec<RunId>,
    /// Attempt identifiers observed in this trace.
    pub attempt_ids: Vec<AttemptId>,
    /// Message identifiers observed in this trace.
    pub message_ids: Vec<MessageId>,
    /// Context projection identifiers observed in this trace.
    pub context_projection_ids: Vec<ContextProjectionId>,
    /// Effect identifiers observed in this trace.
    pub effect_ids: Vec<EffectId>,
    /// Tool call identifiers observed in this trace.
    pub tool_call_ids: Vec<ToolCallId>,
    /// Event-index projections stored alongside matching journal records.
    pub event_indexes: Vec<EventIndexProjection>,
    /// Matching durable journal records in journal order.
    pub records: Vec<JournalRecord>,
}

impl TurnTrace {
    /// Builds a trace for one turn id from durable journal records.
    /// Matching is envelope based and does not inspect raw content.
    pub fn from_records<'a>(
        turn_id: &TurnId,
        records: impl IntoIterator<Item = &'a JournalRecord>,
    ) -> Self {
        Self::from_matching_records(records, |record| record.turn_id.as_ref() == Some(turn_id))
    }

    fn from_matching_records<'a>(
        records: impl IntoIterator<Item = &'a JournalRecord>,
        matches: impl Fn(&JournalRecord) -> bool,
    ) -> Self {
        let mut trace = TurnTrace::default();
        for record in records.into_iter().filter(|record| matches(record)) {
            trace.push_record(record);
        }
        trace
    }

    fn push_record(&mut self, record: &JournalRecord) {
        if self.session_id.is_none() {
            self.session_id = record.session_id.clone();
        }
        if self.turn_id.is_none() {
            self.turn_id = record.turn_id.clone();
        }
        push_unique(&mut self.run_ids, record.run_id.clone());
        if let Some(attempt_id) = record.attempt_id.clone() {
            push_unique(&mut self.attempt_ids, attempt_id);
        }
        self.push_payload_ids(&record.payload);
        self.event_indexes.push(record.event_index.clone());
        self.records.push(record.clone());
    }

    fn push_payload_ids(&mut self, payload: &JournalRecordPayload) {
        match payload {
            JournalRecordPayload::TurnLifecycle(record) => {
                push_unique_opt(&mut self.message_ids, record.input_message_id.clone());
                push_unique_opt(&mut self.message_ids, record.output_message_id.clone());
                push_unique_opt(
                    &mut self.context_projection_ids,
                    record.context_projection_id.clone(),
                );
                for run_id in &record.run_ids {
                    push_unique(&mut self.run_ids, run_id.clone());
                }
            }
            JournalRecordPayload::ContextProjection(record) => {
                push_unique(
                    &mut self.context_projection_ids,
                    record.projection_id.clone(),
                );
            }
            JournalRecordPayload::Message(record) => {
                push_unique(&mut self.message_ids, record.message_id.clone());
            }
            JournalRecordPayload::EffectIntent(record) => {
                push_unique(&mut self.effect_ids, record.effect_id.clone());
            }
            JournalRecordPayload::EffectResult(record) => {
                push_unique(&mut self.effect_ids, record.effect_id.clone());
            }
            JournalRecordPayload::Tool(record) => {
                push_unique(&mut self.tool_call_ids, record.tool_call_id.clone());
            }
            _ => {}
        }
    }

    /// Returns true when no journal records matched the requested trace.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Derived evidence for one run. This is a convenience view over the same
/// journal records used by `TurnTrace`.
pub struct RunTrace {
    /// Run identifier used for lineage, filtering, replay, and dedupe.
    pub run_id: Option<RunId>,
    /// Optional host-provided session identifier for grouping related turns.
    pub session_id: Option<SessionId>,
    /// Turn traces observed for this run, in first-seen journal order.
    pub turn_traces: Vec<TurnTrace>,
    /// Matching durable journal records in journal order.
    pub records: Vec<JournalRecord>,
}

impl RunTrace {
    /// Builds a run trace from durable journal records.
    pub fn from_records<'a>(
        run_id: &RunId,
        records: impl IntoIterator<Item = &'a JournalRecord>,
    ) -> Self {
        let matching = records
            .into_iter()
            .filter(|record| &record.run_id == run_id)
            .cloned()
            .collect::<Vec<_>>();
        let mut trace = RunTrace {
            run_id: Some(run_id.clone()),
            session_id: matching.iter().find_map(|record| record.session_id.clone()),
            turn_traces: Vec::new(),
            records: matching.clone(),
        };
        for turn_id in ordered_turn_ids(&matching) {
            trace
                .turn_traces
                .push(TurnTrace::from_records(&turn_id, matching.iter()));
        }
        trace
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Derived session timeline grouped by turn. Hosts may persist conversation
/// stores separately; this view is only a journal-derived trace index.
pub struct SessionTimeline {
    /// Optional host-provided session identifier for grouping related turns.
    pub session_id: SessionId,
    /// Turn traces observed for this session, in first-seen journal order.
    pub turns: Vec<TurnTrace>,
}

impl SessionTimeline {
    /// Builds a session timeline from durable journal records.
    pub fn from_records<'a>(
        session_id: &SessionId,
        records: impl IntoIterator<Item = &'a JournalRecord>,
    ) -> Self {
        let matching = records
            .into_iter()
            .filter(|record| record.session_id.as_ref() == Some(session_id))
            .cloned()
            .collect::<Vec<_>>();
        let turns = ordered_turn_ids(&matching)
            .into_iter()
            .map(|turn_id| TurnTrace::from_records(&turn_id, matching.iter()))
            .collect();
        Self {
            session_id: session_id.clone(),
            turns,
        }
    }
}

fn ordered_turn_ids(records: &[JournalRecord]) -> Vec<TurnId> {
    let mut turn_ids = Vec::new();
    for record in records {
        if let Some(turn_id) = record.turn_id.clone() {
            push_unique(&mut turn_ids, turn_id);
        }
    }
    turn_ids
}

fn push_unique<T: Eq>(items: &mut Vec<T>, value: T) {
    if !items.contains(&value) {
        items.push(value);
    }
}

fn push_unique_opt<T: Eq>(items: &mut Vec<T>, value: Option<T>) {
    if let Some(value) = value {
        push_unique(items, value);
    }
}
