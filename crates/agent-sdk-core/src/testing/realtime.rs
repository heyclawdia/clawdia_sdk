//! Deterministic test-kit helpers for SDK consumers. Use these fakes and harnesses to
//! exercise public contracts without live providers, real stores, product UI, network
//! telemetry, or wall-clock-dependent infrastructure. They mutate only their
//! in-memory state unless noted. This file contains the realtime portion of that
//! contract.
//!
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use crate::{
    domain::{AgentError, AgentErrorKind, RetryClassification},
    ports::realtime::{
        RealtimeAdapterAck, RealtimeAdapterCall, RealtimeConnectRequest, RealtimeConnectResponse,
        RealtimeProviderAdapter,
    },
    realtime_records::{
        RealtimeCloseReason, RealtimeConnectionId, RealtimeInputFrame, RealtimeOutputFrame,
        RealtimeResponseId, RealtimeSessionId,
    },
};

#[derive(Clone, Debug)]
/// In-memory scripted realtime adapter fixture for SDK conformance tests.
/// Use it to script deterministic behavior in memory; any transcript or endpoint mutation is documented on the method that performs it.
pub struct ScriptedRealtimeAdapter {
    adapter_ref: String,
    calls: Arc<Mutex<Vec<RealtimeAdapterCall>>>,
    output_frames: Arc<Mutex<VecDeque<RealtimeOutputFrame>>>,
    next_connection_seq: Arc<Mutex<u64>>,
    fail_restart: Arc<Mutex<Option<String>>>,
}

impl ScriptedRealtimeAdapter {
    /// Creates a new testing::realtime value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    ///
    /// # Panics
    ///
    /// Panics if constructor invariants fail, such as invalid identifier
    /// text or constructor-specific bounds. Use a fallible constructor such as
    /// `try_new` when one is available for untrusted input.
    pub fn new(adapter_ref: impl Into<String>) -> Self {
        Self {
            adapter_ref: adapter_ref.into(),
            calls: Arc::new(Mutex::new(Vec::new())),
            output_frames: Arc::new(Mutex::new(VecDeque::new())),
            next_connection_seq: Arc::new(Mutex::new(1)),
            fail_restart: Arc::new(Mutex::new(None)),
        }
    }

    /// Push output.
    /// This reads or mutates deterministic in-memory test state unless the method explicitly
    /// names a fixture file.
    pub fn push_output(&self, frame: RealtimeOutputFrame) {
        self.output_frames
            .lock()
            .expect("realtime output lock")
            .push_back(frame);
    }

    /// Fail next restart.
    /// This reads or mutates deterministic in-memory test state unless the method explicitly
    /// names a fixture file.
    pub fn fail_next_restart(&self, message: impl Into<String>) {
        *self
            .fail_restart
            .lock()
            .expect("realtime fail restart lock") = Some(message.into());
    }

    /// Operates on in-memory or journal-derived testing::realtime state for
    /// diagnostics and repair evidence. It does not create a second run loop
    /// or product workflow owner.
    pub fn calls(&self) -> Vec<RealtimeAdapterCall> {
        self.calls.lock().expect("realtime calls lock").clone()
    }

    /// Returns the call names currently held by this value.
    /// This configures deterministic in-memory test state only.
    pub fn call_names(&self) -> Vec<&'static str> {
        self.calls().iter().map(RealtimeAdapterCall::name).collect()
    }

    fn next_connection_id(&self) -> RealtimeConnectionId {
        let mut seq = self
            .next_connection_seq
            .lock()
            .expect("realtime connection seq lock");
        let connection_id = RealtimeConnectionId::new(format!("realtime.connection.{seq}"));
        *seq += 1;
        connection_id
    }
}

impl RealtimeProviderAdapter for ScriptedRealtimeAdapter {
    fn adapter_ref(&self) -> &str {
        &self.adapter_ref
    }

    fn connect(
        &self,
        request: RealtimeConnectRequest,
    ) -> Result<RealtimeConnectResponse, AgentError> {
        self.calls
            .lock()
            .expect("realtime calls lock")
            .push(RealtimeAdapterCall::Connect {
                session_id: request.session_id.clone(),
            });
        Ok(RealtimeConnectResponse {
            session_id: request.session_id,
            connection_id: self.next_connection_id(),
            redacted_summary: "realtime connected".to_string(),
        })
    }

    fn send(
        &self,
        session_id: &RealtimeSessionId,
        frame: RealtimeInputFrame,
    ) -> Result<(), AgentError> {
        self.calls
            .lock()
            .expect("realtime calls lock")
            .push(RealtimeAdapterCall::Send {
                session_id: session_id.clone(),
                redacted_summary: frame.redacted_summary,
            });
        Ok(())
    }

    fn receive(
        &self,
        session_id: &RealtimeSessionId,
    ) -> Result<Option<RealtimeOutputFrame>, AgentError> {
        self.calls
            .lock()
            .expect("realtime calls lock")
            .push(RealtimeAdapterCall::Receive {
                session_id: session_id.clone(),
            });
        Ok(self
            .output_frames
            .lock()
            .expect("realtime output lock")
            .pop_front())
    }

    fn interrupt(
        &self,
        session_id: &RealtimeSessionId,
        response_id: &RealtimeResponseId,
    ) -> Result<RealtimeAdapterAck, AgentError> {
        self.calls
            .lock()
            .expect("realtime calls lock")
            .push(RealtimeAdapterCall::Interrupt {
                session_id: session_id.clone(),
                response_id: response_id.clone(),
            });
        Ok(RealtimeAdapterAck {
            redacted_summary: "realtime response interrupted".to_string(),
        })
    }

    fn restart(
        &self,
        session_id: &RealtimeSessionId,
        current_connection_id: &RealtimeConnectionId,
    ) -> Result<RealtimeConnectResponse, AgentError> {
        if let Some(message) = self
            .fail_restart
            .lock()
            .expect("realtime fail restart lock")
            .take()
        {
            return Err(AgentError::new(
                AgentErrorKind::ProviderFailure,
                RetryClassification::Retryable,
                message,
            ));
        }
        self.calls
            .lock()
            .expect("realtime calls lock")
            .push(RealtimeAdapterCall::Restart {
                session_id: session_id.clone(),
                current_connection_id: current_connection_id.clone(),
            });
        Ok(RealtimeConnectResponse {
            session_id: session_id.clone(),
            connection_id: self.next_connection_id(),
            redacted_summary: "realtime restarted".to_string(),
        })
    }

    fn close(
        &self,
        session_id: &RealtimeSessionId,
        reason: RealtimeCloseReason,
    ) -> Result<RealtimeAdapterAck, AgentError> {
        self.calls
            .lock()
            .expect("realtime calls lock")
            .push(RealtimeAdapterCall::Close {
                session_id: session_id.clone(),
                reason,
            });
        Ok(RealtimeAdapterAck {
            redacted_summary: "realtime closed".to_string(),
        })
    }
}
