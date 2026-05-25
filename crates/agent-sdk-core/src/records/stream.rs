//! Durable and observable SDK records. Use these DTOs for events, journals, effects,
//! context, output, and feature evidence. Constructing records is data-only;
//! persistence, publication, and external actions happen through ports or application
//! coordinators. This file contains the stream portion of that contract.
//!
use core::fmt;

use serde::{Deserialize, Deserializer, Serialize, de::Error as DeError};
use sha2::{Digest, Sha256};

use crate::{
    domain::{
        AgentError, AgentId, ContentRef, DestinationRef, EffectId, EntityKind, EntityRef,
        IdValidationError, MessageId, PolicyKind, PolicyRef, PrivacyClass, RetentionClass, RunId,
        SourceKind, SourceRef, ToolCallId, TurnId,
    },
    effect::{EffectIntent, EffectKind, EffectResult},
    ids::{AttemptId, validate_identifier},
    journal::{JournalRecord, JournalRecordBase, JournalRecordKind, JournalRecordPayload},
};

macro_rules! typed_stream_id {
    ($name:ident) => {
        #[doc = concat!(
                            "Typed stream identifier for `",
                            stringify!($name),
                            "`. Use it to correlate deltas, rules, matchers, and interventions; ",
                            "constructing it is data-only and performs no side effects."
                        )]
        #[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Creates a new records::stream value with explicit
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

            /// Creates a new records::stream value after validation.
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

typed_stream_id!(StreamDeltaId);
typed_stream_id!(StreamRuleId);
typed_stream_id!(StreamInterventionId);
typed_stream_id!(StreamMatchId);
typed_stream_id!(MarkerId);
typed_stream_id!(MarkerVersion);
typed_stream_id!(MatcherEngineRef);

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
/// Carries the rule version record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct RuleVersion(pub u32);

impl RuleVersion {
    /// Creates a new records::stream value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new(version: u32) -> Self {
        assert!(version > 0, "RuleVersion must be nonzero");
        Self(version)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite stream channel cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum StreamChannel {
    /// Use this variant when the contract needs to represent assistant text; selecting it has no side effect by itself.
    AssistantText,
    /// Use this variant when the contract needs to represent reasoning summary; selecting it has no side effect by itself.
    ReasoningSummary,
    /// Use this variant when the contract needs to represent provider exposed reasoning; selecting it has no side effect by itself.
    ProviderExposedReasoning,
    /// Use this variant when the contract needs to represent tool call arguments; selecting it has no side effect by itself.
    ToolCallArguments,
    /// Use this variant when the contract needs to represent tool result text; selecting it has no side effect by itself.
    ToolResultText,
    /// Use this variant when the contract needs to represent realtime transcript; selecting it has no side effect by itself.
    RealtimeTranscript,
    /// Use this variant when the contract needs to represent realtime media; selecting it has no side effect by itself.
    RealtimeMedia,
    /// Use this variant when the contract needs to represent hidden chain of thought; selecting it has no side effect by itself.
    HiddenChainOfThought,
}

impl StreamChannel {
    /// Reports whether this value is policy visible. The check is pure
    /// and does not mutate SDK or host state.
    pub fn is_policy_visible(&self) -> bool {
        !matches!(self, Self::HiddenChainOfThought)
    }

    /// Returns this value as contract name. The accessor is side-effect
    /// free and keeps ownership with the caller.
    pub fn as_contract_name(&self) -> &'static str {
        match self {
            Self::AssistantText => "assistant_text",
            Self::ReasoningSummary => "reasoning_summary",
            Self::ProviderExposedReasoning => "provider_exposed_reasoning",
            Self::ToolCallArguments => "tool_call_arguments",
            Self::ToolResultText => "tool_result_text",
            Self::RealtimeTranscript => "realtime_transcript",
            Self::RealtimeMedia => "realtime_media",
            Self::HiddenChainOfThought => "hidden_chain_of_thought",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite stream direction cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum StreamDirection {
    /// Use this variant when the contract needs to represent input to provider; selecting it has no side effect by itself.
    InputToProvider,
    /// Use this variant when the contract needs to represent output from provider; selecting it has no side effect by itself.
    OutputFromProvider,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite stream cursor precision cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum StreamCursorPrecision {
    /// Use this variant when the contract needs to represent byte offset; selecting it has no side effect by itself.
    ByteOffset,
    /// Use this variant when the contract needs to represent chunk sequence only; selecting it has no side effect by itself.
    ChunkSequenceOnly,
    /// Use this variant when the contract needs to represent marker; selecting it has no side effect by itself.
    Marker,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
/// Carries the stream cursor record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct StreamCursor {
    /// Chunk sequence used by this record or request.
    pub chunk_sequence: u64,
    /// Byte size or byte limit for byte offset.
    /// Use it to enforce bounded reads, writes, summaries, or parser output.
    pub byte_offset: u64,
    /// Precision used by this record or request.
    pub precision: StreamCursorPrecision,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional label value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub label: Option<String>,
}

impl StreamCursor {
    /// Builds the chunk value with the documented defaults.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn chunk(chunk_sequence: u64) -> Self {
        Self {
            chunk_sequence,
            byte_offset: 0,
            precision: StreamCursorPrecision::ChunkSequenceOnly,
            label: None,
        }
    }

    /// Builds the byte value with the documented defaults.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn byte(chunk_sequence: u64, byte_offset: u64) -> Self {
        Self {
            chunk_sequence,
            byte_offset,
            precision: StreamCursorPrecision::ByteOffset,
            label: None,
        }
    }

    /// Marker.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn marker(chunk_sequence: u64, label: impl Into<String>) -> Self {
        Self {
            chunk_sequence,
            byte_offset: 0,
            precision: StreamCursorPrecision::Marker,
            label: Some(label.into()),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the stream delta record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct StreamDelta {
    /// Stable delta id used for typed lineage, lookup, or dedupe.
    pub delta_id: StreamDeltaId,
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
    /// Message identifier for transcript, projection, or provider-response
    /// lineage.
    pub message_id: Option<MessageId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stable tool call id used for typed lineage, lookup, or dedupe.
    pub tool_call_id: Option<ToolCallId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stable realtime session id used for typed lineage, lookup, or dedupe.
    pub realtime_session_id: Option<crate::realtime_records::RealtimeSessionId>,
    /// Channel used by this record or request.
    pub channel: StreamChannel,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional direction value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub direction: Option<StreamDirection>,
    /// Cursor identifying a replay, export, or subscription position.
    /// Use it to resume without widening the original scope.
    pub cursor: StreamCursor,
    /// Source label or ref for this item; it is metadata and does not fetch
    /// content by itself.
    pub source: SourceRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Destination label or ref for this item; it is metadata and does not
    /// deliver content by itself.
    pub destination: Option<DestinationRef>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Content reference where payload bytes or structured tool output are
    /// stored.
    pub content_ref: Option<ContentRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stable marker id used for typed lineage, lookup, or dedupe.
    pub marker_id: Option<MarkerId>,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
    /// Fingerprint of the runtime package snapshot in force when this value was produced.
    /// Use it for replay, dedupe, and package-lineage checks; the field is evidence and does
    /// not execute package behavior.
    pub runtime_package_fingerprint: String,
    #[serde(skip)]
    match_text: Option<String>,
}

impl StreamDelta {
    /// Builds the visible text value with the documented defaults.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn visible_text(
        delta_id: impl Into<String>,
        channel: StreamChannel,
        cursor: StreamCursor,
        text: impl Into<String>,
        source: SourceRef,
    ) -> Self {
        let text = text.into();
        Self {
            delta_id: StreamDeltaId::new(delta_id),
            run_id: RunId::new("run.stream.delta.unset"),
            agent_id: AgentId::new("agent.stream.delta.unset"),
            turn_id: None,
            attempt_id: None,
            message_id: None,
            tool_call_id: None,
            realtime_session_id: None,
            channel,
            direction: None,
            cursor,
            source,
            destination: None,
            policy_refs: Vec::new(),
            privacy: PrivacyClass::Public,
            retention: RetentionClass::RunScoped,
            content_ref: None,
            marker_id: None,
            redacted_summary: format!("{} bytes visible stream text", text.len()),
            runtime_package_fingerprint: "runtime.package.fingerprint.test".to_string(),
            match_text: Some(text),
        }
    }

    /// Marker.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn marker(
        delta_id: impl Into<String>,
        channel: StreamChannel,
        cursor: StreamCursor,
        marker_id: impl Into<String>,
        source: SourceRef,
    ) -> Self {
        let marker_id = MarkerId::new(marker_id);
        Self {
            delta_id: StreamDeltaId::new(delta_id),
            run_id: RunId::new("run.stream.delta.unset"),
            agent_id: AgentId::new("agent.stream.delta.unset"),
            turn_id: None,
            attempt_id: None,
            message_id: None,
            tool_call_id: None,
            realtime_session_id: None,
            channel,
            direction: None,
            cursor,
            source,
            destination: None,
            policy_refs: Vec::new(),
            privacy: PrivacyClass::Internal,
            retention: RetentionClass::RunScoped,
            content_ref: None,
            marker_id: Some(marker_id),
            redacted_summary: "typed stream marker".to_string(),
            runtime_package_fingerprint: "runtime.package.fingerprint.test".to_string(),
            match_text: None,
        }
    }

    /// Returns this value with its run setting replaced. The method
    /// follows builder-style data construction and does not execute
    /// external work.
    pub fn with_run(mut self, run_id: RunId, agent_id: AgentId) -> Self {
        self.run_id = run_id;
        self.agent_id = agent_id;
        self
    }

    /// Returns this value with its turn setting replaced. The method
    /// follows builder-style data construction and does not execute
    /// external work.
    pub fn with_turn(mut self, turn_id: TurnId) -> Self {
        self.turn_id = Some(turn_id);
        self
    }

    /// Returns this value with its attempt setting replaced. The method
    /// follows builder-style data construction and does not execute
    /// external work.
    pub fn with_attempt(mut self, attempt_id: AttemptId) -> Self {
        self.attempt_id = Some(attempt_id);
        self
    }

    /// Returns this value with its direction setting replaced. The
    /// method follows builder-style data construction and does not
    /// execute external work.
    pub fn with_direction(mut self, direction: StreamDirection) -> Self {
        self.direction = Some(direction);
        self
    }

    /// Returns this value with its destination setting replaced. The
    /// method follows builder-style data construction and does not
    /// execute external work.
    pub fn with_destination(mut self, destination: DestinationRef) -> Self {
        self.destination = Some(destination);
        self
    }

    /// Returns this value with its content ref setting replaced. The
    /// method follows builder-style data construction and does not
    /// execute external work.
    pub fn with_content_ref(mut self, content_ref: ContentRef) -> Self {
        self.content_ref = Some(content_ref);
        self
    }

    /// Returns the matcher text currently held by this value.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn matcher_text(&self) -> Option<&str> {
        if !self.channel.is_policy_visible() || self.privacy == PrivacyClass::Secret {
            return None;
        }
        self.match_text.as_deref()
    }

    /// Returns whether serialized raw text absent applies for this contract.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn serialized_raw_text_absent(&self) -> bool {
        serde_json::to_string(self)
            .map(|json| {
                self.match_text
                    .as_ref()
                    .is_none_or(|text| !json.contains(text))
            })
            .unwrap_or(false)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the stream channel selector record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct StreamChannelSelector {
    /// Channel used by this record or request.
    pub channel: StreamChannel,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional direction value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub direction: Option<StreamDirection>,
}

impl StreamChannelSelector {
    /// Builds the channel value with the documented defaults.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn channel(channel: StreamChannel) -> Self {
        Self {
            channel,
            direction: None,
        }
    }

    /// Returns matches for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn matches(&self, delta: &StreamDelta) -> bool {
        self.channel == delta.channel
            && self
                .direction
                .as_ref()
                .is_none_or(|direction| Some(direction) == delta.direction.as_ref())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite regex dialect cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum RegexDialect {
    /// Use this variant when the contract needs to represent safe subset; selecting it has no side effect by itself.
    SafeSubset,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
/// Enumerates the finite stream matcher cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum StreamMatcher {
    /// Use this variant when the contract needs to represent literal; selecting it has no side effect by itself.
    Literal {
        /// Text used by this record or request.
        text: String,
        /// Whether case sensitive is enabled.
        /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
        case_sensitive: bool,
        /// window bytes used for bounds checks, summaries, or truncation
        /// evidence.
        window_bytes: u64,
    },
    /// Use this variant when the contract needs to represent regex; selecting it has no side effect by itself.
    Regex {
        /// Search pattern supplied by the caller.
        /// The grep executor compiles it under regex and output bounds before reading files.
        pattern: String,
        /// Schema dialect used to interpret the output schema.
        /// Validators use it to select the supported JSON-schema subset and compatibility
        /// rules.
        dialect: RegexDialect,
        /// window bytes used for bounds checks, summaries, or truncation
        /// evidence.
        window_bytes: u64,
        /// Timeout budget in milliseconds for the requested operation.
        timeout_ms: u64,
    },
    /// Use this variant when the contract needs to represent marker; selecting it has no side effect by itself.
    Marker {
        /// Stable marker id used for typed lineage, lookup, or dedupe.
        marker_id: MarkerId,
        /// Version string for this capability, package, or protocol surface.
        /// Use it for compatibility checks during package or adapter resolution.
        marker_version: MarkerVersion,
    },
    /// Use this variant when the contract needs to represent host matcher; selecting it has no side effect by itself.
    HostMatcher {
        /// Typed matcher ref reference. Resolving or executing it is a
        /// separate policy-gated step.
        matcher_ref: MatcherEngineRef,
        #[serde(skip_serializing_if = "Option::is_none")]
        /// Typed risk policy ref reference. Resolving or executing it is a
        /// separate policy-gated step.
        risk_policy_ref: Option<PolicyRef>,
    },
}

impl StreamMatcher {
    /// Builds the literal value with the documented defaults.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn literal(text: impl Into<String>, case_sensitive: bool, window_bytes: u64) -> Self {
        Self::Literal {
            text: text.into(),
            case_sensitive,
            window_bytes,
        }
    }

    /// Builds the regex with limits value with the documented defaults.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn regex_with_limits(
        pattern: impl Into<String>,
        window_bytes: u64,
        timeout_ms: u64,
    ) -> Self {
        Self::Regex {
            pattern: pattern.into(),
            dialect: RegexDialect::SafeSubset,
            window_bytes,
            timeout_ms,
        }
    }

    /// Marker.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn marker(marker_id: impl Into<String>) -> Self {
        Self::Marker {
            marker_id: MarkerId::new(marker_id),
            marker_version: MarkerVersion::new("marker.v1"),
        }
    }

    /// Builds the window bytes value with the documented defaults.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn window_bytes(&self) -> u64 {
        match self {
            Self::Literal { window_bytes, .. } | Self::Regex { window_bytes, .. } => *window_bytes,
            Self::Marker { .. } | Self::HostMatcher { .. } => 0,
        }
    }

    /// Returns the kind name currently held by this value.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::Literal { .. } => "literal",
            Self::Regex { .. } => "regex",
            Self::Marker { .. } => "marker",
            Self::HostMatcher { .. } => "host_matcher",
        }
    }

    /// Validates the records::stream invariants and returns a typed
    /// error on failure. Validation is pure and does not perform I/O,
    /// dispatch, journal appends, or adapter calls.
    pub fn validate(&self) -> Result<(), AgentError> {
        match self {
            Self::Literal {
                text, window_bytes, ..
            } => {
                if text.is_empty() {
                    return Err(AgentError::missing_required_field(
                        "stream_matcher.literal.text",
                    ));
                }
                validate_window(*window_bytes)
            }
            Self::Regex {
                pattern,
                window_bytes,
                timeout_ms,
                ..
            } => {
                validate_window(*window_bytes)?;
                if *timeout_ms == 0 || *timeout_ms > 1000 {
                    return Err(AgentError::contract_violation(
                        "stream regex timeout must be between 1 and 1000ms",
                    ));
                }
                validate_safe_regex(pattern)
            }
            Self::Marker { marker_id, .. } => {
                if marker_id.as_str().is_empty() {
                    return Err(AgentError::missing_required_field(
                        "stream_matcher.marker.marker_id",
                    ));
                }
                Ok(())
            }
            Self::HostMatcher {
                risk_policy_ref, ..
            } => {
                if risk_policy_ref.is_none() {
                    return Err(AgentError::contract_violation(
                        "host stream matcher requires declared risk policy ref",
                    ));
                }
                Ok(())
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite stream rule scope cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum StreamRuleScope {
    /// Use this variant when the contract needs to represent run; selecting it has no side effect by itself.
    Run,
    /// Use this variant when the contract needs to represent turn; selecting it has no side effect by itself.
    Turn,
    /// Use this variant when the contract needs to represent attempt; selecting it has no side effect by itself.
    Attempt,
    /// Use this variant when the contract needs to represent realtime session; selecting it has no side effect by itself.
    RealtimeSession,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite partial output policy cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum PartialOutputPolicy {
    /// Use this variant when the contract needs to represent keep; selecting it has no side effect by itself.
    Keep,
    /// Use this variant when the contract needs to represent discard; selecting it has no side effect by itself.
    Discard,
    /// Use this variant when the contract needs to represent mask; selecting it has no side effect by itself.
    Mask,
    /// Use this variant when the contract needs to represent content ref only; selecting it has no side effect by itself.
    ContentRefOnly,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite repeat policy cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum RepeatPolicy {
    /// Use this variant when the contract needs to represent always; selecting it has no side effect by itself.
    Always,
    /// Use this variant when the contract needs to represent once per run; selecting it has no side effect by itself.
    OncePerRun,
    /// Use this variant when the contract needs to represent once per turn; selecting it has no side effect by itself.
    OncePerTurn,
    /// Use this variant when the contract needs to represent once per attempt and span; selecting it has no side effect by itself.
    OncePerAttemptAndSpan,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite match privacy policy cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum MatchPrivacyPolicy {
    /// Use this variant when the contract needs to represent hash length and summary; selecting it has no side effect by itself.
    HashLengthAndSummary,
    /// Use this variant when the contract needs to represent raw capture if policy allows; selecting it has no side effect by itself.
    RawCaptureIfPolicyAllows,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "action", rename_all = "snake_case")]
/// Enumerates the finite stream action cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum StreamAction {
    /// Use this variant when the contract needs to represent stop run; selecting it has no side effect by itself.
    StopRun {
        /// Redacted explanation for a denial, failure, status, or package
        /// delta.
        reason: String,
        /// Partial output used by this record or request.
        partial_output: PartialOutputPolicy,
    },
    /// Use this variant when the contract needs to represent abort and retry; selecting it has no side effect by itself.
    AbortAndRetry {
        /// Redacted summary for display, logs, events, or telemetry.
        /// It should describe the value without exposing raw private content.
        injection_summary: String,
        /// Typed retry policy ref reference. Resolving or executing it is a
        /// separate policy-gated step.
        retry_policy_ref: PolicyRef,
        /// Partial output used by this record or request.
        partial_output: PartialOutputPolicy,
    },
    /// Use this variant when the contract needs to represent pause for approval; selecting it has no side effect by itself.
    PauseForApproval {
        /// Typed approval policy ref reference. Resolving or executing it is
        /// a separate policy-gated step.
        approval_policy_ref: PolicyRef,
        /// Typed resume policy ref reference. Resolving or executing it is a
        /// separate policy-gated step.
        resume_policy_ref: PolicyRef,
    },
    /// Use this variant when the contract needs to represent mask and continue; selecting it has no side effect by itself.
    MaskAndContinue {
        /// Replacement used by this record or request.
        replacement: String,
    },
    /// Use this variant when the contract needs to represent emit only; selecting it has no side effect by itself.
    EmitOnly {
        /// Kind discriminator for notice kind.
        /// Use it to route finite match arms without parsing display text.
        notice_kind: String,
    },
}

impl StreamAction {
    /// Returns an updated value with mask and continue configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn mask_and_continue(replacement: impl Into<String>) -> Self {
        Self::MaskAndContinue {
            replacement: replacement.into(),
        }
    }

    /// Returns an updated value with abort and retry configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn abort_and_retry(
        injection_summary: impl Into<String>,
        retry_policy_ref: PolicyRef,
    ) -> Self {
        Self::AbortAndRetry {
            injection_summary: injection_summary.into(),
            retry_policy_ref,
            partial_output: PartialOutputPolicy::Discard,
        }
    }

    /// Builds the emit only value with the documented defaults.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn emit_only(notice_kind: impl Into<String>) -> Self {
        Self::EmitOnly {
            notice_kind: notice_kind.into(),
        }
    }

    /// Returns the action kind currently held by this value.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn action_kind(&self) -> &'static str {
        match self {
            Self::StopRun { .. } => "stop_run",
            Self::AbortAndRetry { .. } => "abort_and_retry",
            Self::PauseForApproval { .. } => "pause_for_approval",
            Self::MaskAndContinue { .. } => "mask_and_continue",
            Self::EmitOnly { .. } => "emit_only",
        }
    }

    /// Builds the partial output policy value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn partial_output_policy(&self) -> PartialOutputPolicy {
        match self {
            Self::StopRun { partial_output, .. } | Self::AbortAndRetry { partial_output, .. } => {
                partial_output.clone()
            }
            Self::PauseForApproval { .. } | Self::EmitOnly { .. } => PartialOutputPolicy::Keep,
            Self::MaskAndContinue { .. } => PartialOutputPolicy::Mask,
        }
    }

    /// Returns the effect kind hint currently held by this value.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn effect_kind_hint(&self) -> Option<EffectKind> {
        match self {
            Self::AbortAndRetry { .. } => Some(EffectKind::ProviderRequest),
            Self::PauseForApproval { .. } => Some(EffectKind::ApprovalDispatch),
            Self::StopRun { .. } | Self::MaskAndContinue { .. } | Self::EmitOnly { .. } => None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the stream rule record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct StreamRule {
    /// Stable identifier for this record.
    pub id: StreamRuleId,
    /// Version string for this capability, package, or protocol surface.
    /// Use it for compatibility checks during package or adapter resolution.
    pub version: RuleVersion,
    /// Source label or ref for this item; it is metadata and does not fetch
    /// content by itself.
    pub source: SourceRef,
    /// Matcher used by this record or request.
    pub matcher: StreamMatcher,
    /// Collection of channels values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub channels: Vec<StreamChannelSelector>,
    /// Scope used by this record or request.
    pub scope: StreamRuleScope,
    /// Action used by this record or request.
    pub action: StreamAction,
    /// Repeat used by this record or request.
    pub repeat: RepeatPolicy,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy: MatchPrivacyPolicy,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
}

impl StreamRule {
    /// Starts a builder for this records::stream value. Building is
    /// data-only; runtime side effects occur only when a later
    /// coordinator or host port executes the built configuration.
    pub fn builder(id: StreamRuleId) -> StreamRuleBuilder {
        StreamRuleBuilder::new(id)
    }

    /// Returns an updated value with mask regex configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn mask_regex(id: impl Into<String>, pattern: impl Into<String>) -> StreamRuleBuilder {
        StreamRuleBuilder::new(StreamRuleId::new(id))
            .source(SourceRef::with_kind(
                SourceKind::Host,
                "source.host.stream_rules",
            ))
            .matcher(StreamMatcher::regex_with_limits(pattern, 4096, 25))
            .action(StreamAction::mask_and_continue("[redacted]"))
    }

    /// Validates the records::stream invariants and returns a typed
    /// error on failure. Validation is pure and does not perform I/O,
    /// dispatch, journal appends, or adapter calls.
    pub fn validate(&self) -> Result<(), AgentError> {
        self.matcher.validate()?;
        if self.channels.is_empty() {
            return Err(AgentError::missing_required_field("stream_rule.channels"));
        }
        if self.policy_refs.is_empty() {
            return Err(AgentError::missing_required_field(
                "stream_rule.policy_refs",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
/// Carries the stream rule builder record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct StreamRuleBuilder {
    rule: StreamRule,
}

impl StreamRuleBuilder {
    /// Creates a new records::stream value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new(id: StreamRuleId) -> Self {
        Self {
            rule: StreamRule {
                id,
                version: RuleVersion::new(1),
                source: SourceRef::with_kind(SourceKind::Host, "source.host.stream_rules"),
                matcher: StreamMatcher::literal("unset", true, 64),
                channels: Vec::new(),
                scope: StreamRuleScope::Run,
                action: StreamAction::emit_only("notice"),
                repeat: RepeatPolicy::OncePerAttemptAndSpan,
                privacy: MatchPrivacyPolicy::HashLengthAndSummary,
                policy_refs: Vec::new(),
            },
        }
    }

    /// Returns an updated records::stream value with source applied. This is
    /// data construction only and does not execute the configured behavior.
    pub fn source(mut self, source: SourceRef) -> Self {
        self.rule.source = source;
        self
    }

    /// Returns an updated value with matcher configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn matcher(mut self, matcher: StreamMatcher) -> Self {
        self.rule.matcher = matcher;
        self
    }

    /// Returns an updated value with on configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn on(mut self, channel: StreamChannel) -> Self {
        self.rule
            .channels
            .push(StreamChannelSelector::channel(channel));
        self
    }

    /// Returns an updated records::stream value with action applied. This is
    /// data construction only and does not execute the configured behavior.
    pub fn action(mut self, action: StreamAction) -> Self {
        self.rule.action = action;
        self
    }

    /// Returns an updated value with repeat configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn repeat(mut self, repeat: RepeatPolicy) -> Self {
        self.rule.repeat = repeat;
        self
    }

    /// Returns policy for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn policy(mut self, policy_ref: PolicyRef) -> Self {
        self.rule.policy_refs.push(policy_ref);
        self
    }

    /// Finishes builder validation and returns the configured value.
    /// This is data-only unless the surrounding builder explicitly
    /// documents adapter or store access.
    pub fn build(self) -> Result<StreamRule, AgentError> {
        self.rule.validate()?;
        Ok(self.rule)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the redacted match record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct RedactedMatch {
    /// Stable match id used for typed lineage, lookup, or dedupe.
    pub match_id: StreamMatchId,
    /// Channel used by this record or request.
    pub channel: StreamChannel,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional direction value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub direction: Option<StreamDirection>,
    /// Cursor identifying a replay, export, or subscription position.
    /// Use it to resume without widening the original scope.
    pub cursor: StreamCursor,
    /// Observed byte length for the source, sidecar, or extracted record.
    pub byte_len: usize,
    /// Deterministic text hash used for stale checks, package evidence, or
    /// replay comparisons.
    pub text_hash: String,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy: PrivacyClass,
    /// Retention class used by hosts and sinks when storing or exporting this
    /// item.
    pub retention: RetentionClass,
}

impl RedactedMatch {
    /// Constructs this value from text. Use it when adapting canonical
    /// SDK records without introducing a second behavior path.
    pub fn from_text(rule: &StreamRule, delta: &StreamDelta, matched_text: &str) -> Self {
        let hash = format!("sha256:{:x}", Sha256::digest(matched_text.as_bytes()));
        Self {
            match_id: StreamMatchId::new(format!(
                "stream.match.{}.{}",
                safe_id_fragment(rule.id.as_str()),
                &hash["sha256:".len()..("sha256:".len() + 12)]
            )),
            channel: delta.channel.clone(),
            direction: delta.direction.clone(),
            cursor: delta.cursor.clone(),
            byte_len: matched_text.len(),
            text_hash: hash,
            redacted_summary: format!(
                "stream match redacted: {} bytes on {}",
                matched_text.len(),
                delta.channel.as_contract_name()
            ),
            privacy: PrivacyClass::ContentRefsOnly,
            retention: RetentionClass::RunScoped,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the stream match ref record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct StreamMatchRef {
    /// Stable match id used for typed lineage, lookup, or dedupe.
    pub match_id: StreamMatchId,
    /// Stable rule id used for typed lineage, lookup, or dedupe.
    pub rule_id: StreamRuleId,
    /// Version string for this capability, package, or protocol surface.
    /// Use it for compatibility checks during package or adapter resolution.
    pub rule_version: RuleVersion,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the stream intervention record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct StreamIntervention {
    /// Stable intervention id used for typed lineage, lookup, or dedupe.
    pub intervention_id: StreamInterventionId,
    /// Typed rule ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub rule_ref: EntityRef,
    /// Requested action used by this record or request.
    pub requested_action: StreamAction,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional applied action value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub applied_action: Option<StreamAction>,
    /// Typed match ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub match_ref: StreamMatchRef,
    /// Redacted match used by this record or request.
    pub redacted_match: RedactedMatch,
    /// Partial output policy used by this record or request.
    pub partial_output_policy: PartialOutputPolicy,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
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

impl StreamIntervention {
    /// Builds the proposed value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn proposed(rule: &StreamRule, redacted_match: RedactedMatch) -> Self {
        let match_ref = StreamMatchRef {
            match_id: redacted_match.match_id.clone(),
            rule_id: rule.id.clone(),
            rule_version: rule.version,
        };
        Self {
            intervention_id: StreamInterventionId::new(format!(
                "stream.intervention.{}.{}",
                safe_id_fragment(rule.id.as_str()),
                safe_id_fragment(redacted_match.match_id.as_str())
            )),
            rule_ref: EntityRef::new(EntityKind::StreamRule, rule.id.as_str()),
            requested_action: rule.action.clone(),
            applied_action: None,
            match_ref,
            redacted_match,
            partial_output_policy: rule.action.partial_output_policy(),
            policy_refs: rule.policy_refs.clone(),
            effect_intent_ref: None,
            effect_result_ref: None,
            effect_intent: None,
            effect_result: None,
        }
    }

    /// Returns this value with its effect intent setting replaced. The
    /// method follows builder-style data construction and does not
    /// execute external work.
    pub fn with_effect_intent(mut self, effect_id: EffectId) -> Self {
        if let Some(kind) = self.requested_action.effect_kind_hint() {
            let mut intent = EffectIntent::new(
                effect_id.clone(),
                kind,
                self.rule_ref.clone(),
                SourceRef::with_kind(SourceKind::Sdk, "source.sdk.stream_rule"),
                format!(
                    "stream intervention {} requested for redacted match",
                    self.requested_action.action_kind()
                ),
            );
            intent.policy_refs = self.policy_refs.clone();
            self.effect_intent_ref = Some(effect_id);
            self.effect_intent = Some(intent);
        }
        self
    }

    /// Returns this value with its effect result setting replaced. The
    /// method follows builder-style data construction and does not
    /// execute external work.
    pub fn with_effect_result(mut self, result: EffectResult) -> Self {
        self.effect_result_ref = Some(result.effect_id.clone());
        self.effect_result = Some(result);
        self
    }

    /// Returns the effect kind name currently held by this value.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn effect_kind_name(&self) -> Option<&'static str> {
        self.effect_intent.as_ref().map(|intent| match intent.kind {
            EffectKind::ProviderRequest => "provider_request",
            EffectKind::ToolExecution => "tool_execution",
            EffectKind::ApprovalDispatch => "approval_dispatch",
            EffectKind::MemoryWrite => "memory_write",
            EffectKind::ExtensionAction => "extension_action",
            EffectKind::OutputDelivery => "output_delivery",
            EffectKind::FileWrite => "file_write",
            EffectKind::ProcessStart => "process_start",
            EffectKind::ProcessSignal => "process_signal",
            EffectKind::IsolatedProcessStart => "isolated_process_start",
            EffectKind::ChildAgentStart => "child_agent_start",
            EffectKind::RunMessageDelivery => "run_message_delivery",
            EffectKind::ChildArtifactShutdown => "child_artifact_shutdown",
            EffectKind::DetachTransfer => "detach_transfer",
            EffectKind::HookMutation => "hook_mutation",
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite stream rule record kind cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum StreamRuleRecordKind {
    /// Use this variant when the contract needs to represent registered; selecting it has no side effect by itself.
    Registered,
    /// Use this variant when the contract needs to represent compile failed; selecting it has no side effect by itself.
    CompileFailed,
    /// Use this variant when the contract needs to represent matched; selecting it has no side effect by itself.
    Matched,
    /// Use this variant when the contract needs to represent intervention intent; selecting it has no side effect by itself.
    InterventionIntent,
    /// Use this variant when the contract needs to represent intervention result; selecting it has no side effect by itself.
    InterventionResult,
    /// Use this variant when the contract needs to represent repeat state; selecting it has no side effect by itself.
    RepeatState,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the stream rule repeat state snapshot record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct StreamRuleRepeatStateSnapshot {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Collection of seen match keys values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub seen_match_keys: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the stream rule record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct StreamRuleRecord {
    /// Kind discriminator for record kind.
    /// Use it to route finite match arms without parsing display text.
    pub record_kind: StreamRuleRecordKind,
    /// Stable rule id used for typed lineage, lookup, or dedupe.
    pub rule_id: StreamRuleId,
    /// Version string for this capability, package, or protocol surface.
    /// Use it for compatibility checks during package or adapter resolution.
    pub rule_version: RuleVersion,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional channel value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub channel: Option<StreamChannel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional direction value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub direction: Option<StreamDirection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Cursor identifying a replay, export, or subscription position.
    /// Use it to resume without widening the original scope.
    pub cursor: Option<StreamCursor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional redacted match value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub redacted_match: Option<RedactedMatch>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional intervention value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub intervention: Option<StreamIntervention>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional repeat state value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub repeat_state: Option<StreamRuleRepeatStateSnapshot>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

impl StreamRuleRecord {
    /// Builds the matched value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn matched(rule: &StreamRule, intervention: &StreamIntervention) -> Self {
        Self {
            record_kind: StreamRuleRecordKind::Matched,
            rule_id: rule.id.clone(),
            rule_version: rule.version,
            channel: Some(intervention.redacted_match.channel.clone()),
            direction: intervention.redacted_match.direction.clone(),
            cursor: Some(intervention.redacted_match.cursor.clone()),
            redacted_match: Some(intervention.redacted_match.clone()),
            intervention: Some(intervention.clone()),
            repeat_state: None,
            policy_refs: rule.policy_refs.clone(),
            redacted_summary: "stream rule matched redacted content".to_string(),
        }
    }

    /// Builds the repeat state value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn repeat_state(rule: &StreamRule, repeat_state: StreamRuleRepeatStateSnapshot) -> Self {
        Self {
            record_kind: StreamRuleRecordKind::RepeatState,
            rule_id: rule.id.clone(),
            rule_version: rule.version,
            channel: None,
            direction: None,
            cursor: None,
            redacted_match: None,
            intervention: None,
            repeat_state: Some(repeat_state),
            policy_refs: rule.policy_refs.clone(),
            redacted_summary: "stream rule repeat state snapshot".to_string(),
        }
    }

    /// Converts this value into journal record data.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn to_journal_record(&self, base: JournalRecordBase) -> JournalRecord {
        JournalRecord::feature_record(
            base,
            JournalRecordKind::StreamRule,
            "stream_rule",
            self.event_kind_name(),
            EntityRef::new(EntityKind::StreamRule, self.rule_id.as_str()),
            Vec::new(),
            Vec::new(),
            JournalRecordPayload::StreamRule(self.clone()),
        )
    }

    /// Returns the event kind name currently held by this value.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn event_kind_name(&self) -> &'static str {
        match self.record_kind {
            StreamRuleRecordKind::Registered => "stream_rule_registered",
            StreamRuleRecordKind::CompileFailed => "stream_rule_compile_failed",
            StreamRuleRecordKind::Matched => "stream_rule_matched",
            StreamRuleRecordKind::InterventionIntent => "stream_intervention_requested",
            StreamRuleRecordKind::InterventionResult => "stream_intervention_applied",
            StreamRuleRecordKind::RepeatState => "stream_rule_repeat_state_recorded",
        }
    }
}

/// Validates the records::stream invariants and returns a typed error
/// on failure. Validation is pure and does not perform I/O, dispatch,
/// journal appends, or adapter calls.
pub(crate) fn validate_safe_regex(pattern: &str) -> Result<(), AgentError> {
    if pattern.is_empty() {
        return Err(AgentError::missing_required_field(
            "stream_matcher.regex.pattern",
        ));
    }
    if pattern.len() > 512 {
        return Err(AgentError::contract_violation(
            "stream regex pattern exceeds bounded length",
        ));
    }
    let forbidden = ["(?", "\\1", "\\2", "+)+", "*)+", "++", "**", "{0,"];
    if forbidden.iter().any(|needle| pattern.contains(needle)) {
        return Err(AgentError::contract_violation(
            "stream regex pattern uses unsupported or backtracking-prone syntax",
        ));
    }
    Ok(())
}

fn validate_window(window_bytes: u64) -> Result<(), AgentError> {
    if window_bytes == 0 || window_bytes > 64 * 1024 {
        return Err(AgentError::contract_violation(
            "stream matcher window_bytes must be between 1 and 65536",
        ));
    }
    Ok(())
}

/// Computes the stable hash rule fingerprint for this records::stream
/// value. The computation is deterministic and side-effect free so it
/// can be used in package, journal, or test evidence.
pub(crate) fn hash_rule_fingerprint(rule: &StreamRule) -> Result<String, AgentError> {
    let bytes = serde_json::to_vec(rule)
        .map_err(|error| AgentError::contract_violation(error.to_string()))?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

/// Builds the safe id fragment value.
/// This is data construction and performs no I/O, journal append, event publication, or process
pub(crate) fn safe_id_fragment(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '.' || character == '_' {
                character
            } else {
                '.'
            }
        })
        .collect()
}

/// Returns stream policy ref derived from the supplied state.
/// This is data-only and does not perform I/O, call host ports, append journals, publish
/// events, or start processes.
pub fn stream_policy_ref(id: impl Into<String>) -> PolicyRef {
    PolicyRef::with_kind(PolicyKind::RuntimePackage, id)
}
