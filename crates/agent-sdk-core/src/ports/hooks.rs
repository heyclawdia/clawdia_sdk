use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use crate::{
    domain::AgentError,
    package_hooks::{HookExecutorRef, HookInput, HookResponse},
};

pub trait HookExecutor: Send + Sync {
    fn executor_ref(&self) -> &HookExecutorRef;
    fn invoke(&self, input: HookInput) -> Result<HookExecutionOutcome, AgentError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HookExecutionOutcome {
    pub response: HookResponse,
    pub elapsed_ms: u64,
}

impl HookExecutionOutcome {
    pub fn new(response: HookResponse, elapsed_ms: u64) -> Self {
        Self {
            response,
            elapsed_ms,
        }
    }
}

pub trait HookExecutorRegistry: Send + Sync {
    fn resolve(&self, executor_ref: &HookExecutorRef) -> Option<Arc<dyn HookExecutor>>;

    fn contains(&self, executor_ref: &HookExecutorRef) -> bool {
        self.resolve(executor_ref).is_some()
    }
}

#[derive(Clone, Default)]
pub struct InMemoryHookExecutorRegistry {
    executors: Arc<Mutex<BTreeMap<HookExecutorRef, Arc<dyn HookExecutor>>>>,
}

impl InMemoryHookExecutorRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<E>(&self, executor: E) -> Result<(), AgentError>
    where
        E: HookExecutor + 'static,
    {
        self.executors
            .lock()
            .map_err(|_| AgentError::contract_violation("hook executor registry lock poisoned"))?
            .insert(executor.executor_ref().clone(), Arc::new(executor));
        Ok(())
    }
}

impl HookExecutorRegistry for InMemoryHookExecutorRegistry {
    fn resolve(&self, executor_ref: &HookExecutorRef) -> Option<Arc<dyn HookExecutor>> {
        self.executors
            .lock()
            .expect("hook executor registry lock")
            .get(executor_ref)
            .cloned()
    }
}
