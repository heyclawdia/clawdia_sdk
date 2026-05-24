use std::sync::{Arc, Mutex};

use crate::{
    domain::AgentError,
    extension_ports::{
        ExtensionActionExecutionOutput, ExtensionActionExecutionRequest, ExtensionActionExecutor,
    },
    package_extension::ExtensionBridgeRef,
};

#[derive(Clone)]
pub struct ScriptedExtensionActionExecutor {
    bridge_ref: ExtensionBridgeRef,
    output: ExtensionActionExecutionOutput,
    calls: Arc<Mutex<Vec<ExtensionActionExecutionRequest>>>,
}

impl ScriptedExtensionActionExecutor {
    pub fn new(bridge_ref: ExtensionBridgeRef, output: ExtensionActionExecutionOutput) -> Self {
        Self {
            bridge_ref,
            output,
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn calls(&self) -> Vec<ExtensionActionExecutionRequest> {
        self.calls
            .lock()
            .expect("extension action executor calls lock")
            .clone()
    }

    pub fn call_count(&self) -> usize {
        self.calls
            .lock()
            .expect("extension action executor calls lock")
            .len()
    }
}

impl ExtensionActionExecutor for ScriptedExtensionActionExecutor {
    fn bridge_ref(&self) -> &ExtensionBridgeRef {
        &self.bridge_ref
    }

    fn execute(
        &self,
        request: &ExtensionActionExecutionRequest,
    ) -> Result<ExtensionActionExecutionOutput, AgentError> {
        self.calls
            .lock()
            .expect("extension action executor calls lock")
            .push(request.clone());
        Ok(self.output.clone())
    }
}
