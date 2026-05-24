use std::sync::{Arc, Mutex};

use crate::{
    capability::ExecutorRef,
    domain::AgentError,
    tool_ports::{ToolExecutionOutput, ToolExecutionRequest, ToolExecutor},
};

#[derive(Clone)]
pub struct ScriptedToolExecutor {
    executor_ref: ExecutorRef,
    output: ToolExecutionOutput,
    calls: Arc<Mutex<Vec<ToolExecutionRequest>>>,
}

impl ScriptedToolExecutor {
    pub fn new(executor_ref: ExecutorRef, output: ToolExecutionOutput) -> Self {
        Self {
            executor_ref,
            output,
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn calls(&self) -> Vec<ToolExecutionRequest> {
        self.calls.lock().expect("tool executor calls lock").clone()
    }

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
