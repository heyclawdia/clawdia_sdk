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
pub struct TelemetrySinkSpec {
    pub sink_id: TelemetrySinkId,
    pub sink_kind: TelemetrySinkKind,
    pub content_capture: TelemetryContentCaptureMode,
    pub queue_capacity: NonZeroUsize,
    pub terminal_reserve: NonZeroUsize,
    pub requires_idempotent_replay: bool,
}

impl TelemetrySinkSpec {
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

    pub fn with_content_capture(mut self, mode: TelemetryContentCaptureMode) -> Self {
        self.content_capture = mode;
        self
    }

    pub fn with_terminal_reserve(mut self, terminal_reserve: NonZeroUsize) -> Self {
        self.terminal_reserve = terminal_reserve;
        self
    }
}

pub trait TelemetrySink: Send + Sync {
    fn spec(&self) -> &TelemetrySinkSpec;

    fn export(
        &self,
        projection: &TelemetryProjection,
        cursor: &TelemetryExportCursor,
    ) -> Result<TelemetrySinkAck, TelemetrySinkError>;
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TelemetrySinkAck {
    pub cursor: TelemetryExportCursor,
    pub accepted_duplicate: bool,
}

impl TelemetrySinkAck {
    pub fn accepted(cursor: TelemetryExportCursor) -> Self {
        Self {
            cursor,
            accepted_duplicate: false,
        }
    }

    pub fn duplicate(cursor: TelemetryExportCursor) -> Self {
        Self {
            cursor,
            accepted_duplicate: true,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TelemetrySinkError {
    pub failure_kind: crate::telemetry_records::TelemetrySinkFailureKind,
    pub redacted_summary: String,
    pub unsafe_pending_reason: Option<String>,
}

impl TelemetrySinkError {
    pub fn unavailable(summary: impl Into<String>) -> Self {
        Self {
            failure_kind: crate::telemetry_records::TelemetrySinkFailureKind::SinkUnavailable,
            redacted_summary: summary.into(),
            unsafe_pending_reason: None,
        }
    }

    pub fn schema_mismatch(summary: impl Into<String>) -> Self {
        Self {
            failure_kind: crate::telemetry_records::TelemetrySinkFailureKind::SchemaMismatch,
            redacted_summary: summary.into(),
            unsafe_pending_reason: None,
        }
    }

    pub fn into_agent_error(self) -> AgentError {
        AgentError::new(
            crate::domain::AgentErrorKind::TelemetryFailure,
            crate::domain::RetryClassification::RepairNeeded,
            self.redacted_summary,
        )
    }
}
