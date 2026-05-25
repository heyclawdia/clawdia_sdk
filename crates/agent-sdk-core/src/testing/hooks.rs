//! Deterministic test-kit helpers for SDK consumers. Use these fakes and harnesses to
//! exercise public contracts without live providers, real stores, product UI, network
//! telemetry, or wall-clock-dependent infrastructure. They mutate only their
//! in-memory state unless noted. This file contains the hooks portion of that
//! contract.
//!
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use crate::{
    domain::AgentError,
    hook_ports::{HookExecutionOutcome, HookExecutor},
    package_hooks::{HookExecutorRef, HookInput, HookResponse},
};

#[derive(Clone, Debug)]
/// In-memory scripted hook executor fixture for SDK conformance tests.
/// Use it to script deterministic behavior in memory; any transcript or endpoint mutation is documented on the method that performs it.
pub struct ScriptedHookExecutor {
    executor_ref: HookExecutorRef,
    outcomes: Arc<Mutex<VecDeque<Result<HookExecutionOutcome, AgentError>>>>,
    invocations: Arc<Mutex<Vec<HookInput>>>,
}

impl ScriptedHookExecutor {
    /// Creates a new testing::hooks value with explicit caller-provided
    /// inputs. This constructor is data-only and performs no I/O or
    /// external side effects.
    pub fn new(
        executor_ref: impl Into<String>,
        outcomes: impl IntoIterator<Item = Result<HookExecutionOutcome, AgentError>>,
    ) -> Self {
        Self {
            executor_ref: HookExecutorRef::new(executor_ref),
            outcomes: Arc::new(Mutex::new(outcomes.into_iter().collect())),
            invocations: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Builds the once value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn once(executor_ref: impl Into<String>, response: HookResponse, elapsed_ms: u64) -> Self {
        Self::new(
            executor_ref,
            [Ok(HookExecutionOutcome::new(response, elapsed_ms))],
        )
    }

    /// Returns the invocations currently held by this value.
    /// This configures deterministic in-memory test state only.
    pub fn invocations(&self) -> Vec<HookInput> {
        self.invocations
            .lock()
            .expect("hook invocations lock")
            .clone()
    }
}

impl HookExecutor for ScriptedHookExecutor {
    fn executor_ref(&self) -> &HookExecutorRef {
        &self.executor_ref
    }

    fn invoke(&self, input: HookInput) -> Result<HookExecutionOutcome, AgentError> {
        self.invocations
            .lock()
            .expect("hook invocations lock")
            .push(input);
        self.outcomes
            .lock()
            .expect("hook outcomes lock")
            .pop_front()
            .unwrap_or_else(|| Err(AgentError::contract_violation("scripted hook exhausted")))
    }
}
