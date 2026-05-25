//! Deterministic test-kit helpers for SDK consumers. Use these fakes and harnesses to
//! exercise public contracts without live providers, real stores, product UI, network
//! telemetry, or wall-clock-dependent infrastructure. They mutate only their
//! in-memory state unless noted. This file contains the tool portion of that
//! contract.
//!
use std::sync::{Arc, Mutex};

use crate::{
    capability::ExecutorRef,
    domain::AgentError,
    tool_ports::{ToolExecutionOutput, ToolExecutionRequest, ToolExecutor},
};

#[derive(Clone)]
/// In-memory scripted tool executor fixture for SDK conformance tests.
/// Use it to script deterministic behavior in memory; any transcript or endpoint mutation is documented on the method that performs it.
pub struct ScriptedToolExecutor {
    executor_ref: ExecutorRef,
    output: ToolExecutionOutput,
    calls: Arc<Mutex<Vec<ToolExecutionRequest>>>,
}

impl ScriptedToolExecutor {
    /// Creates a new testing::tool value with explicit caller-provided
    /// inputs. This constructor is data-only and performs no I/O or
    /// external side effects.
    ///
    /// # Panics
    ///
    /// Panics if constructor invariants fail, such as invalid identifier
    /// text or constructor-specific bounds. Use a fallible constructor such as
    /// `try_new` when one is available for untrusted input.
    pub fn new(executor_ref: ExecutorRef, output: ToolExecutionOutput) -> Self {
        Self {
            executor_ref,
            output,
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Operates on in-memory or journal-derived testing::tool state for
    /// diagnostics and repair evidence. It does not create a second run loop
    /// or product workflow owner.
    pub fn calls(&self) -> Vec<ToolExecutionRequest> {
        self.calls.lock().expect("tool executor calls lock").clone()
    }

    /// Returns the call count currently held by this value.
    /// This reads deterministic in-memory test state and performs no external I/O.
    pub fn call_count(&self) -> usize {
        self.calls.lock().expect("tool executor calls lock").len()
    }
}

impl ToolExecutor for ScriptedToolExecutor {
    fn executor_ref(&self) -> &ExecutorRef {
        &self.executor_ref
    }

    fn execute(&self, request: &ToolExecutionRequest) -> Result<ToolExecutionOutput, AgentError> {
        self.calls
            .lock()
            .expect("tool executor calls lock")
            .push(request.clone());
        Ok(self.output.clone())
    }
}
