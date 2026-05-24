use std::{collections::BTreeMap, sync::Arc};

use crate::{
    domain::AgentError,
    output_delivery::{
        OutputContentMode, OutputDeliveryKind, OutputDeliveryReceipt, OutputDeliveryRequest,
        OutputSinkRef,
    },
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutputSinkCapabilities {
    pub sink_ref: OutputSinkRef,
    pub capability_version: String,
    pub supports_chunks: bool,
    pub supports_final_messages: bool,
    pub supports_final_validated_outputs: bool,
    pub supported_content_modes: Vec<OutputContentMode>,
    pub can_resolve_content_refs: bool,
    pub may_receive_raw_content: bool,
}

impl OutputSinkCapabilities {
    pub fn refs_and_summaries(sink_ref: OutputSinkRef) -> Self {
        Self {
            sink_ref,
            capability_version: "output.sink.capabilities.v1".to_string(),
            supports_chunks: true,
            supports_final_messages: true,
            supports_final_validated_outputs: true,
            supported_content_modes: vec![
                OutputContentMode::ContentRefsOnly,
                OutputContentMode::RedactedSummary,
            ],
            can_resolve_content_refs: true,
            may_receive_raw_content: false,
        }
    }

    pub fn with_raw_content(mut self) -> Self {
        if !self
            .supported_content_modes
            .contains(&OutputContentMode::RawContentIfPolicyAllows)
        {
            self.supported_content_modes
                .push(OutputContentMode::RawContentIfPolicyAllows);
        }
        self.may_receive_raw_content = true;
        self
    }

    pub fn supports_kind(&self, kind: &OutputDeliveryKind) -> bool {
        match kind {
            OutputDeliveryKind::StreamChunk { .. } => self.supports_chunks,
            OutputDeliveryKind::FinalMessage => self.supports_final_messages,
            OutputDeliveryKind::FinalValidatedOutput => self.supports_final_validated_outputs,
        }
    }

    pub fn supports_content_mode(&self, mode: OutputContentMode) -> bool {
        self.supported_content_modes.contains(&mode)
            && match mode {
                OutputContentMode::ContentRefsOnly => self.can_resolve_content_refs,
                OutputContentMode::RedactedSummary => true,
                OutputContentMode::RawContentIfPolicyAllows => self.may_receive_raw_content,
            }
    }
}

pub trait OutputSink: Send + Sync {
    fn sink_ref(&self) -> OutputSinkRef;
    fn capabilities(&self) -> OutputSinkCapabilities;

    fn send_chunk(
        &self,
        request: OutputDeliveryRequest,
    ) -> Result<OutputDeliveryReceipt, AgentError>;

    fn send_final(
        &self,
        request: OutputDeliveryRequest,
    ) -> Result<OutputDeliveryReceipt, AgentError>;

    fn reconcile(
        &self,
        request: OutputDeliveryRequest,
    ) -> Result<OutputDeliveryReceipt, AgentError>;
}

#[derive(Clone, Default)]
pub struct OutputSinkRegistry {
    sinks: BTreeMap<OutputSinkRef, Arc<dyn OutputSink>>,
}

impl OutputSinkRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<S>(&mut self, sink: S) -> Result<(), AgentError>
    where
        S: OutputSink + 'static,
    {
        let sink_ref = sink.sink_ref();
        if sink_ref.as_str().is_empty() {
            return Err(AgentError::missing_required_field("output_sink.sink_ref"));
        }
        self.sinks.insert(sink_ref, Arc::new(sink));
        Ok(())
    }

    pub fn register_arc(&mut self, sink: Arc<dyn OutputSink>) -> Result<(), AgentError> {
        let sink_ref = sink.sink_ref();
        if sink_ref.as_str().is_empty() {
            return Err(AgentError::missing_required_field("output_sink.sink_ref"));
        }
        self.sinks.insert(sink_ref, sink);
        Ok(())
    }

    pub fn get(&self, sink_ref: &OutputSinkRef) -> Option<Arc<dyn OutputSink>> {
        self.sinks.get(sink_ref).cloned()
    }

    pub fn first(&self) -> Option<Arc<dyn OutputSink>> {
        self.sinks.values().next().cloned()
    }

    pub fn is_empty(&self) -> bool {
        self.sinks.is_empty()
    }

    pub fn len(&self) -> usize {
        self.sinks.len()
    }
}
