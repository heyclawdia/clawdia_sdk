//! Scripted isolated process harness for protocol tests. Use this fake when tests
//! need process-like JSON-RPC exchange without launching a real runtime. It mutates
//! only in-memory endpoint state.
//!
use agent_sdk_core::{
    AgentError, EffectIntent, ExecutionEnvironment, IsolatedProcessSpec, IsolationCapabilityReport,
    IsolationRuntime, ProcessStartRequest, ProcessStartResult,
};

use crate::protocol::JsonRpcLineEndpoint;

#[derive(Clone, Debug)]
/// In-memory isolated json rpc process fixture for SDK conformance tests.
/// Use it to script deterministic behavior in memory; any transcript or endpoint mutation is documented on the method that performs it.
pub struct IsolatedJsonRpcProcess {
    /// Host endpoint used by this record or request.
    pub host_endpoint: JsonRpcLineEndpoint,
    /// Process endpoint used by this record or request.
    pub process_endpoint: JsonRpcLineEndpoint,
    /// Capability report used by this record or request.
    pub capability_report: IsolationCapabilityReport,
    /// Start result used by this record or request.
    pub start_result: ProcessStartResult,
}

impl IsolatedJsonRpcProcess {
    /// Start.
    /// This records a scripted isolation start result for protocol tests without launching a
    /// host process.
    pub fn start(
        runtime: &dyn IsolationRuntime,
        environment: ExecutionEnvironment,
        process: IsolatedProcessSpec,
        effect_intent: EffectIntent,
    ) -> Result<Self, AgentError> {
        let capability_report = runtime.capability_report()?;
        let start_result = runtime.start_process(ProcessStartRequest {
            environment,
            process,
            effect_intent,
        })?;
        let (host_endpoint, process_endpoint) =
            JsonRpcLineEndpoint::pair("host", "isolated-process");
        Ok(Self {
            host_endpoint,
            process_endpoint,
            capability_report,
            start_result,
        })
    }
}
