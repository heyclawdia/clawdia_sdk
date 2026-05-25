//! Host adapter boundaries for the SDK core. Use these traits and registries when
//! hosts provide providers, journals, sinks, tools, isolation, extensions, telemetry,
//! or subscriptions. Implementations may perform external side effects and must honor
//! policy, redaction, idempotency, and replay contracts. This file contains the
//! telemetry portion of that contract.
//!
use std::num::NonZeroUsize;

use serde::{Deserialize, Serialize};

use crate::{
    domain::AgentError,
    telemetry_records::{
        TelemetryContentCaptureMode, TelemetryExportCursor, TelemetryProjection, TelemetrySinkId,
        TelemetrySinkKind,
    },
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries telemetry sink spec data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct TelemetrySinkSpec {
    /// Stable sink id used for typed lineage, lookup, or dedupe.
    pub sink_id: TelemetrySinkId,
    /// Kind discriminator for sink kind.
    /// Use it to route finite match arms without parsing display text.
    pub sink_kind: TelemetrySinkKind,
    /// Content capture used by this record or request.
    pub content_capture: TelemetryContentCaptureMode,
    /// Queue capacity used by this record or request.
    pub queue_capacity: NonZeroUsize,
    /// Queue slots reserved for terminal frames.
    /// This keeps important terminal events available even when non-terminal frames overflow.
    pub terminal_reserve: NonZeroUsize,
    /// Boolean policy/capability flag for whether requires idempotent replay
    /// is enabled.
    pub requires_idempotent_replay: bool,
}

impl TelemetrySinkSpec {
    /// Builds the safe local diagnostic value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn safe_local_diagnostic(sink_id: impl Into<String>) -> Self {
        Self {
            sink_id: TelemetrySinkId::new(sink_id),
            sink_kind: TelemetrySinkKind::LocalDiagnostic,
            content_capture: TelemetryContentCaptureMode::Off,
            queue_capacity: NonZeroUsize::new(64).expect("nonzero queue capacity"),
            terminal_reserve: NonZeroUsize::new(4).expect("nonzero terminal reserve"),
            requires_idempotent_replay: true,
        }
    }

    /// Builds the test value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn test(sink_id: impl Into<String>, queue_capacity: NonZeroUsize) -> Self {
        Self {
            sink_id: TelemetrySinkId::new(sink_id),
            sink_kind: TelemetrySinkKind::Test,
            content_capture: TelemetryContentCaptureMode::Off,
            queue_capacity,
            terminal_reserve: NonZeroUsize::new(1).expect("nonzero terminal reserve"),
            requires_idempotent_replay: true,
        }
    }

    /// Returns this value with its content capture setting replaced.
    /// The method follows builder-style data construction and does not
    /// execute external work.
    pub fn with_content_capture(mut self, mode: TelemetryContentCaptureMode) -> Self {
        self.content_capture = mode;
        self
    }

    /// Returns this value with its terminal reserve setting replaced.
    /// The method follows builder-style data construction and does not
    /// execute external work.
    pub fn with_terminal_reserve(mut self, terminal_reserve: NonZeroUsize) -> Self {
        self.terminal_reserve = terminal_reserve;
        self
    }
}

/// Port or behavior contract for telemetry sink. Implementors should
/// preserve policy, redaction, idempotency, and replay expectations
/// from the surrounding module. Implementations may perform side
/// effects only as described by the trait methods.
pub trait TelemetrySink: Send + Sync {
    /// Returns spec for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    fn spec(&self) -> &TelemetrySinkSpec;

    /// Exports one telemetry projection at the supplied export cursor.
    /// Implementations may write to a telemetry backend, but must not mutate
    /// run state, decide policy, or request raw content beyond the projection.
    fn export(
        &self,
        projection: &TelemetryProjection,
        cursor: &TelemetryExportCursor,
    ) -> Result<TelemetrySinkAck, TelemetrySinkError>;
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries telemetry sink ack data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct TelemetrySinkAck {
    /// Cursor identifying a replay, export, or subscription position.
    /// Use it to resume without widening the original scope.
    pub cursor: TelemetryExportCursor,
    /// Whether accepted duplicate is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub accepted_duplicate: bool,
}

impl TelemetrySinkAck {
    /// Returns accepted for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn accepted(cursor: TelemetryExportCursor) -> Self {
        Self {
            cursor,
            accepted_duplicate: false,
        }
    }

    /// Returns duplicate for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn duplicate(cursor: TelemetryExportCursor) -> Self {
        Self {
            cursor,
            accepted_duplicate: true,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries telemetry sink error data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct TelemetrySinkError {
    /// Kind discriminator for failure kind.
    /// Use it to route finite match arms without parsing display text.
    pub failure_kind: crate::telemetry_records::TelemetrySinkFailureKind,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
    /// Reason a pending side effect is unsafe to retry automatically.
    /// Recovery uses it to require repair or reconciliation before continuing.
    pub unsafe_pending_reason: Option<String>,
}

impl TelemetrySinkError {
    /// Returns unavailable for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn unavailable(summary: impl Into<String>) -> Self {
        Self {
            failure_kind: crate::telemetry_records::TelemetrySinkFailureKind::SinkUnavailable,
            redacted_summary: summary.into(),
            unsafe_pending_reason: None,
        }
    }

    /// Returns schema mismatch for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn schema_mismatch(summary: impl Into<String>) -> Self {
        Self {
            failure_kind: crate::telemetry_records::TelemetrySinkFailureKind::SchemaMismatch,
            redacted_summary: summary.into(),
            unsafe_pending_reason: None,
        }
    }

    /// Converts this value into agent error data.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn into_agent_error(self) -> AgentError {
        AgentError::new(
            crate::domain::AgentErrorKind::TelemetryFailure,
            crate::domain::RetryClassification::RepairNeeded,
            self.redacted_summary,
        )
    }
}
