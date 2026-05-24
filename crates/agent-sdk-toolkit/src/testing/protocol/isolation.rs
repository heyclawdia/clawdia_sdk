use agent_sdk_core::{
    AgentError, EffectIntent, ExecutionEnvironment, IsolatedProcessSpec, IsolationCapabilityReport,
    IsolationRuntime, ProcessStartRequest, ProcessStartResult,
};

use crate::protocol::JsonRpcLineEndpoint;

#[derive(Clone, Debug)]
pub struct IsolatedJsonRpcProcess {
    pub host_endpoint: JsonRpcLineEndpoint,
    pub process_endpoint: JsonRpcLineEndpoint,
    pub capability_report: IsolationCapabilityReport,
    pub start_result: ProcessStartResult,
}

impl IsolatedJsonRpcProcess {
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
