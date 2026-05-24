use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::{
    domain::{AgentError, AgentId, RunId},
    event::{EventCursor, EventDeliverySemantics, EventKind},
    event_bus::AgentEventStream,
    journal::{JournalCursor, JournalRecord, JournalRecordPayload},
    run::{RunResult, RunStatus},
    subscription::RunSubscriptionSource,
};

#[derive(Clone)]
pub struct RunHandle {
    run_id: RunId,
    control: Arc<dyn RunControlStore>,
    subscriptions: Arc<dyn RunSubscriptionSource>,
}

impl RunHandle {
    pub fn new(
        run_id: RunId,
        control: Arc<dyn RunControlStore>,
        subscriptions: Arc<dyn RunSubscriptionSource>,
    ) -> Self {
        Self {
            run_id,
            control,
            subscriptions,
        }
    }

    pub fn run_id(&self) -> &RunId {
        &self.run_id
    }

    pub fn wait(&self) -> Result<RunResult, AgentError> {
        self.consistent_terminal_result()?.ok_or_else(|| {
            AgentError::contract_violation(
                "run is not terminal until journal, handle status, and terminal event agree",
            )
        })
    }

    pub fn wait_with_timeout(&self, _timeout: Duration) -> Result<Option<RunResult>, AgentError> {
        self.consistent_terminal_result()
    }

    pub fn status(&self) -> Result<RunStatus, AgentError> {
        self.control.status(&self.run_id)
    }

    pub fn stream_from(&self, cursor: Option<EventCursor>) -> Result<AgentEventStream, AgentError> {
        self.subscriptions
            .subscribe_run(self.run_id.clone(), cursor)
    }

    pub fn stream_from_journal(
        &self,
        cursor: JournalCursor,
    ) -> Result<AgentEventStream, AgentError> {
        self.subscriptions
            .replay_run_from_cursor(self.run_id.clone(), cursor)
    }

    pub fn cancel(&self) -> Result<(), AgentError> {
        self.control.request_cancel(&self.run_id)
    }

    fn consistent_terminal_result(&self) -> Result<Option<RunResult>, AgentError> {
        let Some(result) = self.control.terminal_result(&self.run_id)? else {
            return Ok(None);
        };
        if !result.status.is_terminal() {
            return Err(AgentError::contract_violation(
                "terminal result carried non-terminal run status",
            ));
        }

        let Some(frame) = self.subscriptions.latest_terminal_event(&self.run_id)? else {
            return Ok(None);
        };
        let Some(event_status) = status_from_terminal_event_kind(&frame.event.envelope.event_kind)
        else {
            return Ok(None);
        };
        if event_status != result.status {
            return Err(AgentError::contract_violation(
                "terminal event status does not match sealed journal result",
            ));
        }
        if !matches!(
            frame.event.envelope.delivery_semantics,
            EventDeliverySemantics::JournalBacked | EventDeliverySemantics::DerivedReplay
        ) || frame.event.envelope.journal_cursor.is_none()
        {
            return Err(AgentError::contract_violation(
                "terminal event must be journal-backed or derived from a journal cursor",
            ));
        }

        Ok(Some(result))
    }
}

impl core::fmt::Debug for RunHandle {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("RunHandle")
            .field("run_id", &self.run_id)
            .finish_non_exhaustive()
    }
}

pub trait RunControlStore: Send + Sync {
    fn status(&self, run_id: &RunId) -> Result<RunStatus, AgentError>;
    fn terminal_result(&self, run_id: &RunId) -> Result<Option<RunResult>, AgentError>;
    fn request_cancel(&self, run_id: &RunId) -> Result<(), AgentError>;
}

#[derive(Clone, Debug, Default)]
pub struct InMemoryRunControlStore {
    records: Arc<Mutex<BTreeMap<RunId, RunControlRecord>>>,
}

impl InMemoryRunControlStore {
    pub fn register_run(&self, run_id: RunId, agent_id: AgentId) -> Result<(), AgentError> {
        self.records
            .lock()
            .map_err(|_| AgentError::contract_violation("run control store lock poisoned"))?
            .entry(run_id)
            .or_insert_with(|| RunControlRecord::new(agent_id));
        Ok(())
    }

    pub fn mark_visible_output_complete(
        &self,
        run_id: &RunId,
        output: impl Into<String>,
    ) -> Result<(), AgentError> {
        let mut records = self
            .records
            .lock()
            .map_err(|_| AgentError::contract_violation("run control store lock poisoned"))?;
        let record = records
            .get_mut(run_id)
            .ok_or_else(|| AgentError::contract_violation("run is not registered"))?;
        record.visible_output = Some(output.into());
        Ok(())
    }

    pub fn seal_terminal_result_from_journal(
        &self,
        record: &JournalRecord,
        output: impl Into<String>,
    ) -> Result<RunResult, AgentError> {
        let status = match &record.payload {
            JournalRecordPayload::TerminalResult(marker) => {
                RunStatus::from_terminal_str(&marker.terminal_status).ok_or_else(|| {
                    AgentError::contract_violation("journal terminal status is not recognized")
                })?
            }
            _ => {
                return Err(AgentError::contract_violation(
                    "journal record is not a terminal result marker",
                ));
            }
        };

        let result = RunResult::new(record.run_id.clone(), status.clone(), output);
        let mut records = self
            .records
            .lock()
            .map_err(|_| AgentError::contract_violation("run control store lock poisoned"))?;
        let stored = records
            .entry(record.run_id.clone())
            .or_insert_with(|| RunControlRecord::new(record.agent_id.clone()));

        if let Some(existing) = stored.final_result.as_ref() {
            if existing != &result {
                return Err(AgentError::contract_violation(
                    "journal terminal result conflicts with existing handle result",
                ));
            }
            return Ok(existing.clone());
        }

        stored.status = status.clone();
        stored.journal_terminal_status = Some(status);
        stored.final_result = Some(result.clone());
        Ok(result)
    }

    pub fn cancel_request_count(&self, run_id: &RunId) -> Result<usize, AgentError> {
        Ok(self
            .records
            .lock()
            .map_err(|_| AgentError::contract_violation("run control store lock poisoned"))?
            .get(run_id)
            .map(|record| record.cancel_request_count)
            .unwrap_or(0))
    }

    pub fn visible_output(&self, run_id: &RunId) -> Result<Option<String>, AgentError> {
        Ok(self
            .records
            .lock()
            .map_err(|_| AgentError::contract_violation("run control store lock poisoned"))?
            .get(run_id)
            .and_then(|record| record.visible_output.clone()))
    }
}

impl RunControlStore for InMemoryRunControlStore {
    fn status(&self, run_id: &RunId) -> Result<RunStatus, AgentError> {
        Ok(self
            .records
            .lock()
            .map_err(|_| AgentError::contract_violation("run control store lock poisoned"))?
            .get(run_id)
            .map(|record| record.status.clone())
            .unwrap_or(RunStatus::Pending))
    }

    fn terminal_result(&self, run_id: &RunId) -> Result<Option<RunResult>, AgentError> {
        let records = self
            .records
            .lock()
            .map_err(|_| AgentError::contract_violation("run control store lock poisoned"))?;
        let Some(record) = records.get(run_id) else {
            return Ok(None);
        };
        let Some(result) = record.final_result.clone() else {
            return Ok(None);
        };
        if record.status != result.status
            || record.journal_terminal_status.as_ref() != Some(&result.status)
        {
            return Err(AgentError::contract_violation(
                "handle status and journal terminal record disagree",
            ));
        }
        Ok(Some(result))
    }

    fn request_cancel(&self, run_id: &RunId) -> Result<(), AgentError> {
        let mut records = self
            .records
            .lock()
            .map_err(|_| AgentError::contract_violation("run control store lock poisoned"))?;
        let record = records
            .get_mut(run_id)
            .ok_or_else(|| AgentError::contract_violation("run is not registered"))?;
        if record.status.is_terminal() || record.cancel_requested {
            return Ok(());
        }
        record.cancel_requested = true;
        record.cancel_request_count += 1;
        record.status = RunStatus::Cancelling;
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct RunControlRecord {
    #[allow(dead_code)]
    agent_id: AgentId,
    status: RunStatus,
    visible_output: Option<String>,
    journal_terminal_status: Option<RunStatus>,
    final_result: Option<RunResult>,
    cancel_requested: bool,
    cancel_request_count: usize,
}

impl RunControlRecord {
    fn new(agent_id: AgentId) -> Self {
        Self {
            agent_id,
            status: RunStatus::Running,
            visible_output: None,
            journal_terminal_status: None,
            final_result: None,
            cancel_requested: false,
            cancel_request_count: 0,
        }
    }
}

fn status_from_terminal_event_kind(kind: &EventKind) -> Option<RunStatus> {
    match kind {
        EventKind::RunCompleted => Some(RunStatus::Completed),
        EventKind::RunFailed => Some(RunStatus::Failed),
        EventKind::RunCancelled => Some(RunStatus::Cancelled),
        _ => None,
    }
}
