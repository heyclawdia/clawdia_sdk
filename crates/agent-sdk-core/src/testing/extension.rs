//! Deterministic test-kit helpers for SDK consumers. Use these fakes and harnesses to
//! exercise public contracts without live providers, real stores, product UI, network
//! telemetry, or wall-clock-dependent infrastructure. They mutate only their
//! in-memory state unless noted. This file contains the extension portion of that
//! contract.
//!
use std::sync::{Arc, Mutex};

use crate::{
    domain::AgentError,
    extension_ports::{
        ExtensionActionExecutionOutput, ExtensionActionExecutionRequest, ExtensionActionExecutor,
    },
    package_extension::ExtensionBridgeRef,
};

#[derive(Clone)]
/// In-memory scripted extension action executor fixture for SDK conformance tests.
/// Use it to script deterministic behavior in memory; any transcript or endpoint mutation is documented on the method that performs it.
pub struct ScriptedExtensionActionExecutor {
    bridge_ref: ExtensionBridgeRef,
    output: ExtensionActionExecutionOutput,
    calls: Arc<Mutex<Vec<ExtensionActionExecutionRequest>>>,
}

impl ScriptedExtensionActionExecutor {
    /// Creates a new testing::extension value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    ///
    /// # Panics
    ///
    /// Panics if constructor invariants fail, such as invalid identifier
    /// text or constructor-specific bounds. Use a fallible constructor such as
    /// `try_new` when one is available for untrusted input.
    pub fn new(bridge_ref: ExtensionBridgeRef, output: ExtensionActionExecutionOutput) -> Self {
        Self {
            bridge_ref,
            output,
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Operates on in-memory or journal-derived testing::extension state for
    /// diagnostics and repair evidence. It does not create a second run loop
    /// or product workflow owner.
    pub fn calls(&self) -> Vec<ExtensionActionExecutionRequest> {
        self.calls
            .lock()
            .expect("extension action executor calls lock")
            .clone()
    }

    /// Returns the call count currently held by this value.
    /// This reads deterministic in-memory test state and performs no external I/O.
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
