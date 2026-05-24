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

typed_stream_id!(StreamDeltaId);
typed_stream_id!(StreamRuleId);
typed_stream_id!(StreamInterventionId);
typed_stream_id!(StreamMatchId);
typed_stream_id!(MarkerId);
typed_stream_id!(MarkerVersion);
typed_stream_id!(MatcherEngineRef);

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct RuleVersion(pub u32);

impl RuleVersion {
    pub fn new(version: u32) -> Self {
        assert!(version > 0, "RuleVersion must be nonzero");
        Self(version)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamChannel {
    AssistantText,
    ReasoningSummary,
    ProviderExposedReasoning,
    ToolCallArguments,
    ToolResultText,
    RealtimeTranscript,
    RealtimeMedia,
    HiddenChainOfThought,
}

impl StreamChannel {
    pub fn is_policy_visible(&self) -> bool {
        !matches!(self, Self::HiddenChainOfThought)
    }

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
pub enum StreamDirection {
    InputToProvider,
    OutputFromProvider,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamCursorPrecision {
    ByteOffset,
    ChunkSequenceOnly,
    Marker,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct StreamCursor {
    pub chunk_sequence: u64,
    pub byte_offset: u64,
    pub precision: StreamCursorPrecision,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

impl StreamCursor {
    pub fn chunk(chunk_sequence: u64) -> Self {
        Self {
            chunk_sequence,
            byte_offset: 0,
            precision: StreamCursorPrecision::ChunkSequenceOnly,
            label: None,
        }
    }

    pub fn byte(chunk_sequence: u64, byte_offset: u64) -> Self {
        Self {
            chunk_sequence,
            byte_offset,
            precision: StreamCursorPrecision::ByteOffset,
            label: None,
        }
    }

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
pub struct StreamDelta {
    pub delta_id: StreamDeltaId,
    pub run_id: RunId,
    pub agent_id: AgentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<TurnId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt_id: Option<AttemptId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<MessageId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<ToolCallId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub realtime_session_id: Option<crate::realtime_records::RealtimeSessionId>,
    pub channel: StreamChannel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<StreamDirection>,
    pub cursor: StreamCursor,
    pub source: SourceRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<DestinationRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policy_refs: Vec<PolicyRef>,
    pub privacy: PrivacyClass,
    pub retention: RetentionClass,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_ref: Option<ContentRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub marker_id: Option<MarkerId>,
    pub redacted_summary: String,
    pub runtime_package_fingerprint: String,
    #[serde(skip)]
    match_text: Option<String>,
}

impl StreamDelta {
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

    pub fn with_run(mut self, run_id: RunId, agent_id: AgentId) -> Self {
        self.run_id = run_id;
        self.agent_id = agent_id;
        self
    }

    pub fn with_turn(mut self, turn_id: TurnId) -> Self {
        self.turn_id = Some(turn_id);
        self
    }

    pub fn with_attempt(mut self, attempt_id: AttemptId) -> Self {
        self.attempt_id = Some(attempt_id);
        self
    }

    pub fn with_direction(mut self, direction: StreamDirection) -> Self {
        self.direction = Some(direction);
        self
    }

    pub fn with_destination(mut self, destination: DestinationRef) -> Self {
        self.destination = Some(destination);
        self
    }

    pub fn with_content_ref(mut self, content_ref: ContentRef) -> Self {
        self.content_ref = Some(content_ref);
        self
    }

    pub fn matcher_text(&self) -> Option<&str> {
        if !self.channel.is_policy_visible() || self.privacy == PrivacyClass::Secret {
            return None;
        }
        self.match_text.as_deref()
    }

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
pub struct StreamChannelSelector {
    pub channel: StreamChannel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<StreamDirection>,
}

impl StreamChannelSelector {
    pub fn channel(channel: StreamChannel) -> Self {
        Self {
            channel,
            direction: None,
        }
    }

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
pub enum RegexDialect {
    SafeSubset,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StreamMatcher {
    Literal {
        text: String,
        case_sensitive: bool,
        window_bytes: u64,
    },
    Regex {
        pattern: String,
        dialect: RegexDialect,
        window_bytes: u64,
        timeout_ms: u64,
    },
    Marker {
        marker_id: MarkerId,
        marker_version: MarkerVersion,
    },
    HostMatcher {
        matcher_ref: MatcherEngineRef,
        #[serde(skip_serializing_if = "Option::is_none")]
        risk_policy_ref: Option<PolicyRef>,
    },
}

impl StreamMatcher {
    pub fn literal(text: impl Into<String>, case_sensitive: bool, window_bytes: u64) -> Self {
        Self::Literal {
            text: text.into(),
            case_sensitive,
            window_bytes,
        }
    }

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

    pub fn marker(marker_id: impl Into<String>) -> Self {
        Self::Marker {
            marker_id: MarkerId::new(marker_id),
            marker_version: MarkerVersion::new("marker.v1"),
        }
    }

    pub fn window_bytes(&self) -> u64 {
        match self {
            Self::Literal { window_bytes, .. } | Self::Regex { window_bytes, .. } => *window_bytes,
            Self::Marker { .. } | Self::HostMatcher { .. } => 0,
        }
    }

    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::Literal { .. } => "literal",
            Self::Regex { .. } => "regex",
            Self::Marker { .. } => "marker",
            Self::HostMatcher { .. } => "host_matcher",
        }
    }

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
pub enum StreamRuleScope {
    Run,
    Turn,
    Attempt,
    RealtimeSession,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PartialOutputPolicy {
    Keep,
    Discard,
    Mask,
    ContentRefOnly,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RepeatPolicy {
    Always,
    OncePerRun,
    OncePerTurn,
    OncePerAttemptAndSpan,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchPrivacyPolicy {
    HashLengthAndSummary,
    RawCaptureIfPolicyAllows,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum StreamAction {
    StopRun {
        reason: String,
        partial_output: PartialOutputPolicy,
    },
    AbortAndRetry {
        injection_summary: String,
        retry_policy_ref: PolicyRef,
        partial_output: PartialOutputPolicy,
    },
    PauseForApproval {
        approval_policy_ref: PolicyRef,
        resume_policy_ref: PolicyRef,
    },
    MaskAndContinue {
        replacement: String,
    },
    EmitOnly {
        notice_kind: String,
    },
}

impl StreamAction {
    pub fn mask_and_continue(replacement: impl Into<String>) -> Self {
        Self::MaskAndContinue {
            replacement: replacement.into(),
        }
    }

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

    pub fn emit_only(notice_kind: impl Into<String>) -> Self {
        Self::EmitOnly {
            notice_kind: notice_kind.into(),
        }
    }

    pub fn action_kind(&self) -> &'static str {
        match self {
            Self::StopRun { .. } => "stop_run",
            Self::AbortAndRetry { .. } => "abort_and_retry",
            Self::PauseForApproval { .. } => "pause_for_approval",
            Self::MaskAndContinue { .. } => "mask_and_continue",
            Self::EmitOnly { .. } => "emit_only",
        }
    }

    pub fn partial_output_policy(&self) -> PartialOutputPolicy {
        match self {
            Self::StopRun { partial_output, .. } | Self::AbortAndRetry { partial_output, .. } => {
                partial_output.clone()
            }
            Self::PauseForApproval { .. } | Self::EmitOnly { .. } => PartialOutputPolicy::Keep,
            Self::MaskAndContinue { .. } => PartialOutputPolicy::Mask,
        }
    }

    pub fn effect_kind_hint(&self) -> Option<EffectKind> {
        match self {
            Self::AbortAndRetry { .. } => Some(EffectKind::ProviderRequest),
            Self::PauseForApproval { .. } => Some(EffectKind::ApprovalDispatch),
            Self::StopRun { .. } | Self::MaskAndContinue { .. } | Self::EmitOnly { .. } => None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StreamRule {
    pub id: StreamRuleId,
    pub version: RuleVersion,
    pub source: SourceRef,
    pub matcher: StreamMatcher,
    pub channels: Vec<StreamChannelSelector>,
    pub scope: StreamRuleScope,
    pub action: StreamAction,
    pub repeat: RepeatPolicy,
    pub privacy: MatchPrivacyPolicy,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policy_refs: Vec<PolicyRef>,
}

impl StreamRule {
    pub fn builder(id: StreamRuleId) -> StreamRuleBuilder {
        StreamRuleBuilder::new(id)
    }

    pub fn mask_regex(id: impl Into<String>, pattern: impl Into<String>) -> StreamRuleBuilder {
        StreamRuleBuilder::new(StreamRuleId::new(id))
            .source(SourceRef::with_kind(
                SourceKind::Host,
                "source.host.stream_rules",
            ))
            .matcher(StreamMatcher::regex_with_limits(pattern, 4096, 25))
            .action(StreamAction::mask_and_continue("[redacted]"))
    }

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
pub struct StreamRuleBuilder {
    rule: StreamRule,
}

impl StreamRuleBuilder {
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

    pub fn source(mut self, source: SourceRef) -> Self {
        self.rule.source = source;
        self
    }

    pub fn matcher(mut self, matcher: StreamMatcher) -> Self {
        self.rule.matcher = matcher;
        self
    }

    pub fn on(mut self, channel: StreamChannel) -> Self {
        self.rule
            .channels
            .push(StreamChannelSelector::channel(channel));
        self
    }

    pub fn action(mut self, action: StreamAction) -> Self {
        self.rule.action = action;
        self
    }

    pub fn repeat(mut self, repeat: RepeatPolicy) -> Self {
        self.rule.repeat = repeat;
        self
    }

    pub fn policy(mut self, policy_ref: PolicyRef) -> Self {
        self.rule.policy_refs.push(policy_ref);
        self
    }

    pub fn build(self) -> Result<StreamRule, AgentError> {
        self.rule.validate()?;
        Ok(self.rule)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RedactedMatch {
    pub match_id: StreamMatchId,
    pub channel: StreamChannel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<StreamDirection>,
    pub cursor: StreamCursor,
    pub byte_len: usize,
    pub text_hash: String,
    pub redacted_summary: String,
    pub privacy: PrivacyClass,
    pub retention: RetentionClass,
}

impl RedactedMatch {
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
pub struct StreamMatchRef {
    pub match_id: StreamMatchId,
    pub rule_id: StreamRuleId,
    pub rule_version: RuleVersion,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StreamIntervention {
    pub intervention_id: StreamInterventionId,
    pub rule_ref: EntityRef,
    pub requested_action: StreamAction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applied_action: Option<StreamAction>,
    pub match_ref: StreamMatchRef,
    pub redacted_match: RedactedMatch,
    pub partial_output_policy: PartialOutputPolicy,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policy_refs: Vec<PolicyRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect_intent_ref: Option<EffectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect_result_ref: Option<EffectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect_intent: Option<EffectIntent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect_result: Option<EffectResult>,
}

impl StreamIntervention {
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

    pub fn with_effect_result(mut self, result: EffectResult) -> Self {
        self.effect_result_ref = Some(result.effect_id.clone());
        self.effect_result = Some(result);
        self
    }

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
pub enum StreamRuleRecordKind {
    Registered,
    CompileFailed,
    Matched,
    InterventionIntent,
    InterventionResult,
    RepeatState,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StreamRuleRepeatStateSnapshot {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub seen_match_keys: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StreamRuleRecord {
    pub record_kind: StreamRuleRecordKind,
    pub rule_id: StreamRuleId,
    pub rule_version: RuleVersion,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<StreamChannel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<StreamDirection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<StreamCursor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redacted_match: Option<RedactedMatch>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intervention: Option<StreamIntervention>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repeat_state: Option<StreamRuleRepeatStateSnapshot>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policy_refs: Vec<PolicyRef>,
    pub redacted_summary: String,
}

impl StreamRuleRecord {
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

pub(crate) fn hash_rule_fingerprint(rule: &StreamRule) -> Result<String, AgentError> {
    let bytes = serde_json::to_vec(rule)
        .map_err(|error| AgentError::contract_violation(error.to_string()))?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

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

pub fn stream_policy_ref(id: impl Into<String>) -> PolicyRef {
    PolicyRef::with_kind(PolicyKind::RuntimePackage, id)
}
