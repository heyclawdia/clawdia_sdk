//! Reconnectable run-handle helpers. Use this module when a host needs to wait for
//! output, stream events from a cursor, replay journal frames, or request
//! cancellation. Handle operations read or mutate the configured run-control store
//! and may publish cancellation intent.
//!
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
/// Holds run handle application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct RunHandle {
    run_id: RunId,
    control: Arc<dyn RunControlStore>,
    subscriptions: Arc<dyn RunSubscriptionSource>,
}

impl RunHandle {
    /// Creates a new application::run_handle value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
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

    /// Returns run id for this application::run_handle value without
    /// performing external I/O.
    pub fn run_id(&self) -> &RunId {
        &self.run_id
    }

    /// Returns the terminal result once the handle store, terminal event, and
    /// journal agree. This reads run-control state and does not drive the run or
    /// start new side effects.
    pub fn wait(&self) -> Result<RunResult, AgentError> {
        self.consistent_terminal_result()?.ok_or_else(|| {
            AgentError::contract_violation(
                "run is not terminal until journal, handle status, and terminal event agree",
            )
        })
    }

    /// Returns the terminal result if it is already available before the
    /// timeout budget. This first-slice implementation is non-blocking and does
    /// not cancel or drive the run.
    pub fn wait_with_timeout(&self, _timeout: Duration) -> Result<Option<RunResult>, AgentError> {
        self.consistent_terminal_result()
    }

    /// Returns the status currently held by this value.
    /// This reads run-control status for the handle and does not change the run.
    pub fn status(&self) -> Result<RunStatus, AgentError> {
        self.control.status(&self.run_id)
    }

    /// Opens an event stream for this run from the supplied live-event cursor.
    /// This delegates to the configured subscription port to create a read-only stream; it does
    /// not drive the run, execute tools, or call the provider.
    pub fn stream_from(&self, cursor: Option<EventCursor>) -> Result<AgentEventStream, AgentError> {
        self.subscriptions
            .subscribe_run(self.run_id.clone(), cursor)
    }

    /// Opens a replay-derived stream for this run from a journal cursor.
    /// This delegates to the subscription port's journal replay path and does not mutate run
    /// control or execute runtime work.
    pub fn stream_from_journal(
        &self,
        cursor: JournalCursor,
    ) -> Result<AgentEventStream, AgentError> {
        self.subscriptions
            .replay_run_from_cursor(self.run_id.clone(), cursor)
    }

    /// Cancel.
    /// This forwards cancellation to run control; adapter/process cleanup remains owned by the
    /// run coordinator.
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

/// Port or behavior contract for run control store. Implementors should
/// preserve policy, redaction, idempotency, and replay expectations
/// from the surrounding module. Implementations may perform side
/// effects only as described by the trait methods.
pub trait RunControlStore: Send + Sync {
    /// Returns the status currently held by this value.
    /// This reads run-control status for the handle and does not change the run.
    fn status(&self, run_id: &RunId) -> Result<RunStatus, AgentError>;
    /// Returns terminal result for callers that need to inspect the contract state.
    /// Implementations read terminal result state for the run and do not change run status.
    fn terminal_result(&self, run_id: &RunId) -> Result<Option<RunResult>, AgentError>;
    /// Requests cancellation for a registered run.
    /// Implementations may mutate run-control state to record the request; provider/tool cleanup
    /// remains owned by the run coordinator that observes the cancellation.
    fn request_cancel(&self, run_id: &RunId) -> Result<(), AgentError>;
}

#[derive(Clone, Debug, Default)]
/// Holds in memory run control store application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct InMemoryRunControlStore {
    records: Arc<Mutex<BTreeMap<RunId, RunControlRecord>>>,
}

impl InMemoryRunControlStore {
    /// Register run.
    /// This inserts a run into in-memory test run-control state for deterministic handle tests.
    pub fn register_run(&self, run_id: RunId, agent_id: AgentId) -> Result<(), AgentError> {
        self.records
            .lock()
            .map_err(|_| AgentError::contract_violation("run control store lock poisoned"))?
            .entry(run_id)
            .or_insert_with(|| RunControlRecord::new(agent_id));
        Ok(())
    }

    /// Mark visible output complete.
    /// This records final visible output in in-memory test run-control state.
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

    /// Seals terminal run-control state from a journal terminal record.
    /// This stores the derived `RunResult` in the in-memory run-control map; it does not append a
    /// journal record, publish an event, or execute provider/tool work.
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

    /// Returns the cancel request count currently held by this value.
    /// This reads the number of cancellation requests recorded for the run.
    pub fn cancel_request_count(&self, run_id: &RunId) -> Result<usize, AgentError> {
        Ok(self
            .records
            .lock()
            .map_err(|_| AgentError::contract_violation("run control store lock poisoned"))?
            .get(run_id)
            .map(|record| record.cancel_request_count)
            .unwrap_or(0))
    }

    /// Returns visible output for callers that need to inspect the contract state.
    /// This reads visible output state from run control and does not change run status or
    /// output delivery.
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
