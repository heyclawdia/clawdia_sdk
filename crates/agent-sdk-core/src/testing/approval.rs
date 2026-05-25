//! Deterministic test-kit helpers for SDK consumers. Use these fakes and harnesses to
//! exercise public contracts without live providers, real stores, product UI, network
//! telemetry, or wall-clock-dependent infrastructure. They mutate only their
//! in-memory state unless noted. This file contains the approval portion of that
//! contract.
//!
use std::sync::{Arc, Mutex};

use crate::{
    approval_ports::{ApprovalDispatchRequest, ApprovalDispatchResponse, ApprovalDispatcher},
    domain::AgentError,
};

#[derive(Clone)]
/// In-memory scripted approval dispatcher fixture for SDK conformance tests.
/// Use it to script deterministic behavior in memory; any transcript or endpoint mutation is documented on the method that performs it.
pub struct ScriptedApprovalDispatcher {
    response: ApprovalDispatchResponse,
    requests: Arc<Mutex<Vec<ApprovalDispatchRequest>>>,
}

impl ScriptedApprovalDispatcher {
    /// Creates a new testing::approval value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    ///
    /// # Panics
    ///
    /// Panics if constructor invariants fail, such as invalid identifier
    /// text or constructor-specific bounds. Use a fallible constructor such as
    /// `try_new` when one is available for untrusted input.
    pub fn new(response: ApprovalDispatchResponse) -> Self {
        Self {
            response,
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Returns the requests currently held by this value.
    /// This configures deterministic in-memory test state only.
    pub fn requests(&self) -> Vec<ApprovalDispatchRequest> {
        self.requests
            .lock()
            .expect("approval dispatcher requests lock")
            .clone()
    }
}

impl ApprovalDispatcher for ScriptedApprovalDispatcher {
    fn dispatch(
        &self,
        request: ApprovalDispatchRequest,
    ) -> Result<ApprovalDispatchResponse, AgentError> {
        self.requests
            .lock()
            .expect("approval dispatcher requests lock")
            .push(request);
        Ok(self.response.clone())
    }
}
