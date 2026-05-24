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
        #[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self::try_new(value).expect(concat!(stringify!($name), " must be valid"))
            }

            pub fn try_new(value: impl Into<String>) -> Result<Self, IdValidationError> {
                let value = value.into();
                validate_identifier(&value)?;
                Ok(Self(value))
            }

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
pub enum RealtimeMediaKind {
    Audio,
    Text,
    Image,
    Video,
    Transcript,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RealtimeSessionStatus {
    Connecting,
    Connected,
    InputSent,
    OutputReceived,
    Interrupted,
    RestartRequested,
    RestartStarted,
    RestartCompleted,
    RestartFailed,
    Closed,
    Failed,
    Detached,
    BackpressureApplied,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RealtimeCloseReason {
    Normal,
    Cancel,
    ProviderFailure,
    PolicyDenial,
    HostOwnedDetach,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RealtimeBackpressureAction {
    Block,
    Gate,
    DropNoncritical,
    Summarize,
    FailSend,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RealtimeBackpressureState {
    pub capacity: usize,
    pub queued_frames: usize,
    pub dropped_frames: usize,
    pub gated_frames: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_action: Option<RealtimeBackpressureAction>,
    pub policy_ref: PolicyRef,
}

impl RealtimeBackpressureState {
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

    pub fn gate(mut self) -> Self {
        self.gated_frames += 1;
        self.last_action = Some(RealtimeBackpressureAction::Gate);
        self
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RealtimeInputFrame {
    pub frame_id: RealtimeFrameId,
    pub media_kind: RealtimeMediaKind,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content_refs: Vec<ContentRef>,
    pub redacted_summary: String,
    pub cursor: StreamCursor,
    pub privacy: PrivacyClass,
    pub retention: RetentionClass,
}

impl RealtimeInputFrame {
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

    pub fn with_cursor(mut self, cursor: StreamCursor) -> Self {
        self.cursor = cursor;
        self
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RealtimeOutputFrame {
    pub frame_id: RealtimeFrameId,
    pub response_id: RealtimeResponseId,
    pub media_kind: RealtimeMediaKind,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content_refs: Vec<ContentRef>,
    pub redacted_summary: String,
    pub cursor: StreamCursor,
    pub privacy: PrivacyClass,
    pub retention: RetentionClass,
}

impl RealtimeOutputFrame {
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

    pub fn with_cursor(mut self, cursor: StreamCursor) -> Self {
        self.cursor = cursor;
        self
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RealtimeSessionState {
    pub session_id: RealtimeSessionId,
    pub connection_id: RealtimeConnectionId,
    pub provider_route_ref: String,
    pub send_cursor: StreamCursor,
    pub receive_cursor: StreamCursor,
    pub restart_count: u32,
    pub backpressure_state: RealtimeBackpressureState,
    pub lifecycle_status: RealtimeSessionStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policy_refs: Vec<PolicyRef>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RealtimeSessionRecordKind {
    ConnectRequested,
    Connected,
    InputSendRequested,
    InputSent,
    OutputReceiveRequested,
    OutputReceived,
    InterruptRequested,
    Interrupted,
    RestartRequested,
    RestartStarted,
    RestartCompleted,
    RestartFailed,
    CloseRequested,
    Closed,
    BackpressureApplied,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RealtimeSessionRecord {
    pub kind: RealtimeSessionRecordKind,
    pub session_id: RealtimeSessionId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<RealtimeConnectionId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_id: Option<RealtimeResponseId>,
    pub run_id: RunId,
    pub agent_id: AgentId,
    pub provider_route_ref: String,
    pub send_cursor: StreamCursor,
    pub receive_cursor: StreamCursor,
    pub restart_count: u32,
    pub backpressure_state: RealtimeBackpressureState,
    pub status: RealtimeSessionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub close_reason: Option<RealtimeCloseReason>,
    pub channel: StreamChannel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<StreamDirection>,
    pub media_kind: RealtimeMediaKind,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content_refs: Vec<ContentRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policy_refs: Vec<PolicyRef>,
    pub privacy: PrivacyClass,
    pub retention: RetentionClass,
    pub redacted_summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect_intent_ref: Option<EffectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect_result_ref: Option<EffectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect_intent: Option<EffectIntent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect_result: Option<EffectResult>,
}

impl RealtimeSessionRecord {
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
