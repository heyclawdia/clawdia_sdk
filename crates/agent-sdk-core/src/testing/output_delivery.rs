use std::sync::{Arc, Mutex};

use crate::{
    domain::{AgentError, AgentErrorKind, RetryClassification},
    output_delivery::{OutputDeliveryReceipt, OutputDeliveryRequest, OutputSinkRef},
    output_delivery_port::{OutputSink, OutputSinkCapabilities},
};

#[derive(Clone)]
pub struct ScriptedOutputSink {
    sink_ref: OutputSinkRef,
    capabilities: OutputSinkCapabilities,
    calls: Arc<Mutex<Vec<OutputDeliveryRequest>>>,
    next_receipts: Arc<Mutex<Vec<Result<OutputDeliveryReceipt, AgentError>>>>,
}

impl ScriptedOutputSink {
    pub fn new(sink_ref: OutputSinkRef, capabilities: OutputSinkCapabilities) -> Self {
        Self {
            sink_ref,
            capabilities,
            calls: Arc::new(Mutex::new(Vec::new())),
            next_receipts: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn push_receipt(&self, receipt: Result<OutputDeliveryReceipt, AgentError>) {
        self.next_receipts
            .lock()
            .expect("output sink receipt lock")
            .push(receipt);
    }

    pub fn calls(&self) -> Vec<OutputDeliveryRequest> {
        self.calls.lock().expect("output sink calls lock").clone()
    }

    fn next_receipt(
        &self,
        request: OutputDeliveryRequest,
    ) -> Result<OutputDeliveryReceipt, AgentError> {
        self.calls
            .lock()
            .expect("output sink calls lock")
            .push(request.clone());
        self.next_receipts
            .lock()
            .expect("output sink receipt lock")
            .pop()
            .unwrap_or_else(|| {
                Ok(OutputDeliveryReceipt::completed(
                    request.delivery_id,
                    "ack.output_delivery.fake",
                ))
            })
    }
}

impl OutputSink for ScriptedOutputSink {
    fn sink_ref(&self) -> OutputSinkRef {
        self.sink_ref.clone()
    }

    fn capabilities(&self) -> OutputSinkCapabilities {
        self.capabilities.clone()
    }

    fn send_chunk(
        &self,
        request: OutputDeliveryRequest,
    ) -> Result<OutputDeliveryReceipt, AgentError> {
        if !self.capabilities.supports_chunks {
            return Err(AgentError::new(
                AgentErrorKind::HostConfigurationNeeded,
                RetryClassification::HostConfigurationNeeded,
                "output sink does not support chunks",
            ));
        }
        self.next_receipt(request)
    }

    fn send_final(
        &self,
        request: OutputDeliveryRequest,
    ) -> Result<OutputDeliveryReceipt, AgentError> {
        self.next_receipt(request)
    }

    fn reconcile(
        &self,
        request: OutputDeliveryRequest,
    ) -> Result<OutputDeliveryReceipt, AgentError> {
        self.next_receipt(request)
    }
}
