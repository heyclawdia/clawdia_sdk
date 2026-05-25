//! Output sink port boundary. Hosts implement this module to deliver output to CLI,
//! desktop, webhook, file, or remote channels. Sink calls may perform external I/O
//! and must honor declared content modes.
//!
use std::{collections::BTreeMap, sync::Arc};

use crate::{
    domain::AgentError,
    output_delivery::{
        OutputContentMode, OutputDeliveryKind, OutputDeliveryReceipt, OutputDeliveryRequest,
        OutputSinkRef,
    },
};

#[derive(Clone, Debug, Eq, PartialEq)]
/// Carries output sink capabilities data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct OutputSinkCapabilities {
    /// Typed sink ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub sink_ref: OutputSinkRef,
    /// Capability version advertised by the provider or package.
    /// Use it to match compatible feature contracts during package resolution.
    pub capability_version: String,
    /// Boolean policy/capability flag for whether supports chunks is enabled.
    pub supports_chunks: bool,
    /// Boolean policy/capability flag for whether supports final messages is
    /// enabled.
    pub supports_final_messages: bool,
    /// Whether the sink can receive final validated-output payloads.
    /// When false, delivery policy should fall back to supported chunk or final-message modes
    /// instead of sending typed output.
    pub supports_final_validated_outputs: bool,
    /// Collection of supported content modes values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub supported_content_modes: Vec<OutputContentMode>,
    /// Typed can resolve content refs references. Resolving them is separate
    /// from constructing this record.
    pub can_resolve_content_refs: bool,
    /// Raw content or raw-content control for this value.
    /// Use it only when policy explicitly allows raw content capture or delivery.
    pub may_receive_raw_content: bool,
}

impl OutputSinkCapabilities {
    /// Builds the refs and summaries value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
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

    /// Returns this value with its raw content setting replaced. The
    /// method follows builder-style data construction and does not
    /// execute external work.
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

    /// Reads the stored supports kind without registry or runtime work.
    /// This reads sink registry metadata or builds sink policy data without sending output.
    pub fn supports_kind(&self, kind: &OutputDeliveryKind) -> bool {
        match kind {
            OutputDeliveryKind::StreamChunk { .. } => self.supports_chunks,
            OutputDeliveryKind::FinalMessage => self.supports_final_messages,
            OutputDeliveryKind::FinalValidatedOutput => self.supports_final_validated_outputs,
        }
    }

    /// Reads the stored supports content mode without registry or runtime work.
    /// This reads sink registry metadata or builds sink policy data without sending output.
    pub fn supports_content_mode(&self, mode: OutputContentMode) -> bool {
        self.supported_content_modes.contains(&mode)
            && match mode {
                OutputContentMode::ContentRefsOnly => self.can_resolve_content_refs,
                OutputContentMode::RedactedSummary => true,
                OutputContentMode::RawContentIfPolicyAllows => self.may_receive_raw_content,
            }
    }
}

/// Port or behavior contract for output sink. Implementors should
/// preserve policy, redaction, idempotency, and replay expectations
/// from the surrounding module. Implementations may perform side
/// effects only as described by the trait methods.
pub trait OutputSink: Send + Sync {
    /// Returns sink ref for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    fn sink_ref(&self) -> OutputSinkRef;
    /// Computes or returns capabilities for the ports::output_delivery
    /// contract without external I/O or side effects.
    fn capabilities(&self) -> OutputSinkCapabilities;

    /// Sends one output chunk to the configured sink.
    /// Implementations may perform sink I/O, but the runtime owns delivery
    /// policy checks, dedupe proof, and intent/result journal records.
    fn send_chunk(
        &self,
        request: OutputDeliveryRequest,
    ) -> Result<OutputDeliveryReceipt, AgentError>;

    /// Sends the final output payload to the configured sink.
    /// Implementations may perform sink I/O, but the runtime owns delivery
    /// policy checks, dedupe proof, and intent/result journal records.
    fn send_final(
        &self,
        request: OutputDeliveryRequest,
    ) -> Result<OutputDeliveryReceipt, AgentError>;

    /// Reconciles prior delivery state with the sink without resending completed
    /// payloads.
    /// Implementations reconcile prior delivery state with the sink without resending completed
    /// payloads.
    fn reconcile(
        &self,
        request: OutputDeliveryRequest,
    ) -> Result<OutputDeliveryReceipt, AgentError>;
}

#[derive(Clone, Default)]
/// Carries output sink registry data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct OutputSinkRegistry {
    sinks: BTreeMap<OutputSinkRef, Arc<dyn OutputSink>>,
}

impl OutputSinkRegistry {
    /// Creates a new ports::output_delivery value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds data to this in-memory ports::output_delivery collection. It does
    /// not perform external I/O, execute tools, or append journals.
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

    /// Returns an updated value with register arc configured.
    /// This is sink policy or capability metadata construction and does not send output.
    pub fn register_arc(&mut self, sink: Arc<dyn OutputSink>) -> Result<(), AgentError> {
        let sink_ref = sink.sink_ref();
        if sink_ref.as_str().is_empty() {
            return Err(AgentError::missing_required_field("output_sink.sink_ref"));
        }
        self.sinks.insert(sink_ref, sink);
        Ok(())
    }

    /// Looks up an entry in this local store without registry or runtime work.
    /// This reads sink registry metadata or builds sink policy data without sending output.
    pub fn get(&self, sink_ref: &OutputSinkRef) -> Option<Arc<dyn OutputSink>> {
        self.sinks.get(sink_ref).cloned()
    }

    /// Returns first for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn first(&self) -> Option<Arc<dyn OutputSink>> {
        self.sinks.values().next().cloned()
    }

    /// Reports whether this value is empty. The check is pure and does
    /// not mutate SDK or host state.
    pub fn is_empty(&self) -> bool {
        self.sinks.is_empty()
    }

    /// Reads the stored len without registry or runtime work.
    /// This reads sink registry metadata or builds sink policy data without sending output.
    pub fn len(&self) -> usize {
        self.sinks.len()
    }
}
