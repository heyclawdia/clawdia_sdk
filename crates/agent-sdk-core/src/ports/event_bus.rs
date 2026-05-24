use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use crate::{
    domain::{AgentError, AgentId, RunId},
    event::{
        AgentEvent, ArchiveCursor, CompiledEventFilter, EventCursor, EventFilter, EventFrame,
        EventKind, EventOverflowNotice, EventOverflowReason, EventStreamScope,
        SubscriberOverflowPolicy, SubscriberQueueConfig, SubscriptionOptions, cursor_compatible,
    },
};

pub trait AgentEventBus: Send + Sync {
    fn publish(&self, frame: EventFrame) -> Result<(), AgentError>;

    fn subscribe_all(&self, cursor: Option<EventCursor>) -> Result<AgentEventStream, AgentError>;

    fn subscribe_all_with_options(
        &self,
        cursor: Option<EventCursor>,
        options: SubscriptionOptions,
    ) -> Result<AgentEventStream, AgentError>;

    fn subscribe_run(
        &self,
        run_id: RunId,
        cursor: Option<EventCursor>,
    ) -> Result<AgentEventStream, AgentError>;

    fn subscribe_run_with_options(
        &self,
        run_id: RunId,
        cursor: Option<EventCursor>,
        options: SubscriptionOptions,
    ) -> Result<AgentEventStream, AgentError>;

    fn subscribe_agent(
        &self,
        agent_id: AgentId,
        cursor: Option<EventCursor>,
    ) -> Result<AgentEventStream, AgentError>;

    fn subscribe_agent_with_options(
        &self,
        agent_id: AgentId,
        cursor: Option<EventCursor>,
        options: SubscriptionOptions,
    ) -> Result<AgentEventStream, AgentError>;

    fn subscribe_filtered(
        &self,
        filter: CompiledEventFilter,
        cursor: Option<EventCursor>,
    ) -> Result<AgentEventStream, AgentError>;
}

pub trait EventArchive: Send + Sync {
    fn replay_filtered_from_cursor(
        &self,
        filter: CompiledEventFilter,
        cursor: ArchiveCursor,
    ) -> Result<AgentEventStream, AgentError>;
}

#[derive(Clone, Debug)]
pub struct AgentEventStream {
    frames: VecDeque<EventFrame>,
}

impl AgentEventStream {
    pub fn new(frames: impl IntoIterator<Item = EventFrame>) -> Self {
        Self {
            frames: frames.into_iter().collect(),
        }
    }
}

impl Iterator for AgentEventStream {
    type Item = EventFrame;

    fn next(&mut self) -> Option<Self::Item> {
        self.frames.pop_front()
    }
}

#[derive(Clone, Debug, Default)]
pub struct InMemoryAgentEventBus {
    frames: Arc<Mutex<Vec<EventFrame>>>,
    next_event_seq: Arc<AtomicU64>,
}

impl InMemoryAgentEventBus {
    pub fn publish(&self, frame: EventFrame) -> Result<(), AgentError> {
        let frame = self.assign_live_sequence(frame);
        self.frames
            .lock()
            .map_err(|_| AgentError::contract_violation("event bus lock poisoned"))?
            .push(frame);
        Ok(())
    }

    pub fn publish_all(
        &self,
        frames: impl IntoIterator<Item = EventFrame>,
    ) -> Result<(), AgentError> {
        let frames = frames
            .into_iter()
            .map(|frame| self.assign_live_sequence(frame))
            .collect::<Vec<_>>();
        let mut locked = self
            .frames
            .lock()
            .map_err(|_| AgentError::contract_violation("event bus lock poisoned"))?;
        locked.extend(frames);
        Ok(())
    }

    fn filtered_stream(
        &self,
        requested_scope: EventStreamScope,
        filter: CompiledEventFilter,
        cursor: Option<EventCursor>,
        queue: SubscriberQueueConfig,
    ) -> Result<AgentEventStream, AgentError> {
        cursor_compatible(&requested_scope, cursor.as_ref())?;
        reject_live_overflow_policy(&queue)?;
        let start_after = cursor.as_ref().map(|cursor| cursor.event_seq);
        let frames = self
            .frames
            .lock()
            .map_err(|_| AgentError::contract_violation("event bus lock poisoned"))?
            .iter()
            .filter(|frame| start_after.is_none_or(|seq| frame.cursor.event_seq > seq))
            .filter(|frame| filter.matches_envelope(&frame.event.envelope))
            .map(|frame| {
                let mut frame = frame.clone();
                frame.cursor = frame.event.envelope.cursor(requested_scope.clone());
                frame
            })
            .collect::<Vec<_>>();
        let frames = apply_queue_bounds(frames, &queue);
        Ok(AgentEventStream::new(frames))
    }

    fn assign_live_sequence(&self, mut frame: EventFrame) -> EventFrame {
        let event_seq = self.next_event_seq.fetch_add(1, Ordering::SeqCst) + 1;
        frame.event.envelope.event_seq = event_seq;
        frame.cursor = frame.event.envelope.cursor(frame.cursor.scope.clone());
        frame
    }
}

impl AgentEventBus for InMemoryAgentEventBus {
    fn publish(&self, frame: EventFrame) -> Result<(), AgentError> {
        InMemoryAgentEventBus::publish(self, frame)
    }

    fn subscribe_all(&self, cursor: Option<EventCursor>) -> Result<AgentEventStream, AgentError> {
        self.subscribe_all_with_options(cursor, SubscriptionOptions::default())
    }

    fn subscribe_all_with_options(
        &self,
        cursor: Option<EventCursor>,
        options: SubscriptionOptions,
    ) -> Result<AgentEventStream, AgentError> {
        self.filtered_stream(
            EventStreamScope::All,
            EventFilter::default().compile()?,
            cursor,
            options.queue,
        )
    }

    fn subscribe_run(
        &self,
        run_id: RunId,
        cursor: Option<EventCursor>,
    ) -> Result<AgentEventStream, AgentError> {
        self.subscribe_run_with_options(run_id, cursor, SubscriptionOptions::default())
    }

    fn subscribe_run_with_options(
        &self,
        run_id: RunId,
        cursor: Option<EventCursor>,
        options: SubscriptionOptions,
    ) -> Result<AgentEventStream, AgentError> {
        self.filtered_stream(
            EventStreamScope::Run(run_id.clone()),
            EventFilter::run(run_id).compile()?,
            cursor,
            options.queue,
        )
    }

    fn subscribe_agent(
        &self,
        agent_id: AgentId,
        cursor: Option<EventCursor>,
    ) -> Result<AgentEventStream, AgentError> {
        self.subscribe_agent_with_options(agent_id, cursor, SubscriptionOptions::default())
    }

    fn subscribe_agent_with_options(
        &self,
        agent_id: AgentId,
        cursor: Option<EventCursor>,
        options: SubscriptionOptions,
    ) -> Result<AgentEventStream, AgentError> {
        self.filtered_stream(
            EventStreamScope::Agent(agent_id.clone()),
            EventFilter::agent(agent_id).compile()?,
            cursor,
            options.queue,
        )
    }

    fn subscribe_filtered(
        &self,
        filter: CompiledEventFilter,
        cursor: Option<EventCursor>,
    ) -> Result<AgentEventStream, AgentError> {
        let queue = filter.queue.clone();
        self.filtered_stream(filter.cursor_scope(), filter, cursor, queue)
    }
}

fn apply_queue_bounds(
    frames: impl IntoIterator<Item = EventFrame>,
    queue: &SubscriberQueueConfig,
) -> Vec<EventFrame> {
    let capacity = queue.capacity.get();
    let normal_capacity = capacity.saturating_sub(queue.terminal_reserve.get().min(capacity));
    let mut bounded = VecDeque::new();
    let mut overflow = OverflowAccumulator::default();
    let mut summary = ProgressSummaryAccumulator::default();

    for frame in frames {
        if frame.event.envelope.event_kind.is_terminal() {
            flush_progress_summary(
                &mut bounded,
                queue,
                normal_capacity,
                &mut summary,
                &mut overflow,
            );
            while bounded.len() >= capacity {
                if !drop_oldest_nonterminal(
                    &mut bounded,
                    &mut overflow,
                    EventOverflowReason::PolicyDroppedNonTerminal,
                ) {
                    if let Some(dropped) = bounded.pop_front() {
                        overflow.record_drop(&dropped, EventOverflowReason::SubscriberQueueFull);
                    } else {
                        break;
                    }
                }
            }
            push_with_notice(&mut bounded, frame, queue.overflow.clone(), &mut overflow);
            continue;
        }

        match queue.overflow {
            SubscriberOverflowPolicy::DropNonTerminal => {
                if can_accept_nonterminal(&bounded, capacity, normal_capacity) {
                    push_with_notice(&mut bounded, frame, queue.overflow.clone(), &mut overflow);
                } else {
                    overflow.record_drop(&frame, EventOverflowReason::PolicyDroppedNonTerminal);
                }
            }
            SubscriberOverflowPolicy::DropProgress => {
                if can_accept_nonterminal(&bounded, capacity, normal_capacity) {
                    push_with_notice(&mut bounded, frame, queue.overflow.clone(), &mut overflow);
                } else if is_progress_event(&frame.event.envelope.event_kind) {
                    overflow.record_drop(&frame, EventOverflowReason::PolicyDroppedProgress);
                } else if drop_oldest_progress(&mut bounded, &mut overflow) {
                    push_with_notice(&mut bounded, frame, queue.overflow.clone(), &mut overflow);
                } else {
                    overflow.record_drop(&frame, EventOverflowReason::SubscriberQueueFull);
                }
            }
            SubscriberOverflowPolicy::SummarizeAndContinue => {
                if is_progress_event(&frame.event.envelope.event_kind) {
                    summary.record_progress(frame);
                } else {
                    flush_progress_summary(
                        &mut bounded,
                        queue,
                        normal_capacity,
                        &mut summary,
                        &mut overflow,
                    );
                    if can_accept_nonterminal(&bounded, capacity, normal_capacity) {
                        push_with_notice(
                            &mut bounded,
                            frame,
                            queue.overflow.clone(),
                            &mut overflow,
                        );
                    } else {
                        overflow.record_drop(&frame, EventOverflowReason::SubscriberQueueFull);
                    }
                }
            }
            SubscriberOverflowPolicy::FailSubscriber => {
                if can_accept_nonterminal(&bounded, capacity, normal_capacity) {
                    push_with_notice(&mut bounded, frame, queue.overflow.clone(), &mut overflow);
                } else {
                    overflow.record_drop(&frame, EventOverflowReason::SubscriberQueueFull);
                    if let Some(last) = bounded.back_mut() {
                        last.overflow = Some(overflow.take_notice(queue.overflow.clone()));
                    }
                    break;
                }
            }
            SubscriberOverflowPolicy::BackpressureCaller => unreachable!(
                "live event bus rejects backpressure overflow policy before queue bounding"
            ),
        }
    }

    flush_progress_summary(
        &mut bounded,
        queue,
        normal_capacity,
        &mut summary,
        &mut overflow,
    );

    if overflow.has_drop() {
        if let Some(last) = bounded.back_mut() {
            last.overflow = Some(overflow.notice(queue.overflow.clone()));
        }
    }

    bounded.into_iter().collect()
}

fn reject_live_overflow_policy(queue: &SubscriberQueueConfig) -> Result<(), AgentError> {
    if queue.overflow == SubscriberOverflowPolicy::BackpressureCaller {
        return Err(AgentError::contract_violation(
            "InvalidOverflowPolicy: backpressure_caller is rejected for live event bus subscriptions",
        ));
    }
    Ok(())
}

fn can_accept_nonterminal(
    frames: &VecDeque<EventFrame>,
    capacity: usize,
    normal_capacity: usize,
) -> bool {
    frames.len() < capacity && nonterminal_count(frames) < normal_capacity
}

fn push_with_notice(
    frames: &mut VecDeque<EventFrame>,
    mut frame: EventFrame,
    policy: SubscriberOverflowPolicy,
    overflow: &mut OverflowAccumulator,
) {
    if overflow.has_drop() {
        frame.overflow = Some(overflow.take_notice(policy));
    }
    frames.push_back(frame);
}

fn drop_oldest_nonterminal(
    frames: &mut VecDeque<EventFrame>,
    overflow: &mut OverflowAccumulator,
    reason: EventOverflowReason,
) -> bool {
    let Some(index) = frames
        .iter()
        .position(|frame| !frame.event.envelope.event_kind.is_terminal())
    else {
        return false;
    };
    if let Some(dropped) = frames.remove(index) {
        overflow.record_drop(&dropped, reason);
        true
    } else {
        false
    }
}

fn drop_oldest_progress(
    frames: &mut VecDeque<EventFrame>,
    overflow: &mut OverflowAccumulator,
) -> bool {
    let Some(index) = frames
        .iter()
        .position(|frame| is_progress_event(&frame.event.envelope.event_kind))
    else {
        return false;
    };
    if let Some(dropped) = frames.remove(index) {
        overflow.record_drop(&dropped, EventOverflowReason::PolicyDroppedProgress);
        true
    } else {
        false
    }
}

fn nonterminal_count(frames: &VecDeque<EventFrame>) -> usize {
    frames
        .iter()
        .filter(|frame| !frame.event.envelope.event_kind.is_terminal())
        .count()
}

fn is_progress_event(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::ModelStreamDelta
            | EventKind::StreamRuleRepeatStateRecorded
            | EventKind::RealtimeInputSent
            | EventKind::RealtimeOutputReceived
            | EventKind::RealtimeBackpressureApplied
            | EventKind::IsolationProcessIoCaptured
            | EventKind::IsolationProcessStatsRecorded
            | EventKind::UsageRecorded
            | EventKind::CostEstimated
            | EventKind::CostCorrected
    )
}

fn flush_progress_summary(
    frames: &mut VecDeque<EventFrame>,
    queue: &SubscriberQueueConfig,
    normal_capacity: usize,
    summary: &mut ProgressSummaryAccumulator,
    overflow: &mut OverflowAccumulator,
) {
    let Some(frame) = summary.take_summary_frame() else {
        return;
    };
    if can_accept_nonterminal(frames, queue.capacity.get(), normal_capacity) {
        push_with_notice(frames, frame, queue.overflow.clone(), overflow);
    } else {
        overflow.record_drop(&frame, EventOverflowReason::PolicyDroppedProgress);
    }
}

#[derive(Default)]
struct ProgressSummaryAccumulator {
    dropped_count: u64,
    gap_start: Option<EventCursor>,
    gap_end: Option<EventCursor>,
    repair_from: Option<crate::domain::JournalCursor>,
    summary_frame: Option<EventFrame>,
}

impl ProgressSummaryAccumulator {
    fn record_progress(&mut self, frame: EventFrame) {
        self.dropped_count += 1;
        self.gap_start.get_or_insert_with(|| frame.cursor.clone());
        self.gap_end = Some(frame.cursor.clone());
        if self.repair_from.is_none() {
            self.repair_from = frame.cursor.journal_cursor.clone();
        }
        self.summary_frame = Some(frame);
    }

    fn take_summary_frame(&mut self) -> Option<EventFrame> {
        if self.dropped_count == 0 {
            return None;
        }
        let mut frame = self.summary_frame.take()?;
        let notice = EventOverflowNotice {
            policy: SubscriberOverflowPolicy::SummarizeAndContinue,
            dropped_count: self.dropped_count,
            gap_start: self.gap_start.clone(),
            gap_end: self
                .gap_end
                .clone()
                .unwrap_or_else(|| self.gap_start.clone().expect("summary gap start")),
            repair_from: self.repair_from.clone(),
            terminal_preserved: true,
            reason: EventOverflowReason::PolicyDroppedProgress,
        };
        frame.event = AgentEvent::with_redacted_summary(
            frame.event.envelope.clone(),
            format!(
                "redacted progress summary for {} dropped progress frames",
                self.dropped_count
            ),
        );
        frame.overflow = Some(notice);
        *self = Self::default();
        Some(frame)
    }
}

#[derive(Default)]
struct OverflowAccumulator {
    dropped_count: u64,
    gap_start: Option<EventCursor>,
    gap_end: Option<EventCursor>,
    repair_from: Option<crate::domain::JournalCursor>,
    terminal_dropped: bool,
    reason: Option<EventOverflowReason>,
}

impl OverflowAccumulator {
    fn record_drop(&mut self, frame: &EventFrame, reason: EventOverflowReason) {
        self.dropped_count += 1;
        self.gap_start.get_or_insert_with(|| frame.cursor.clone());
        self.gap_end = Some(frame.cursor.clone());
        if self.repair_from.is_none() {
            self.repair_from = frame.cursor.journal_cursor.clone();
        }
        self.terminal_dropped |= frame.event.envelope.event_kind.is_terminal();
        self.reason.get_or_insert(reason);
    }

    fn has_drop(&self) -> bool {
        self.dropped_count > 0
    }

    fn take_notice(&mut self, policy: SubscriberOverflowPolicy) -> EventOverflowNotice {
        let notice = self.notice(policy);
        *self = Self::default();
        notice
    }

    fn notice(&self, policy: SubscriberOverflowPolicy) -> EventOverflowNotice {
        EventOverflowNotice {
            policy,
            dropped_count: self.dropped_count,
            gap_start: self.gap_start.clone(),
            gap_end: self
                .gap_end
                .clone()
                .unwrap_or_else(|| self.gap_start.clone().expect("overflow gap start")),
            repair_from: self.repair_from.clone(),
            terminal_preserved: !self.terminal_dropped,
            reason: if self.terminal_dropped {
                EventOverflowReason::SubscriberQueueFull
            } else {
                self.reason
                    .clone()
                    .unwrap_or(EventOverflowReason::SubscriberQueueFull)
            },
        }
    }
}
