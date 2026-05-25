//! Durable and observable SDK records. Use these DTOs for events, journals, effects,
//! context, output, and feature evidence. Constructing records is data-only;
//! persistence, publication, and external actions happen through ports or application
//! coordinators. This file contains the telemetry portion of that contract.
//!
use serde::{Deserialize, Serialize};

use crate::{
    domain::{
        AgentId, AttemptId, DedupeKey, DestinationRef, EntityRef, EventId, JournalCursor,
        PolicyRef, PrivacyClass, RetentionClass, RunId, SourceRef, SpanId, TraceId, TurnId,
    },
    event::{EventCursor, EventFamily, EventKind},
    provider::ProviderUsage,
};

/// Constant value for the records::telemetry contract. Use it to keep
/// SDK records and tests aligned on the same stable value.
pub const TELEMETRY_SCHEMA_VERSION: u16 = 1;

macro_rules! telemetry_id {
    ($name:ident, $debug:literal) => {
        #[doc = concat!(
                            "Typed telemetry identifier for `",
                            stringify!($name),
                            "`. Use it for telemetry projection, export, usage, or cost records; ",
                            "constructing it is data-only and performs no side effects."
                        )]
        #[derive(Clone, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Creates a new records::telemetry value with explicit
            /// caller-provided inputs. This constructor is data-only
            /// and performs no I/O or external side effects.
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            /// Returns this value as str. The accessor is side-effect
            /// free and keeps ownership with the caller.
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
/// Enumerates the finite telemetry sink kind cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum TelemetrySinkKind {
    /// Use this variant when the contract needs to represent open telemetry; selecting it has no side effect by itself.
    OpenTelemetry,
    /// Use this variant when the contract needs to represent durable trace; selecting it has no side effect by itself.
    DurableTrace,
    /// Use this variant when the contract needs to represent local diagnostic; selecting it has no side effect by itself.
    LocalDiagnostic,
    /// Use this variant when the contract needs to represent cli summary; selecting it has no side effect by itself.
    CliSummary,
    /// Use this variant when the contract needs to represent test; selecting it has no side effect by itself.
    Test,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite telemetry content capture mode cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum TelemetryContentCaptureMode {
    /// Use this variant when the contract needs to represent off; selecting it has no side effect by itself.
    Off,
    /// Use this variant when the contract needs to represent metadata only; selecting it has no side effect by itself.
    MetadataOnly,
    /// Use this variant when the contract needs to represent redacted summary; selecting it has no side effect by itself.
    RedactedSummary,
    /// Use this variant when the contract needs to represent payload refs; selecting it has no side effect by itself.
    PayloadRefs,
    /// Use this variant when the contract needs to represent raw content; selecting it has no side effect by itself.
    RawContent,
}

impl TelemetryContentCaptureMode {
    /// Returns whether captures raw content applies for this contract.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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
/// Enumerates the finite telemetry projection kind cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum TelemetryProjectionKind {
    /// Use this variant when the contract needs to represent progress; selecting it has no side effect by itself.
    Progress,
    /// Use this variant when the contract needs to represent run terminal; selecting it has no side effect by itself.
    RunTerminal,
    /// Use this variant when the contract needs to represent usage; selecting it has no side effect by itself.
    Usage,
    /// Use this variant when the contract needs to represent cost estimate; selecting it has no side effect by itself.
    CostEstimate,
    /// Use this variant when the contract needs to represent cost correction; selecting it has no side effect by itself.
    CostCorrection,
    /// Use this variant when the contract needs to represent sink health; selecting it has no side effect by itself.
    SinkHealth,
    /// Use this variant when the contract needs to represent repair cursor; selecting it has no side effect by itself.
    RepairCursor,
}

impl TelemetryProjectionKind {
    /// Reports whether this value is terminal preserved. The check is
    /// pure and does not mutate SDK or host state.
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
/// Carries the telemetry projection record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct TelemetryProjection {
    /// Wire schema version used for compatibility checks.
    pub schema_version: u16,
    /// Stable projection id used for typed lineage, lookup, or dedupe.
    pub projection_id: TelemetryProjectionId,
    /// Projection controls for exposing data to a provider or subscriber.
    /// Use it to keep provider-visible data separate from private SDK state.
    pub projection_kind: TelemetryProjectionKind,
    /// Source record used by this record or request.
    pub source_record: TelemetrySourceRecord,
    /// Run identifier used for lineage, filtering, replay, and dedupe.
    pub run_id: RunId,
    /// Agent identifier used for lineage, filtering, and ownership checks.
    pub agent_id: AgentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Turn identifier for one loop turn within a run.
    pub turn_id: Option<TurnId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Attempt identifier for retry, repair, provider, or tool execution
    /// evidence.
    pub attempt_id: Option<AttemptId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Event identifier used to correlate live events with journal or replay
    /// evidence.
    pub event_id: Option<EventId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Cursor identifying a replay, export, or subscription position.
    /// Use it to resume without widening the original scope.
    pub journal_cursor: Option<JournalCursor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stable trace id used for typed lineage, lookup, or dedupe.
    pub trace_id: Option<TraceId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stable span id used for typed lineage, lookup, or dedupe.
    pub span_id: Option<SpanId>,
    /// Fingerprint of the runtime package snapshot in force when this value was produced.
    /// Use it for replay, dedupe, and package-lineage checks; the field is evidence and does
    /// not execute package behavior.
    pub runtime_package_fingerprint: String,
    /// Source label or ref for this item; it is metadata and does not fetch
    /// content by itself.
    pub source: SourceRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Destination label or ref for this item; it is metadata and does not
    /// deliver content by itself.
    pub destination: Option<DestinationRef>,
    /// Typed subject ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub subject_ref: EntityRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy: PrivacyClass,
    /// Retention class used by hosts and sinks when storing or exporting this
    /// item.
    pub retention: RetentionClass,
    /// Content capture used by this record or request.
    pub content_capture: TelemetryContentCaptureMode,
    /// Stable redaction policy id used for typed lineage, lookup, or dedupe.
    pub redaction_policy_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stable provider id used for typed lineage, lookup, or dedupe.
    pub provider_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stable model id used for typed lineage, lookup, or dedupe.
    pub model_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional tool name value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional usage value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub usage: Option<UsageUnits>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional cost value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub cost: Option<CostUnits>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional terminal status value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub terminal_status: Option<TelemetryTerminalStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional sink health value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub sink_health: Option<TelemetrySinkHealth>,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Raw content or raw-content control for this value.
    /// Use it only when policy explicitly allows raw content capture or delivery.
    pub raw_content: Option<String>,
}

impl TelemetryProjection {
    /// Reports whether this value is terminal preserved. The check is
    /// pure and does not mutate SDK or host state.
    pub fn is_terminal_preserved(&self) -> bool {
        self.projection_kind.is_terminal_preserved()
    }

    /// Returns an updated value with without raw content configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn without_raw_content(mut self) -> Self {
        self.raw_content = None;
        if self.content_capture.captures_raw_content() {
            self.content_capture = TelemetryContentCaptureMode::RedactedSummary;
        }
        self
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the telemetry source record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct TelemetrySourceRecord {
    /// Event family used by this record or request.
    pub event_family: EventFamily,
    /// Kind discriminator for event kind.
    /// Use it to route finite match arms without parsing display text.
    pub event_kind: EventKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Cursor identifying a replay, export, or subscription position.
    /// Use it to resume without widening the original scope.
    pub event_cursor: Option<EventCursor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Cursor identifying the source event or journal position.
    /// Use it to connect projections back to durable evidence.
    pub source_cursor: Option<TelemetrySourceCursor>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", content = "cursor", rename_all = "snake_case")]
/// Enumerates the finite telemetry source cursor cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum TelemetrySourceCursor {
    /// Use this variant when the contract needs to represent journal; selecting it has no side effect by itself.
    Journal(JournalCursor),
    /// Use this variant when the contract needs to represent event; selecting it has no side effect by itself.
    Event(EventCursor),
    /// Use this variant when the contract needs to represent archive; selecting it has no side effect by itself.
    Archive(String),
    /// Use this variant when the contract needs to represent usage; selecting it has no side effect by itself.
    Usage(TelemetryUsageRecordId),
    /// Use this variant when the contract needs to represent cost; selecting it has no side effect by itself.
    Cost(TelemetryCostRecordId),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the telemetry export cursor record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct TelemetryExportCursor {
    /// Stable sink id used for typed lineage, lookup, or dedupe.
    pub sink_id: TelemetrySinkId,
    /// Export seq used by this record or request.
    pub export_seq: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional last acknowledged source value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub last_acknowledged_source: Option<TelemetrySourceCursor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Attempt identifier or attempt history for bounded retry/repair.
    /// Use it to preserve ordering and avoid retry loops that cannot be audited.
    pub last_attempted_source: Option<TelemetrySourceCursor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Dedupe policy or key for a side-effecting operation.
    /// Replay and repair use it to avoid sending or executing the same effect twice.
    pub sink_dedupe_key: Option<DedupeKey>,
}

impl TelemetryExportCursor {
    /// Creates a new records::telemetry value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new(sink_id: TelemetrySinkId) -> Self {
        Self {
            sink_id,
            export_seq: 0,
            last_acknowledged_source: None,
            last_attempted_source: None,
            sink_dedupe_key: None,
        }
    }

    /// Returns an updated value with attempted configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn attempted(mut self, source: Option<TelemetrySourceCursor>) -> Self {
        self.last_attempted_source = source;
        self
    }

    /// Returns an updated value with acknowledged configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn acknowledged(mut self, source: Option<TelemetrySourceCursor>) -> Self {
        self.export_seq += 1;
        self.last_acknowledged_source = source;
        self.last_attempted_source = None;
        self
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the usage units record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct UsageUnits {
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional input tokens value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub input_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional output tokens value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional total tokens value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub total_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Byte size or byte limit for bytes.
    /// Use it to enforce bounded reads, writes, summaries, or parser output.
    pub bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// media duration ms duration in milliseconds.
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
/// Carries the cost units record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct CostUnits {
    /// Cost amount expressed in millionths of the currency unit.
    /// Use micros to keep cost accounting deterministic and integer-based.
    pub amount_micros: u64,
    /// Currency code for the cost amount.
    /// Cost accounting uses it with amount micros and rate-table version.
    pub currency: String,
    /// Version of the rate table used for cost estimation.
    /// This distinguishes estimated cost from provider-reported billing.
    pub rate_table_version: RateTableVersion,
    /// Whether cost or usage values are estimated or provider-reported.
    /// Use it to avoid treating estimates as final billing truth.
    pub estimate_status: CostEstimateStatus,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite cost estimate status cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum CostEstimateStatus {
    /// Use this variant when the contract needs to represent estimated; selecting it has no side effect by itself.
    Estimated,
    /// Use this variant when the contract needs to represent provider reported; selecting it has no side effect by itself.
    ProviderReported,
    /// Use this variant when the contract needs to represent corrected; selecting it has no side effect by itself.
    Corrected,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite telemetry terminal status cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum TelemetryTerminalStatus {
    /// Use this variant when the contract needs to represent completed; selecting it has no side effect by itself.
    Completed,
    /// Use this variant when the contract needs to represent failed; selecting it has no side effect by itself.
    Failed,
    /// Use this variant when the contract needs to represent cancelled; selecting it has no side effect by itself.
    Cancelled,
    /// Use this variant when the contract needs to represent timed out; selecting it has no side effect by itself.
    TimedOut,
    /// Use this variant when the contract needs to represent unknown; selecting it has no side effect by itself.
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the telemetry sink health record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct TelemetrySinkHealth {
    /// Stable sink id used for typed lineage, lookup, or dedupe.
    pub sink_id: TelemetrySinkId,
    /// Kind discriminator for sink kind.
    /// Use it to route finite match arms without parsing display text.
    pub sink_kind: TelemetrySinkKind,
    /// State used by this record or request.
    pub state: TelemetrySinkHealthState,
    /// Kind discriminator for failure kind.
    /// Use it to route finite match arms without parsing display text.
    pub failure_kind: Option<TelemetrySinkFailureKind>,
    /// Whether terminal preserved is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub terminal_preserved: bool,
    /// Count of dropped items observed or included in this record.
    pub dropped_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Cursor for telemetry export acknowledgement.
    /// Sinks use it to ack exactly which projection was exported.
    pub export_cursor: Option<TelemetryExportCursor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Reason a pending side effect is unsafe to retry automatically.
    /// Recovery uses it to require repair or reconciliation before continuing.
    pub unsafe_pending_reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite telemetry sink health state cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum TelemetrySinkHealthState {
    /// Use this variant when the contract needs to represent healthy; selecting it has no side effect by itself.
    Healthy,
    /// Use this variant when the contract needs to represent failed; selecting it has no side effect by itself.
    Failed,
    /// Use this variant when the contract needs to represent recovered; selecting it has no side effect by itself.
    Recovered,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite telemetry sink failure kind cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum TelemetrySinkFailureKind {
    /// Use this variant when the contract needs to represent overflow; selecting it has no side effect by itself.
    Overflow,
    /// Use this variant when the contract needs to represent export rejected; selecting it has no side effect by itself.
    ExportRejected,
    /// Use this variant when the contract needs to represent serialization; selecting it has no side effect by itself.
    Serialization,
    /// Use this variant when the contract needs to represent schema mismatch; selecting it has no side effect by itself.
    SchemaMismatch,
    /// Use this variant when the contract needs to represent sink unavailable; selecting it has no side effect by itself.
    SinkUnavailable,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the telemetry record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct TelemetryRecord {
    /// Wire schema version used for compatibility checks.
    pub schema_version: u16,
    /// Stable record id used for typed lineage, lookup, or dedupe.
    pub record_id: TelemetryRecordId,
    /// Run identifier used for lineage, filtering, replay, and dedupe.
    pub run_id: RunId,
    /// Agent identifier used for lineage, filtering, and ownership checks.
    pub agent_id: AgentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Cursor identifying the source event or journal position.
    /// Use it to connect projections back to durable evidence.
    pub source_cursor: Option<TelemetrySourceCursor>,
    /// Fingerprint of the runtime package snapshot in force when this value was produced.
    /// Use it for replay, dedupe, and package-lineage checks; the field is evidence and does
    /// not execute package behavior.
    pub runtime_package_fingerprint: String,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy: PrivacyClass,
    /// Retention class used by hosts and sinks when storing or exporting this
    /// item.
    pub retention: RetentionClass,
    /// Content capture used by this record or request.
    pub content_capture: TelemetryContentCaptureMode,
    /// Stable redaction policy id used for typed lineage, lookup, or dedupe.
    pub redaction_policy_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
    /// Payload carried by this record.
    /// Use the surrounding policy and redaction fields to decide whether it can be exposed.
    pub payload: TelemetryRecordPayload,
}

impl TelemetryRecord {
    /// Returns usage derived from the supplied state.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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

    /// Returns cost derived from the supplied state.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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

    /// Returns an updated value with sink failed configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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

    /// Returns an updated value with sink recovered configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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

    /// Returns an updated value with export cursor configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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
/// Enumerates the finite telemetry record payload cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum TelemetryRecordPayload {
    /// Use this variant when the contract needs to represent usage; selecting it has no side effect by itself.
    Usage(UsageTelemetryRecord),
    /// Use this variant when the contract needs to represent cost; selecting it has no side effect by itself.
    Cost(CostTelemetryRecord),
    /// Use this variant when the contract needs to represent sink failed; selecting it has no side effect by itself.
    SinkFailed(TelemetrySinkFailureRecord),
    /// Use this variant when the contract needs to represent sink recovered; selecting it has no side effect by itself.
    SinkRecovered(TelemetrySinkRecoveryRecord),
    /// Use this variant when the contract needs to represent export cursor; selecting it has no side effect by itself.
    ExportCursor(TelemetryExportCursorRecord),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the usage telemetry record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct UsageTelemetryRecord {
    /// Stable usage record id used for typed lineage, lookup, or dedupe.
    pub usage_record_id: TelemetryUsageRecordId,
    /// Units used by this record or request.
    pub units: UsageUnits,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stable provider id used for typed lineage, lookup, or dedupe.
    pub provider_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stable model id used for typed lineage, lookup, or dedupe.
    pub model_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the cost telemetry record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct CostTelemetryRecord {
    /// Stable cost record id used for typed lineage, lookup, or dedupe.
    pub cost_record_id: TelemetryCostRecordId,
    /// Units used by this record or request.
    pub units: CostUnits,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Typed correction ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub correction_ref: Option<TelemetryCostRecordId>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the telemetry sink failure record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct TelemetrySinkFailureRecord {
    /// Stable sink id used for typed lineage, lookup, or dedupe.
    pub sink_id: TelemetrySinkId,
    /// Kind discriminator for sink kind.
    /// Use it to route finite match arms without parsing display text.
    pub sink_kind: TelemetrySinkKind,
    /// Kind discriminator for failure kind.
    /// Use it to route finite match arms without parsing display text.
    pub failure_kind: TelemetrySinkFailureKind,
    /// Whether terminal preserved is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub terminal_preserved: bool,
    /// Count of dropped items observed or included in this record.
    pub dropped_count: u64,
    /// Last telemetry export cursor acknowledged by the sink.
    /// Repair uses it to resume after the last confirmed export.
    pub last_acknowledged_cursor: Option<TelemetryExportCursor>,
    /// Cursor where repair or reconciliation should resume.
    /// Use it to continue recovery without replaying unrelated records.
    pub repair_cursor: Option<TelemetrySourceCursor>,
    /// Reason a pending side effect is unsafe to retry automatically.
    /// Recovery uses it to require repair or reconciliation before continuing.
    pub unsafe_pending_reason: Option<String>,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the telemetry sink recovery record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct TelemetrySinkRecoveryRecord {
    /// Stable sink id used for typed lineage, lookup, or dedupe.
    pub sink_id: TelemetrySinkId,
    /// Kind discriminator for sink kind.
    /// Use it to route finite match arms without parsing display text.
    pub sink_kind: TelemetrySinkKind,
    /// Cursor for telemetry export acknowledgement.
    /// Sinks use it to ack exactly which projection was exported.
    pub export_cursor: TelemetryExportCursor,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the telemetry export cursor record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct TelemetryExportCursorRecord {
    /// Cursor identifying a replay, export, or subscription position.
    /// Use it to resume without widening the original scope.
    pub cursor: TelemetryExportCursor,
}
