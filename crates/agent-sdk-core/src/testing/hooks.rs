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
pub struct ScriptedHookExecutor {
    executor_ref: HookExecutorRef,
    outcomes: Arc<Mutex<VecDeque<Result<HookExecutionOutcome, AgentError>>>>,
    invocations: Arc<Mutex<Vec<HookInput>>>,
}

impl ScriptedHookExecutor {
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

    pub fn once(executor_ref: impl Into<String>, response: HookResponse, elapsed_ms: u64) -> Self {
        Self::new(
            executor_ref,
            [Ok(HookExecutionOutcome::new(response, elapsed_ms))],
        )
    }

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
