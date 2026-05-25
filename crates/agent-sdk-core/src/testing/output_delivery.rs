//! Deterministic test-kit helpers for SDK consumers. Use these fakes and harnesses to
//! exercise public contracts without live providers, real stores, product UI, network
//! telemetry, or wall-clock-dependent infrastructure. They mutate only their
//! in-memory state unless noted. This file contains the output delivery portion of
//! that contract.
//!
use std::sync::{Arc, Mutex};

use crate::{
    domain::{AgentError, AgentErrorKind, RetryClassification},
    output_delivery::{OutputDeliveryReceipt, OutputDeliveryRequest, OutputSinkRef},
    output_delivery_port::{OutputSink, OutputSinkCapabilities},
};

#[derive(Clone)]
/// In-memory scripted output sink fixture for SDK conformance tests.
/// Use it to script deterministic behavior in memory; any transcript or endpoint mutation is documented on the method that performs it.
pub struct ScriptedOutputSink {
    sink_ref: OutputSinkRef,
    capabilities: OutputSinkCapabilities,
    calls: Arc<Mutex<Vec<OutputDeliveryRequest>>>,
    next_receipts: Arc<Mutex<Vec<Result<OutputDeliveryReceipt, AgentError>>>>,
}

impl ScriptedOutputSink {
    /// Creates a new testing::output_delivery value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    ///
    /// # Panics
    ///
    /// Panics if constructor invariants fail, such as invalid identifier
    /// text or constructor-specific bounds. Use a fallible constructor such as
    /// `try_new` when one is available for untrusted input.
    pub fn new(sink_ref: OutputSinkRef, capabilities: OutputSinkCapabilities) -> Self {
        Self {
            sink_ref,
            capabilities,
            calls: Arc::new(Mutex::new(Vec::new())),
            next_receipts: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Push receipt.
    /// This reads or mutates deterministic in-memory test state unless the method explicitly
    /// names a fixture file.
    pub fn push_receipt(&self, receipt: Result<OutputDeliveryReceipt, AgentError>) {
        self.next_receipts
            .lock()
            .expect("output sink receipt lock")
            .push(receipt);
    }

    /// Operates on in-memory or journal-derived testing::output_delivery
    /// state for diagnostics and repair evidence. It does not create a second
    /// run loop or product workflow owner.
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
