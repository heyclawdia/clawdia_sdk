use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    domain::{
        AgentId, AttemptId, ContentRef, DedupeKey, DestinationRef, EffectId, EntityKind, EntityRef,
        IdempotencyKey, MessageId, PolicyRef, PrivacyClass, RetentionClass, RunId, SourceRef,
        TurnId, ValidatedOutputId,
    },
    effect::{EffectIntent, EffectKind, EffectResult, EffectTerminalStatus},
    error::RetryClassification,
    journal::{
        EventIndexProjection, JOURNAL_SCHEMA_VERSION, JournalRecord, JournalRecordKind,
        JournalRecordPayload,
    },
    package::RuntimePackageFingerprint,
};

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct OutputDeliveryId(String);

impl OutputDeliveryId {
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        assert!(!value.is_empty(), "OutputDeliveryId must not be empty");
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct OutputSinkRef(String);

impl OutputSinkRef {
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        assert!(!value.is_empty(), "OutputSinkRef must not be empty");
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputDeliveryRequirement {
    Disabled,
    Optional,
    Required,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutputDeliveryKind {
    StreamChunk {
        stream_cursor: String,
        chunk_index: u64,
    },
    FinalMessage,
    FinalValidatedOutput,
}

impl OutputDeliveryKind {
    pub fn is_chunk(&self) -> bool {
        matches!(self, Self::StreamChunk { .. })
    }

    pub(crate) fn dedupe_fragment(&self) -> String {
        match self {
            Self::StreamChunk {
                stream_cursor,
                chunk_index,
            } => format!("stream:{stream_cursor}:{chunk_index}"),
            Self::FinalMessage => "final:message".to_string(),
            Self::FinalValidatedOutput => "final:validated_output".to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputContentMode {
    ContentRefsOnly,
    RedactedSummary,
    RawContentIfPolicyAllows,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OutputDeliveryPolicy {
    pub policy_ref: PolicyRef,
    pub requirement: OutputDeliveryRequirement,
    pub default_content_mode: OutputContentMode,
    #[serde(default)]
    pub allowed_content_modes: Vec<OutputContentMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_sink_ref: Option<OutputSinkRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_policy_ref: Option<PolicyRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reconciliation_policy_ref: Option<PolicyRef>,
    pub raw_content_policy: RawOutputContentPolicy,
}

impl OutputDeliveryPolicy {
    pub fn required(policy_ref: PolicyRef, sink_ref: OutputSinkRef) -> Self {
        Self {
            policy_ref,
            requirement: OutputDeliveryRequirement::Required,
            default_content_mode: OutputContentMode::ContentRefsOnly,
            allowed_content_modes: vec![
                OutputContentMode::ContentRefsOnly,
                OutputContentMode::RedactedSummary,
            ],
            required_sink_ref: Some(sink_ref),
            retry_policy_ref: None,
            reconciliation_policy_ref: None,
            raw_content_policy: RawOutputContentPolicy::deny(),
        }
    }

    pub fn optional(policy_ref: PolicyRef) -> Self {
        Self {
            policy_ref,
            requirement: OutputDeliveryRequirement::Optional,
            default_content_mode: OutputContentMode::RedactedSummary,
            allowed_content_modes: vec![OutputContentMode::RedactedSummary],
            required_sink_ref: None,
            retry_policy_ref: None,
            reconciliation_policy_ref: None,
            raw_content_policy: RawOutputContentPolicy::deny(),
        }
    }

    pub fn disabled(policy_ref: PolicyRef) -> Self {
        Self {
            policy_ref,
            requirement: OutputDeliveryRequirement::Disabled,
            default_content_mode: OutputContentMode::ContentRefsOnly,
            allowed_content_modes: Vec::new(),
            required_sink_ref: None,
            retry_policy_ref: None,
            reconciliation_policy_ref: None,
            raw_content_policy: RawOutputContentPolicy::deny(),
        }
    }

    pub fn allows_mode(&self, mode: OutputContentMode) -> bool {
        self.allowed_content_modes.contains(&mode)
    }

    pub fn policy_refs(&self) -> Vec<PolicyRef> {
        let mut refs = vec![self.policy_ref.clone()];
        if let Some(policy_ref) = &self.retry_policy_ref {
            refs.push(policy_ref.clone());
        }
        if let Some(policy_ref) = &self.reconciliation_policy_ref {
            refs.push(policy_ref.clone());
        }
        refs
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RawOutputContentPolicy {
    pub policy_ref: PolicyRef,
    pub allow_raw_content: bool,
    pub retention_named: bool,
    pub redaction_policy_named: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_sink_ref: Option<OutputSinkRef>,
    pub byte_limit: u64,
}

impl RawOutputContentPolicy {
    pub fn deny() -> Self {
        Self {
            policy_ref: PolicyRef::new("policy.output_delivery.raw.deny"),
            allow_raw_content: false,
            retention_named: true,
            redaction_policy_named: true,
            allowed_sink_ref: None,
            byte_limit: 0,
        }
    }

    pub fn allow_for_sink(policy_ref: PolicyRef, sink_ref: OutputSinkRef, byte_limit: u64) -> Self {
        Self {
            policy_ref,
            allow_raw_content: true,
            retention_named: true,
            redaction_policy_named: true,
            allowed_sink_ref: Some(sink_ref),
            byte_limit,
        }
    }

    pub fn allows_raw_for(&self, sink_ref: &OutputSinkRef, byte_len: usize) -> bool {
        self.allow_raw_content
            && self.retention_named
            && self.redaction_policy_named
            && self
                .allowed_sink_ref
                .as_ref()
                .is_some_and(|allowed| allowed == sink_ref)
            && self.byte_limit >= byte_len as u64
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OutputDeliveryRequest {
    pub delivery_id: OutputDeliveryId,
    pub effect_id: EffectId,
    pub run_id: RunId,
    pub agent_id: AgentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<TurnId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt_id: Option<AttemptId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_message_id: Option<MessageId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validated_output_id: Option<ValidatedOutputId>,
    pub destination: DestinationRef,
    pub sink_ref: OutputSinkRef,
    pub delivery_kind: OutputDeliveryKind,
    pub content_mode: OutputContentMode,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content_refs: Vec<ContentRef>,
    pub redacted_summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_content: Option<String>,
    pub privacy: PrivacyClass,
    pub retention: RetentionClass,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policy_refs: Vec<PolicyRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<IdempotencyKey>,
    pub dedupe_key: DedupeKey,
    pub runtime_package_fingerprint: RuntimePackageFingerprint,
}

impl OutputDeliveryRequest {
    pub fn effect_intent(&self) -> EffectIntent {
        let mut intent = EffectIntent::new(
            self.effect_id.clone(),
            EffectKind::OutputDelivery,
            EntityRef::new(EntityKind::OutputDelivery, self.delivery_id.as_str()),
            SourceRef::with_kind(crate::domain::SourceKind::Sdk, "source.sdk.output_delivery"),
            self.redacted_summary.clone(),
        );
        intent.destination = Some(self.destination.clone());
        intent.policy_refs = self.policy_refs.clone();
        intent.idempotency_key = self.idempotency_key.clone();
        intent.dedupe_key = Some(self.dedupe_key.clone());
        intent.content_refs = self.content_refs.clone();
        intent
    }

    pub fn carries_raw_content(&self) -> bool {
        self.raw_content.is_some()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OutputDeliveryReceipt {
    pub delivery_id: OutputDeliveryId,
    pub status: OutputDispatchStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ack_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination_cursor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_operation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reconciliation_ref: Option<String>,
    pub redacted_summary: String,
}

impl OutputDeliveryReceipt {
    pub fn completed(delivery_id: OutputDeliveryId, ack_ref: impl Into<String>) -> Self {
        Self {
            delivery_id,
            status: OutputDispatchStatus::Completed,
            ack_ref: Some(ack_ref.into()),
            destination_cursor: None,
            external_operation_id: None,
            reconciliation_ref: None,
            redacted_summary: "output delivery completed".to_string(),
        }
    }

    pub fn unknown(delivery_id: OutputDeliveryId, reconciliation_ref: impl Into<String>) -> Self {
        Self {
            delivery_id,
            status: OutputDispatchStatus::ReconciliationNeeded,
            ack_ref: None,
            destination_cursor: None,
            external_operation_id: None,
            reconciliation_ref: Some(reconciliation_ref.into()),
            redacted_summary: "output delivery outcome unknown".to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputDispatchStatus {
    Requested,
    Completed,
    Failed,
    Deduped,
    HostConfigurationNeeded,
    PolicyDenied,
    SkippedOptional,
    ReconciliationNeeded,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputDeliveryEventKind {
    OutputDispatchRequested,
    OutputDispatchCompleted,
    OutputDispatchFailed,
    OutputDispatchDeduped,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OutputDeliveryIntentRecord {
    pub delivery_id: OutputDeliveryId,
    pub effect_intent: EffectIntent,
    pub destination: DestinationRef,
    pub sink_ref: OutputSinkRef,
    pub desired_sink_ref: OutputSinkRef,
    pub delivery_kind: OutputDeliveryKind,
    pub content_mode: OutputContentMode,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content_refs: Vec<ContentRef>,
    pub redacted_summary: String,
    pub privacy: PrivacyClass,
    pub retention: RetentionClass,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policy_refs: Vec<PolicyRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<IdempotencyKey>,
    pub dedupe_key: DedupeKey,
    pub runtime_package_fingerprint: RuntimePackageFingerprint,
}

impl OutputDeliveryIntentRecord {
    pub fn from_request(request: &OutputDeliveryRequest) -> Self {
        Self {
            delivery_id: request.delivery_id.clone(),
            effect_intent: request.effect_intent(),
            destination: request.destination.clone(),
            sink_ref: request.sink_ref.clone(),
            desired_sink_ref: request.sink_ref.clone(),
            delivery_kind: request.delivery_kind.clone(),
            content_mode: request.content_mode,
            content_refs: request.content_refs.clone(),
            redacted_summary: request.redacted_summary.clone(),
            privacy: request.privacy.clone(),
            retention: request.retention.clone(),
            policy_refs: request.policy_refs.clone(),
            idempotency_key: request.idempotency_key.clone(),
            dedupe_key: request.dedupe_key.clone(),
            runtime_package_fingerprint: request.runtime_package_fingerprint.clone(),
        }
    }

    pub fn to_journal_record(&self, base: OutputDeliveryJournalBase) -> JournalRecord {
        output_delivery_effect_record(
            base,
            JournalRecordKind::OutputDispatch,
            "output_dispatch_requested",
            Some(self.idempotency_key.clone()).flatten(),
            Some(self.dedupe_key.clone()),
            self.effect_intent.content_refs.clone(),
            JournalRecordPayload::OutputDelivery(OutputDeliveryRecord::Intent(self.clone())),
            EntityRef::new(EntityKind::OutputDelivery, self.delivery_id.as_str()),
            self.destination.clone(),
            self.policy_refs.clone(),
            self.privacy.clone(),
        )
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OutputDeliveryResultRecord {
    pub delivery_id: OutputDeliveryId,
    pub effect_result: EffectResult,
    pub destination: DestinationRef,
    pub sink_ref: OutputSinkRef,
    pub dispatch_status: OutputDispatchStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ack_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_operation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content_refs: Vec<ContentRef>,
    pub redacted_summary: String,
    pub retry_classification: RetryClassification,
}

impl OutputDeliveryResultRecord {
    pub fn completed(request: &OutputDeliveryRequest, receipt: &OutputDeliveryReceipt) -> Self {
        Self::from_status(
            request,
            OutputDispatchStatus::Completed,
            EffectTerminalStatus::Completed,
            receipt.ack_ref.clone(),
            receipt.external_operation_id.clone(),
            None,
            None,
            RetryClassification::NotRetryable,
            receipt.redacted_summary.clone(),
        )
    }

    pub fn failed(
        request: &OutputDeliveryRequest,
        status: OutputDispatchStatus,
        error_ref: impl Into<String>,
        retry_classification: RetryClassification,
    ) -> Self {
        Self::from_status(
            request,
            status,
            EffectTerminalStatus::Failed,
            None,
            None,
            None,
            Some(error_ref.into()),
            retry_classification,
            "output delivery failed before host send or at sink boundary",
        )
    }

    pub fn reconciliation_needed(
        request: &OutputDeliveryRequest,
        receipt: &OutputDeliveryReceipt,
    ) -> Self {
        Self::from_status(
            request,
            OutputDispatchStatus::ReconciliationNeeded,
            EffectTerminalStatus::Unknown,
            receipt.ack_ref.clone(),
            receipt.external_operation_id.clone(),
            receipt.reconciliation_ref.clone(),
            None,
            RetryClassification::RepairNeeded,
            receipt.redacted_summary.clone(),
        )
    }

    fn from_status(
        request: &OutputDeliveryRequest,
        dispatch_status: OutputDispatchStatus,
        terminal_status: EffectTerminalStatus,
        ack_ref: Option<String>,
        external_operation_id: Option<String>,
        reconciliation_ref: Option<String>,
        error_ref: Option<String>,
        retry_classification: RetryClassification,
        redacted_summary: impl Into<String>,
    ) -> Self {
        let redacted_summary = redacted_summary.into();
        Self {
            delivery_id: request.delivery_id.clone(),
            effect_result: EffectResult {
                effect_id: request.effect_id.clone(),
                terminal_status,
                external_operation_id: external_operation_id.clone(),
                reconciliation_ref,
                error_ref: error_ref.clone(),
                content_refs: request.content_refs.clone(),
                redacted_summary: redacted_summary.clone(),
            },
            destination: request.destination.clone(),
            sink_ref: request.sink_ref.clone(),
            dispatch_status,
            ack_ref,
            external_operation_id,
            error_ref,
            content_refs: request.content_refs.clone(),
            redacted_summary,
            retry_classification,
        }
    }

    pub fn to_journal_record(&self, base: OutputDeliveryJournalBase) -> JournalRecord {
        let event_kind = match self.dispatch_status {
            OutputDispatchStatus::Completed => "output_dispatch_completed",
            OutputDispatchStatus::HostConfigurationNeeded
            | OutputDispatchStatus::PolicyDenied
            | OutputDispatchStatus::Failed => "output_dispatch_failed",
            OutputDispatchStatus::ReconciliationNeeded => "output_dispatch_reconciliation_needed",
            OutputDispatchStatus::Deduped => "output_dispatch_deduped",
            OutputDispatchStatus::Requested | OutputDispatchStatus::SkippedOptional => {
                "output_dispatch_status"
            }
        };
        output_delivery_effect_record(
            base,
            JournalRecordKind::OutputDispatch,
            event_kind,
            None,
            None,
            self.effect_result.content_refs.clone(),
            JournalRecordPayload::OutputDelivery(OutputDeliveryRecord::Result(self.clone())),
            EntityRef::new(EntityKind::OutputDelivery, self.delivery_id.as_str()),
            self.destination.clone(),
            Vec::new(),
            PrivacyClass::ContentRefsOnly,
        )
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OutputDeliveryDedupeRecord {
    pub delivery_id: OutputDeliveryId,
    pub dedupe_key: DedupeKey,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prior_delivery_id: Option<OutputDeliveryId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prior_external_operation_id: Option<String>,
    pub prior_terminal_status: OutputDispatchStatus,
    pub current_status: OutputDispatchStatus,
    pub redacted_summary: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policy_refs: Vec<PolicyRef>,
}

impl OutputDeliveryDedupeRecord {
    pub fn to_journal_record(
        &self,
        base: OutputDeliveryJournalBase,
        destination: DestinationRef,
    ) -> JournalRecord {
        output_delivery_effect_record(
            base,
            JournalRecordKind::OutputDispatch,
            "output_dispatch_deduped",
            None,
            Some(self.dedupe_key.clone()),
            Vec::new(),
            JournalRecordPayload::OutputDelivery(OutputDeliveryRecord::Dedupe(self.clone())),
            EntityRef::new(EntityKind::OutputDelivery, self.delivery_id.as_str()),
            destination,
            self.policy_refs.clone(),
            PrivacyClass::ContentRefsOnly,
        )
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OutputDeliveryReconciliationRecord {
    pub delivery_id: OutputDeliveryId,
    pub intent_record_id: String,
    pub side_effect_kind: EffectKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<IdempotencyKey>,
    pub dedupe_key: DedupeKey,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_operation_id: Option<String>,
    pub terminal_status: OutputDispatchStatus,
    pub terminal_append_status: TerminalAppendStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reconciliation_adapter: Option<OutputSinkRef>,
    pub unsafe_pending_reason: String,
    pub replay_decision: ReplayRepairDecision,
    pub resend_allowed: bool,
}

impl OutputDeliveryReconciliationRecord {
    pub fn to_journal_record(
        &self,
        base: OutputDeliveryJournalBase,
        destination: DestinationRef,
    ) -> JournalRecord {
        output_delivery_effect_record(
            base,
            JournalRecordKind::OutputDispatch,
            "output_dispatch_reconciliation_needed",
            self.idempotency_key.clone(),
            Some(self.dedupe_key.clone()),
            Vec::new(),
            JournalRecordPayload::OutputDelivery(OutputDeliveryRecord::Reconciliation(
                self.clone(),
            )),
            EntityRef::new(EntityKind::OutputDelivery, self.delivery_id.as_str()),
            destination,
            Vec::new(),
            PrivacyClass::ContentRefsOnly,
        )
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalAppendStatus {
    NotAttempted,
    Appended,
    AppendFailed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplayRepairDecision {
    CompletedByDedupeProof,
    RequiresHostReconciliation,
    UnsafePending,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OutputDeliveryEventRecord {
    pub event_kind: OutputDeliveryEventKind,
    pub delivery_id: OutputDeliveryId,
    pub destination: DestinationRef,
    pub sink_ref: OutputSinkRef,
    pub dedupe_key: DedupeKey,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_message_id: Option<MessageId>,
    pub dispatch_status: OutputDispatchStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ack_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reconciliation_status: Option<ReplayRepairDecision>,
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "record_type", content = "record", rename_all = "snake_case")]
pub enum OutputDeliveryRecord {
    Intent(OutputDeliveryIntentRecord),
    Result(OutputDeliveryResultRecord),
    Dedupe(OutputDeliveryDedupeRecord),
    Reconciliation(OutputDeliveryReconciliationRecord),
    Event(OutputDeliveryEventRecord),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OutputDeliveryJournalBase {
    pub journal_seq: u64,
    pub record_id: String,
    pub run_id: RunId,
    pub agent_id: AgentId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<TurnId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt_id: Option<AttemptId>,
    pub source: SourceRef,
    pub destination: DestinationRef,
    pub timestamp_millis: u64,
    pub runtime_package_fingerprint: RuntimePackageFingerprint,
    pub redaction_policy_id: String,
}

pub fn build_output_delivery_dedupe_key(request: &OutputDeliveryRequest) -> DedupeKey {
    let content_refs = request
        .content_refs
        .iter()
        .map(|content_ref| content_ref.as_str())
        .collect::<Vec<_>>()
        .join(",");
    let policy_refs = request
        .policy_refs
        .iter()
        .map(|policy_ref| {
            format!(
                "{}:{}",
                policy_ref.as_str(),
                policy_ref.version.as_deref().unwrap_or("unversioned")
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let preimage = format!(
        "run={}|destination_kind={:?}|destination={}|sink={}|kind={}|message={}|validated={}|content={}|policies={}|package={}",
        request.run_id.as_str(),
        request.destination.kind,
        request.destination.as_str(),
        request.sink_ref.as_str(),
        request.delivery_kind.dedupe_fragment(),
        request
            .source_message_id
            .as_ref()
            .map(|id| id.as_str())
            .unwrap_or("none"),
        request
            .validated_output_id
            .as_ref()
            .map(|id| id.as_str())
            .unwrap_or("none"),
        content_refs,
        policy_refs,
        request.runtime_package_fingerprint.as_str(),
    );
    let digest = Sha256::digest(preimage.as_bytes());
    DedupeKey::new(format!("dedupe.output_delivery.{digest:x}"))
}

fn output_delivery_effect_record(
    base: OutputDeliveryJournalBase,
    record_kind: JournalRecordKind,
    event_kind: &str,
    idempotency_key: Option<IdempotencyKey>,
    dedupe_key: Option<DedupeKey>,
    content_refs: Vec<ContentRef>,
    payload: JournalRecordPayload,
    subject_ref: EntityRef,
    destination: DestinationRef,
    _policy_refs: Vec<PolicyRef>,
    privacy: PrivacyClass,
) -> JournalRecord {
    let related_refs = content_refs
        .iter()
        .map(|content_ref| EntityRef::new(EntityKind::Content, content_ref.as_str()))
        .collect::<Vec<_>>();
    JournalRecord {
        journal_schema_version: JOURNAL_SCHEMA_VERSION,
        journal_seq: base.journal_seq,
        record_id: base.record_id,
        record_kind,
        run_id: base.run_id.clone(),
        agent_id: base.agent_id.clone(),
        turn_id: base.turn_id.clone(),
        attempt_id: base.attempt_id.clone(),
        subject_ref: subject_ref.clone(),
        related_refs: related_refs.clone(),
        causal_refs: Vec::new(),
        source: base.source.clone(),
        destination: Some(destination.clone()),
        correlation_keys: Vec::new(),
        tags: vec!["output_delivery".to_string()],
        delivery_semantics: "journal_backed".to_string(),
        event_index: EventIndexProjection {
            run_id: base.run_id,
            agent_id: base.agent_id,
            turn_id: base.turn_id,
            event_family: "output_delivery".to_string(),
            event_kind: event_kind.to_string(),
            source: base.source,
            destination: Some(destination),
            subject_ref,
            related_refs,
            correlation_keys: Vec::new(),
            tags: vec!["output_delivery".to_string()],
            privacy_class: privacy.clone(),
            delivery_semantics: "journal_backed".to_string(),
        },
        timestamp_millis: base.timestamp_millis,
        runtime_package_fingerprint: base.runtime_package_fingerprint.as_str().to_string(),
        privacy,
        content_refs,
        redaction_policy_id: base.redaction_policy_id,
        idempotency_key,
        dedupe_key,
        checkpoint_ref: None,
        payload,
    }
}
