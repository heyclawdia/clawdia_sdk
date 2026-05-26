//! Host adapter boundaries for the SDK core. Use these traits and registries when
//! hosts provide providers, journals, sinks, tools, isolation, extensions, telemetry,
//! or subscriptions. Implementations may perform external side effects and must honor
//! policy, redaction, idempotency, and replay contracts. This file contains the
//! providers portion of that contract.
//!
pub use crate::provider::{
    ProviderAdapter, ProviderCapabilities, ProviderConformanceCase, ProviderMessage,
    ProviderMessageRole, ProviderModality, ProviderProjectedMetadata, ProviderProjectionPolicy,
    ProviderRequest, ProviderResponse, ProviderStopReason, ProviderStreamChunk,
    ProviderStreamDelta, ProviderToolCall, ProviderUsage,
};
