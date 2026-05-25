//! Durable and observable SDK records. Use these DTOs for events, journals, effects,
//! context, output, and feature evidence. Constructing records is data-only;
//! persistence, publication, and external actions happen through ports or application
//! coordinators. This file contains the realtime portion of that contract.
//!
use core::fmt;

use serde::{Deserialize, Deserializer, Serialize, de::Error as DeError};

use crate::{
    domain::{
        AgentId, ContentRef, EffectId, IdValidationError, PolicyRef, PrivacyClass, RetentionClass,
        RunId,
    },
    effect::{EffectIntent, EffectResult},
    ids::validate_identifier,
    stream_records::{StreamChannel, StreamCursor, StreamDirection},
};

macro_rules! typed_realtime_id {
    ($name:ident) => {
        #[doc = concat!(
                            "Typed realtime identifier for `",
                            stringify!($name),
                            "`. Use it to correlate realtime sessions, frames, and responses; ",
                            "constructing it is data-only and performs no side effects."
                        )]
        #[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Creates a new records::realtime value with explicit
            /// caller-provided inputs. This constructor is data-only
            /// and performs no I/O or external side effects.
            ///
            /// # Panics
            ///
            /// Panics if constructor invariants fail, such as invalid identifier
            /// text or constructor-specific bounds. Use a fallible constructor such as
            /// `try_new` when one is available for untrusted input.
            pub fn new(value: impl Into<String>) -> Self {
                Self::try_new(value).expect(concat!(stringify!($name), " must be valid"))
            }

            /// Creates a new records::realtime value after validation.
            /// Returns an SDK error instead of panicking when the
            /// identifier or input does not satisfy the contract.
            pub fn try_new(value: impl Into<String>) -> Result<Self, IdValidationError> {
                let value = value.into();
                validate_identifier(&value)?;
                Ok(Self(value))
            }

            /// Returns this value as str. The accessor is side-effect
            /// free and keeps ownership with the caller.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self::new(value)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::try_new(value).map_err(D::Error::custom)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(concat!(stringify!($name), "(redacted)"))
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(concat!(stringify!($name), "(redacted)"))
            }
        }
    };
}

typed_realtime_id!(RealtimeSessionId);
typed_realtime_id!(RealtimeConnectionId);
typed_realtime_id!(RealtimeResponseId);
typed_realtime_id!(RealtimeFrameId);

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite realtime media kind cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum RealtimeMediaKind {
    /// Use this variant when the contract needs to represent audio; selecting it has no side effect by itself.
    Audio,
    /// Use this variant when the contract needs to represent text; selecting it has no side effect by itself.
    Text,
    /// Use this variant when the contract needs to represent image; selecting it has no side effect by itself.
    Image,
    /// Use this variant when the contract needs to represent video; selecting it has no side effect by itself.
    Video,
    /// Use this variant when the contract needs to represent transcript; selecting it has no side effect by itself.
    Transcript,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite realtime session status cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum RealtimeSessionStatus {
    /// Use this variant when the contract needs to represent connecting; selecting it has no side effect by itself.
    Connecting,
    /// Use this variant when the contract needs to represent connected; selecting it has no side effect by itself.
    Connected,
    /// Use this variant when the contract needs to represent input sent; selecting it has no side effect by itself.
    InputSent,
    /// Use this variant when the contract needs to represent output received; selecting it has no side effect by itself.
    OutputReceived,
    /// Use this variant when the contract needs to represent interrupted; selecting it has no side effect by itself.
    Interrupted,
    /// Use this variant when the contract needs to represent restart requested; selecting it has no side effect by itself.
    RestartRequested,
    /// Use this variant when the contract needs to represent restart started; selecting it has no side effect by itself.
    RestartStarted,
    /// Use this variant when the contract needs to represent restart completed; selecting it has no side effect by itself.
    RestartCompleted,
    /// Use this variant when the contract needs to represent restart failed; selecting it has no side effect by itself.
    RestartFailed,
    /// Use this variant when the contract needs to represent closed; selecting it has no side effect by itself.
    Closed,
    /// Use this variant when the contract needs to represent failed; selecting it has no side effect by itself.
    Failed,
    /// Use this variant when the contract needs to represent detached; selecting it has no side effect by itself.
    Detached,
    /// Use this variant when the contract needs to represent backpressure applied; selecting it has no side effect by itself.
    BackpressureApplied,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite realtime close reason cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum RealtimeCloseReason {
    /// Use this variant when the contract needs to represent normal; selecting it has no side effect by itself.
    Normal,
    /// Use this variant when the contract needs to represent cancel; selecting it has no side effect by itself.
    Cancel,
    /// Use this variant when the contract needs to represent provider failure; selecting it has no side effect by itself.
    ProviderFailure,
    /// Use this variant when the contract needs to represent policy denial; selecting it has no side effect by itself.
    PolicyDenial,
    /// Use this variant when the contract needs to represent host owned detach; selecting it has no side effect by itself.
    HostOwnedDetach,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite realtime backpressure action cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum RealtimeBackpressureAction {
    /// Use this variant when the contract needs to represent block; selecting it has no side effect by itself.
    Block,
    /// Use this variant when the contract needs to represent gate; selecting it has no side effect by itself.
    Gate,
    /// Use this variant when the contract needs to represent drop noncritical; selecting it has no side effect by itself.
    DropNoncritical,
    /// Use this variant when the contract needs to represent summarize; selecting it has no side effect by itself.
    Summarize,
    /// Use this variant when the contract needs to represent fail send; selecting it has no side effect by itself.
    FailSend,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the realtime backpressure state record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct RealtimeBackpressureState {
    /// Total subscriber queue capacity.
    /// This bounds buffered event frames for a live subscriber.
    pub capacity: usize,
    /// Queued frames used by this record or request.
    pub queued_frames: usize,
    /// Dropped frames used by this record or request.
    pub dropped_frames: usize,
    /// Gated frames used by this record or request.
    pub gated_frames: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional last action value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub last_action: Option<RealtimeBackpressureAction>,
    /// Policy reference that must be resolved by the host or runtime before
    /// execution.
    pub policy_ref: PolicyRef,
}

impl RealtimeBackpressureState {
    /// Builds the bounded value with the documented defaults.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn bounded(capacity: usize, policy_ref: PolicyRef) -> Self {
        Self {
            capacity,
            queued_frames: 0,
            dropped_frames: 0,
            gated_frames: 0,
            last_action: None,
            policy_ref,
        }
    }

    /// Returns an updated value with gate configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn gate(mut self) -> Self {
        self.gated_frames += 1;
        self.last_action = Some(RealtimeBackpressureAction::Gate);
        self
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the realtime input frame record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct RealtimeInputFrame {
    /// Stable frame id used for typed lineage, lookup, or dedupe.
    pub frame_id: RealtimeFrameId,
    /// Kind discriminator for media kind.
    /// Use it to route finite match arms without parsing display text.
    pub media_kind: RealtimeMediaKind,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Content references associated with this record; resolving them is a
    /// separate policy-gated step.
    pub content_refs: Vec<ContentRef>,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
    /// Cursor identifying a replay, export, or subscription position.
    /// Use it to resume without widening the original scope.
    pub cursor: StreamCursor,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy: PrivacyClass,
    /// Retention class used by hosts and sinks when storing or exporting this
    /// item.
    pub retention: RetentionClass,
}

impl RealtimeInputFrame {
    /// Builds the media ref value with the documented defaults.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn media_ref(
        media_kind: RealtimeMediaKind,
        content_ref: ContentRef,
        redacted_summary: impl Into<String>,
    ) -> Self {
        Self {
            frame_id: RealtimeFrameId::new(format!("realtime.frame.{}", content_ref.as_str())),
            media_kind,
            content_refs: vec![content_ref],
            redacted_summary: redacted_summary.into(),
            cursor: StreamCursor::chunk(0),
            privacy: PrivacyClass::ContentRefsOnly,
            retention: RetentionClass::RunScoped,
        }
    }

    /// Returns this value with its cursor setting replaced. The method
    /// follows builder-style data construction and does not execute
    /// external work.
    pub fn with_cursor(mut self, cursor: StreamCursor) -> Self {
        self.cursor = cursor;
        self
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the realtime output frame record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct RealtimeOutputFrame {
    /// Stable frame id used for typed lineage, lookup, or dedupe.
    pub frame_id: RealtimeFrameId,
    /// Stable response id used for typed lineage, lookup, or dedupe.
    pub response_id: RealtimeResponseId,
    /// Kind discriminator for media kind.
    /// Use it to route finite match arms without parsing display text.
    pub media_kind: RealtimeMediaKind,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Content references associated with this record; resolving them is a
    /// separate policy-gated step.
    pub content_refs: Vec<ContentRef>,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
    /// Cursor identifying a replay, export, or subscription position.
    /// Use it to resume without widening the original scope.
    pub cursor: StreamCursor,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy: PrivacyClass,
    /// Retention class used by hosts and sinks when storing or exporting this
    /// item.
    pub retention: RetentionClass,
}

impl RealtimeOutputFrame {
    /// Builds the transcript value with the documented defaults.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn transcript(
        response_id: impl Into<String>,
        content_ref: impl Into<String>,
        redacted_summary: impl Into<String>,
    ) -> Self {
        let content_ref = ContentRef::new(content_ref);
        Self {
            frame_id: RealtimeFrameId::new(format!("realtime.frame.{}", content_ref.as_str())),
            response_id: RealtimeResponseId::new(response_id),
            media_kind: RealtimeMediaKind::Transcript,
            content_refs: vec![content_ref],
            redacted_summary: redacted_summary.into(),
            cursor: StreamCursor::chunk(0),
            privacy: PrivacyClass::ContentRefsOnly,
            retention: RetentionClass::RunScoped,
        }
    }

    /// Returns this value with its cursor setting replaced. The method
    /// follows builder-style data construction and does not execute
    /// external work.
    pub fn with_cursor(mut self, cursor: StreamCursor) -> Self {
        self.cursor = cursor;
        self
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the realtime session state record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct RealtimeSessionState {
    /// Stable session id used for typed lineage, lookup, or dedupe.
    pub session_id: RealtimeSessionId,
    /// Stable connection id used for typed lineage, lookup, or dedupe.
    pub connection_id: RealtimeConnectionId,
    /// Typed provider route ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub provider_route_ref: String,
    /// Cursor identifying a replay, export, or subscription position.
    /// Use it to resume without widening the original scope.
    pub send_cursor: StreamCursor,
    /// Cursor identifying a replay, export, or subscription position.
    /// Use it to resume without widening the original scope.
    pub receive_cursor: StreamCursor,
    /// Count of restart items observed or included in this record.
    pub restart_count: u32,
    /// Backpressure state used by this record or request.
    pub backpressure_state: RealtimeBackpressureState,
    /// Lifecycle status used by this record or request.
    pub lifecycle_status: RealtimeSessionStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite realtime session record kind cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum RealtimeSessionRecordKind {
    /// Use this variant when the contract needs to represent connect requested; selecting it has no side effect by itself.
    ConnectRequested,
    /// Use this variant when the contract needs to represent connected; selecting it has no side effect by itself.
    Connected,
    /// Use this variant when the contract needs to represent input send requested; selecting it has no side effect by itself.
    InputSendRequested,
    /// Use this variant when the contract needs to represent input sent; selecting it has no side effect by itself.
    InputSent,
    /// Use this variant when the contract needs to represent output receive requested; selecting it has no side effect by itself.
    OutputReceiveRequested,
    /// Use this variant when the contract needs to represent output received; selecting it has no side effect by itself.
    OutputReceived,
    /// Use this variant when the contract needs to represent interrupt requested; selecting it has no side effect by itself.
    InterruptRequested,
    /// Use this variant when the contract needs to represent interrupted; selecting it has no side effect by itself.
    Interrupted,
    /// Use this variant when the contract needs to represent restart requested; selecting it has no side effect by itself.
    RestartRequested,
    /// Use this variant when the contract needs to represent restart started; selecting it has no side effect by itself.
    RestartStarted,
    /// Use this variant when the contract needs to represent restart completed; selecting it has no side effect by itself.
    RestartCompleted,
    /// Use this variant when the contract needs to represent restart failed; selecting it has no side effect by itself.
    RestartFailed,
    /// Use this variant when the contract needs to represent close requested; selecting it has no side effect by itself.
    CloseRequested,
    /// Use this variant when the contract needs to represent closed; selecting it has no side effect by itself.
    Closed,
    /// Use this variant when the contract needs to represent backpressure applied; selecting it has no side effect by itself.
    BackpressureApplied,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the realtime session record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct RealtimeSessionRecord {
    /// Kind/category for this record, capability, event, or detected
    /// resource.
    pub kind: RealtimeSessionRecordKind,
    /// Stable session id used for typed lineage, lookup, or dedupe.
    pub session_id: RealtimeSessionId,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stable connection id used for typed lineage, lookup, or dedupe.
    pub connection_id: Option<RealtimeConnectionId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stable response id used for typed lineage, lookup, or dedupe.
    pub response_id: Option<RealtimeResponseId>,
    /// Run identifier used for lineage, filtering, replay, and dedupe.
    pub run_id: RunId,
    /// Agent identifier used for lineage, filtering, and ownership checks.
    pub agent_id: AgentId,
    /// Typed provider route ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub provider_route_ref: String,
    /// Cursor identifying a replay, export, or subscription position.
    /// Use it to resume without widening the original scope.
    pub send_cursor: StreamCursor,
    /// Cursor identifying a replay, export, or subscription position.
    /// Use it to resume without widening the original scope.
    pub receive_cursor: StreamCursor,
    /// Count of restart items observed or included in this record.
    pub restart_count: u32,
    /// Backpressure state used by this record or request.
    pub backpressure_state: RealtimeBackpressureState,
    /// Finite status for this record or lifecycle stage.
    pub status: RealtimeSessionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional close reason value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub close_reason: Option<RealtimeCloseReason>,
    /// Channel used by this record or request.
    pub channel: StreamChannel,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional direction value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub direction: Option<StreamDirection>,
    /// Kind discriminator for media kind.
    /// Use it to route finite match arms without parsing display text.
    pub media_kind: RealtimeMediaKind,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Content references associated with this record; resolving them is a
    /// separate policy-gated step.
    pub content_refs: Vec<ContentRef>,
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
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Typed effect intent ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub effect_intent_ref: Option<EffectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Typed effect result ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub effect_result_ref: Option<EffectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional effect intent value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub effect_intent: Option<EffectIntent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional effect result value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub effect_result: Option<EffectResult>,
}

impl RealtimeSessionRecord {
    /// Returns the event kind name currently held by this value.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn event_kind_name(&self) -> &'static str {
        match self.kind {
            RealtimeSessionRecordKind::ConnectRequested => "realtime_connect_requested",
            RealtimeSessionRecordKind::Connected => "realtime_connected",
            RealtimeSessionRecordKind::InputSendRequested => "realtime_input_send_requested",
            RealtimeSessionRecordKind::InputSent => "realtime_input_sent",
            RealtimeSessionRecordKind::OutputReceiveRequested => {
                "realtime_output_receive_requested"
            }
            RealtimeSessionRecordKind::OutputReceived => "realtime_output_received",
            RealtimeSessionRecordKind::InterruptRequested => "realtime_interrupt_requested",
            RealtimeSessionRecordKind::Interrupted => "realtime_interrupted",
            RealtimeSessionRecordKind::RestartRequested => "realtime_restart_requested",
            RealtimeSessionRecordKind::RestartStarted => "realtime_restart_started",
            RealtimeSessionRecordKind::RestartCompleted => "realtime_restart_completed",
            RealtimeSessionRecordKind::RestartFailed => "realtime_restart_failed",
            RealtimeSessionRecordKind::CloseRequested => "realtime_close_requested",
            RealtimeSessionRecordKind::Closed => "realtime_closed",
            RealtimeSessionRecordKind::BackpressureApplied => "realtime_backpressure_applied",
        }
    }
}
