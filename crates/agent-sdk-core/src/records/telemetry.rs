use serde::{Deserialize, Serialize};

use crate::{
    domain::{
        AgentId, AttemptId, DedupeKey, DestinationRef, EntityRef, EventId, JournalCursor,
        PolicyRef, PrivacyClass, RetentionClass, RunId, SourceRef, SpanId, TraceId, TurnId,
    },
    event::{EventCursor, EventFamily, EventKind},
    provider::ProviderUsage,
};

pub const TELEMETRY_SCHEMA_VERSION: u16 = 1;

macro_rules! telemetry_id {
    ($name:ident, $debug:literal) => {
        #[derive(Clone, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl core::fmt::Debug for $name {
            fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter.write_str(concat!($debug, "(redacted)"))
            }
        }

        impl core::fmt::Display for $name {
            fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter.write_str(concat!($debug, "(redacted)"))
            }
        }
    };
}

telemetry_id!(TelemetryProjectionId, "TelemetryProjectionId");
telemetry_id!(TelemetryRecordId, "TelemetryRecordId");
telemetry_id!(TelemetrySinkId, "TelemetrySinkId");
telemetry_id!(TelemetryExportAttemptId, "TelemetryExportAttemptId");
telemetry_id!(TelemetryUsageRecordId, "TelemetryUsageRecordId");
telemetry_id!(TelemetryCostRecordId, "TelemetryCostRecordId");
telemetry_id!(RateTableVersion, "RateTableVersion");

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TelemetrySinkKind {
    OpenTelemetry,
    DurableTrace,
    LocalDiagnostic,
    CliSummary,
    Test,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TelemetryContentCaptureMode {
    Off,
    MetadataOnly,
    RedactedSummary,
    PayloadRefs,
    RawContent,
}

impl TelemetryContentCaptureMode {
    pub fn captures_raw_content(&self) -> bool {
        matches!(self, Self::RawContent)
    }
}

impl Default for TelemetryContentCaptureMode {
    fn default() -> Self {
        Self::Off
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TelemetryProjectionKind {
    Progress,
    RunTerminal,
    Usage,
    CostEstimate,
    CostCorrection,
    SinkHealth,
    RepairCursor,
}

impl TelemetryProjectionKind {
    pub fn is_terminal_preserved(&self) -> bool {
        matches!(
            self,
            Self::RunTerminal
                | Self::Usage
                | Self::CostEstimate
                | Self::CostCorrection
                | Self::SinkHealth
                | Self::RepairCursor
        )
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TelemetryProjection {
    pub schema_version: u16,
    pub projection_id: TelemetryProjectionId,
    pub projection_kind: TelemetryProjectionKind,
    pub source_record: TelemetrySourceRecord,
    pub run_id: RunId,
    pub agent_id: AgentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<TurnId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt_id: Option<AttemptId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<EventId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub journal_cursor: Option<JournalCursor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<TraceId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span_id: Option<SpanId>,
    pub runtime_package_fingerprint: String,
    pub source: SourceRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<DestinationRef>,
    pub subject_ref: EntityRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policy_refs: Vec<PolicyRef>,
    pub privacy: PrivacyClass,
    pub retention: RetentionClass,
    pub content_capture: TelemetryContentCaptureMode,
    pub redaction_policy_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<UsageUnits>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<CostUnits>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_status: Option<TelemetryTerminalStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sink_health: Option<TelemetrySinkHealth>,
    pub redacted_summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_content: Option<String>,
}

impl TelemetryProjection {
    pub fn is_terminal_preserved(&self) -> bool {
        self.projection_kind.is_terminal_preserved()
    }

    pub fn without_raw_content(mut self) -> Self {
        self.raw_content = None;
        if self.content_capture.captures_raw_content() {
            self.content_capture = TelemetryContentCaptureMode::RedactedSummary;
        }
        self
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TelemetrySourceRecord {
    pub event_family: EventFamily,
    pub event_kind: EventKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_cursor: Option<EventCursor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_cursor: Option<TelemetrySourceCursor>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", content = "cursor", rename_all = "snake_case")]
pub enum TelemetrySourceCursor {
    Journal(JournalCursor),
    Event(EventCursor),
    Archive(String),
    Usage(TelemetryUsageRecordId),
    Cost(TelemetryCostRecordId),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TelemetryExportCursor {
    pub sink_id: TelemetrySinkId,
    pub export_seq: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_acknowledged_source: Option<TelemetrySourceCursor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_attempted_source: Option<TelemetrySourceCursor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sink_dedupe_key: Option<DedupeKey>,
}

impl TelemetryExportCursor {
    pub fn new(sink_id: TelemetrySinkId) -> Self {
        Self {
            sink_id,
            export_seq: 0,
            last_acknowledged_source: None,
            last_attempted_source: None,
            sink_dedupe_key: None,
        }
    }

    pub fn attempted(mut self, source: Option<TelemetrySourceCursor>) -> Self {
        self.last_attempted_source = source;
        self
    }

    pub fn acknowledged(mut self, source: Option<TelemetrySourceCursor>) -> Self {
        self.export_seq += 1;
        self.last_acknowledged_source = source;
        self.last_attempted_source = None;
        self
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct UsageUnits {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_duration_ms: Option<u64>,
}

impl From<ProviderUsage> for UsageUnits {
    fn from(usage: ProviderUsage) -> Self {
        Self {
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            total_tokens: usage.total_tokens,
            bytes: None,
            media_duration_ms: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CostUnits {
    pub amount_micros: u64,
    pub currency: String,
    pub rate_table_version: RateTableVersion,
    pub estimate_status: CostEstimateStatus,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CostEstimateStatus {
    Estimated,
    ProviderReported,
    Corrected,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TelemetryTerminalStatus {
    Completed,
    Failed,
    Cancelled,
    TimedOut,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TelemetrySinkHealth {
    pub sink_id: TelemetrySinkId,
    pub sink_kind: TelemetrySinkKind,
    pub state: TelemetrySinkHealthState,
    pub failure_kind: Option<TelemetrySinkFailureKind>,
    pub terminal_preserved: bool,
    pub dropped_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub export_cursor: Option<TelemetryExportCursor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unsafe_pending_reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TelemetrySinkHealthState {
    Healthy,
    Failed,
    Recovered,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TelemetrySinkFailureKind {
    Overflow,
    ExportRejected,
    Serialization,
    SchemaMismatch,
    SinkUnavailable,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TelemetryRecord {
    pub schema_version: u16,
    pub record_id: TelemetryRecordId,
    pub run_id: RunId,
    pub agent_id: AgentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_cursor: Option<TelemetrySourceCursor>,
    pub runtime_package_fingerprint: String,
    pub privacy: PrivacyClass,
    pub retention: RetentionClass,
    pub content_capture: TelemetryContentCaptureMode,
    pub redaction_policy_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policy_refs: Vec<PolicyRef>,
    pub payload: TelemetryRecordPayload,
}

impl TelemetryRecord {
    pub fn usage(
        record_id: TelemetryRecordId,
        projection: &TelemetryProjection,
        usage_record_id: TelemetryUsageRecordId,
    ) -> Self {
        Self::from_projection(
            record_id,
            projection,
            TelemetryRecordPayload::Usage(UsageTelemetryRecord {
                usage_record_id,
                units: projection.usage.clone().unwrap_or_default(),
                provider_id: projection.provider_id.clone(),
                model_id: projection.model_id.clone(),
            }),
        )
    }

    pub fn cost(
        record_id: TelemetryRecordId,
        projection: &TelemetryProjection,
        cost_record_id: TelemetryCostRecordId,
        correction_ref: Option<TelemetryCostRecordId>,
    ) -> Self {
        let payload = CostTelemetryRecord {
            cost_record_id,
            units: projection
                .cost
                .clone()
                .expect("cost projection has cost units"),
            correction_ref,
        };
        Self::from_projection(record_id, projection, TelemetryRecordPayload::Cost(payload))
    }

    pub fn sink_failed(
        record_id: TelemetryRecordId,
        projection: &TelemetryProjection,
        failure: TelemetrySinkFailureRecord,
    ) -> Self {
        Self::from_projection(
            record_id,
            projection,
            TelemetryRecordPayload::SinkFailed(failure),
        )
    }

    pub fn sink_recovered(
        record_id: TelemetryRecordId,
        projection: &TelemetryProjection,
        recovery: TelemetrySinkRecoveryRecord,
    ) -> Self {
        Self::from_projection(
            record_id,
            projection,
            TelemetryRecordPayload::SinkRecovered(recovery),
        )
    }

    pub fn export_cursor(
        record_id: TelemetryRecordId,
        projection: &TelemetryProjection,
        cursor: TelemetryExportCursor,
    ) -> Self {
        Self::from_projection(
            record_id,
            projection,
            TelemetryRecordPayload::ExportCursor(TelemetryExportCursorRecord { cursor }),
        )
    }

    fn from_projection(
        record_id: TelemetryRecordId,
        projection: &TelemetryProjection,
        payload: TelemetryRecordPayload,
    ) -> Self {
        Self {
            schema_version: TELEMETRY_SCHEMA_VERSION,
            record_id,
            run_id: projection.run_id.clone(),
            agent_id: projection.agent_id.clone(),
            source_cursor: projection.source_record.source_cursor.clone(),
            runtime_package_fingerprint: projection.runtime_package_fingerprint.clone(),
            privacy: projection.privacy.clone(),
            retention: projection.retention.clone(),
            content_capture: projection.content_capture.clone(),
            redaction_policy_id: projection.redaction_policy_id.clone(),
            policy_refs: projection.policy_refs.clone(),
            payload,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", content = "record", rename_all = "snake_case")]
pub enum TelemetryRecordPayload {
    Usage(UsageTelemetryRecord),
    Cost(CostTelemetryRecord),
    SinkFailed(TelemetrySinkFailureRecord),
    SinkRecovered(TelemetrySinkRecoveryRecord),
    ExportCursor(TelemetryExportCursorRecord),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct UsageTelemetryRecord {
    pub usage_record_id: TelemetryUsageRecordId,
    pub units: UsageUnits,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CostTelemetryRecord {
    pub cost_record_id: TelemetryCostRecordId,
    pub units: CostUnits,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correction_ref: Option<TelemetryCostRecordId>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TelemetrySinkFailureRecord {
    pub sink_id: TelemetrySinkId,
    pub sink_kind: TelemetrySinkKind,
    pub failure_kind: TelemetrySinkFailureKind,
    pub terminal_preserved: bool,
    pub dropped_count: u64,
    pub last_acknowledged_cursor: Option<TelemetryExportCursor>,
    pub repair_cursor: Option<TelemetrySourceCursor>,
    pub unsafe_pending_reason: Option<String>,
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TelemetrySinkRecoveryRecord {
    pub sink_id: TelemetrySinkId,
    pub sink_kind: TelemetrySinkKind,
    pub export_cursor: TelemetryExportCursor,
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TelemetryExportCursorRecord {
    pub cursor: TelemetryExportCursor,
}
