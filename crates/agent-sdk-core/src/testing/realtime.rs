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
pub struct ScriptedRealtimeAdapter {
    adapter_ref: String,
    calls: Arc<Mutex<Vec<RealtimeAdapterCall>>>,
    output_frames: Arc<Mutex<VecDeque<RealtimeOutputFrame>>>,
    next_connection_seq: Arc<Mutex<u64>>,
    fail_restart: Arc<Mutex<Option<String>>>,
}

impl ScriptedRealtimeAdapter {
    pub fn new(adapter_ref: impl Into<String>) -> Self {
        Self {
            adapter_ref: adapter_ref.into(),
            calls: Arc::new(Mutex::new(Vec::new())),
            output_frames: Arc::new(Mutex::new(VecDeque::new())),
            next_connection_seq: Arc::new(Mutex::new(1)),
            fail_restart: Arc::new(Mutex::new(None)),
        }
    }

    pub fn push_output(&self, frame: RealtimeOutputFrame) {
        self.output_frames
            .lock()
            .expect("realtime output lock")
            .push_back(frame);
    }

    pub fn fail_next_restart(&self, message: impl Into<String>) {
        *self
            .fail_restart
            .lock()
            .expect("realtime fail restart lock") = Some(message.into());
    }

    pub fn calls(&self) -> Vec<RealtimeAdapterCall> {
        self.calls.lock().expect("realtime calls lock").clone()
    }

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
