use serde::{Deserialize, Serialize};

use crate::{
    domain::{
        AgentId, EffectId, EntityKind, EntityRef, PolicyRef, PrivacyClass, RunId, SourceKind,
        SourceRef,
    },
    effect::{EffectIntent, EffectKind, EffectResult},
    journal::{
        EventIndexProjection, JOURNAL_SCHEMA_VERSION, JournalRecord, JournalRecordBase,
        JournalRecordKind, JournalRecordPayload,
    },
    package_hooks::{HookId, HookPoint, HookResponseClass, HookSpec},
};

pub const HOOK_RECORD_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HookRecord {
    pub schema_version: u16,
    pub hook_id: HookId,
    pub point: HookPoint,
    pub payload: HookRecordPayload,
}

impl HookRecord {
    pub fn registered(spec: &HookSpec) -> Result<Self, crate::domain::AgentError> {
        Ok(Self {
            schema_version: HOOK_RECORD_SCHEMA_VERSION,
            hook_id: spec.hook_id.clone(),
            point: spec.point.clone(),
            payload: HookRecordPayload::Registered {
                spec_hash: spec.spec_hash()?,
                executor_ref: spec.executor_ref.as_str().to_string(),
                policy_ref: spec.policy_ref.clone(),
            },
        })
    }

    pub fn invocation_started(spec: &HookSpec, invocation_id: impl Into<String>) -> Self {
        Self {
            schema_version: HOOK_RECORD_SCHEMA_VERSION,
            hook_id: spec.hook_id.clone(),
            point: spec.point.clone(),
            payload: HookRecordPayload::InvocationStarted {
                invocation_id: invocation_id.into(),
            },
        }
    }

    pub fn completed(spec: &HookSpec, invocation_id: impl Into<String>, elapsed_ms: u64) -> Self {
        Self {
            schema_version: HOOK_RECORD_SCHEMA_VERSION,
            hook_id: spec.hook_id.clone(),
            point: spec.point.clone(),
            payload: HookRecordPayload::Completed {
                invocation_id: invocation_id.into(),
                elapsed_ms,
            },
        }
    }

    pub fn timeout(spec: &HookSpec, invocation_id: impl Into<String>, elapsed_ms: u64) -> Self {
        Self {
            schema_version: HOOK_RECORD_SCHEMA_VERSION,
            hook_id: spec.hook_id.clone(),
            point: spec.point.clone(),
            payload: HookRecordPayload::TimedOut {
                invocation_id: invocation_id.into(),
                elapsed_ms,
                failure_policy: format!("{:?}", spec.failure),
            },
        }
    }

    pub fn cancelled(spec: &HookSpec, invocation_id: impl Into<String>) -> Self {
        Self {
            schema_version: HOOK_RECORD_SCHEMA_VERSION,
            hook_id: spec.hook_id.clone(),
            point: spec.point.clone(),
            payload: HookRecordPayload::Cancelled {
                invocation_id: invocation_id.into(),
            },
        }
    }

    pub fn failed(
        spec: &HookSpec,
        invocation_id: impl Into<String>,
        redacted_summary: impl Into<String>,
    ) -> Self {
        Self {
            schema_version: HOOK_RECORD_SCHEMA_VERSION,
            hook_id: spec.hook_id.clone(),
            point: spec.point.clone(),
            payload: HookRecordPayload::Failed {
                invocation_id: invocation_id.into(),
                failure_policy: format!("{:?}", spec.failure),
                redacted_summary: redacted_summary.into(),
            },
        }
    }

    pub fn response_decision(
        spec: &HookSpec,
        invocation_id: impl Into<String>,
        decision: HookResponseDecision,
        response_class: HookResponseClass,
        target_domain_refs: Vec<EntityRef>,
    ) -> Self {
        Self {
            schema_version: HOOK_RECORD_SCHEMA_VERSION,
            hook_id: spec.hook_id.clone(),
            point: spec.point.clone(),
            payload: HookRecordPayload::ResponseDecision {
                invocation_id: invocation_id.into(),
                decision,
                response_class,
                mutation_right_decision: "checked_against_point_and_spec_rights".to_string(),
                target_domain_refs,
            },
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HookRecordPayload {
    Registered {
        spec_hash: String,
        executor_ref: String,
        policy_ref: PolicyRef,
    },
    InvocationStarted {
        invocation_id: String,
    },
    Completed {
        invocation_id: String,
        elapsed_ms: u64,
    },
    TimedOut {
        invocation_id: String,
        elapsed_ms: u64,
        failure_policy: String,
    },
    Cancelled {
        invocation_id: String,
    },
    Failed {
        invocation_id: String,
        failure_policy: String,
        redacted_summary: String,
    },
    ResponseDecision {
        invocation_id: String,
        decision: HookResponseDecision,
        response_class: HookResponseClass,
        mutation_right_decision: String,
        target_domain_refs: Vec<EntityRef>,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HookResponseDecision {
    AcceptedJournaledBeforeApply,
    RejectedMutationRight,
    RejectedPointMatrix,
    RejectedPolicy,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HookMutationJournalPlan {
    pub hook_record: HookRecord,
    pub effect_intent: EffectIntent,
    pub hook_journal_record: JournalRecord,
    pub intent_journal_record: JournalRecord,
    pub result_journal_record: JournalRecord,
}

impl HookMutationJournalPlan {
    pub fn accepted_response(
        journal_seq: u64,
        record_id: impl Into<String>,
        run_id: RunId,
        agent_id: AgentId,
        source: SourceRef,
        spec: &HookSpec,
        invocation_id: impl Into<String>,
        response_class: HookResponseClass,
        runtime_package_fingerprint: impl Into<String>,
    ) -> Self {
        let invocation_id = invocation_id.into();
        let hook_record = HookRecord::response_decision(
            spec,
            invocation_id.clone(),
            HookResponseDecision::AcceptedJournaledBeforeApply,
            response_class.clone(),
            vec![hook_entity_ref(&spec.hook_id)],
        );
        let effect_id = EffectId::new(format!(
            "effect.hook.{}.{}",
            spec.hook_id.as_str(),
            invocation_id
        ));
        let mut intent = EffectIntent::new(
            effect_id,
            EffectKind::HookMutation,
            hook_entity_ref(&spec.hook_id),
            SourceRef::with_kind(SourceKind::Hook, spec.hook_id.as_str()),
            format!("accepted hook response {:?}", response_class),
        );
        intent.policy_refs = vec![spec.policy_ref.clone()];

        let record_id = record_id.into();
        let runtime_package_fingerprint = runtime_package_fingerprint.into();
        let mut hook_base = JournalRecordBase::new(
            journal_seq,
            format!("{record_id}.response"),
            run_id.clone(),
            agent_id.clone(),
            source.clone(),
        );
        hook_base.runtime_package_fingerprint = runtime_package_fingerprint.clone();
        hook_base.privacy = PrivacyClass::ContentRefsOnly;
        hook_base.redaction_policy_id = "policy.redaction.hook.default".to_string();
        let mut intent_base = JournalRecordBase::new(
            journal_seq + 1,
            format!("{record_id}.intent"),
            run_id.clone(),
            agent_id.clone(),
            source.clone(),
        );
        intent_base.runtime_package_fingerprint = runtime_package_fingerprint.clone();
        intent_base.privacy = PrivacyClass::ContentRefsOnly;
        intent_base.redaction_policy_id = "policy.redaction.hook.default".to_string();
        let mut result_base = JournalRecordBase::new(
            journal_seq + 2,
            format!("{record_id}.result"),
            run_id,
            agent_id,
            source,
        );
        result_base.runtime_package_fingerprint = runtime_package_fingerprint;
        result_base.privacy = PrivacyClass::ContentRefsOnly;
        result_base.redaction_policy_id = "policy.redaction.hook.default".to_string();

        let hook_journal_record = hook_journal_record(
            hook_base,
            hook_record.clone(),
            SourceRef::with_kind(SourceKind::Hook, spec.hook_id.as_str()),
            "hook_response_decision",
        );
        let intent_journal_record = JournalRecord::effect_intent(intent_base, intent.clone());
        let result = EffectResult::completed(
            intent.effect_id.clone(),
            format!(
                "accepted hook response {:?} journaled before apply",
                response_class
            ),
        );
        let result_journal_record = JournalRecord::effect_result(result_base, result);
        Self {
            hook_record,
            effect_intent: intent,
            hook_journal_record,
            intent_journal_record,
            result_journal_record,
        }
    }
}

pub fn hook_entity_ref(hook_id: &HookId) -> EntityRef {
    EntityRef::new(EntityKind::Hook, hook_id.as_str())
}

fn hook_journal_record(
    base: JournalRecordBase,
    record: HookRecord,
    source: SourceRef,
    event_kind: impl Into<String>,
) -> JournalRecord {
    let subject_ref = hook_entity_ref(&record.hook_id);
    JournalRecord {
        journal_schema_version: JOURNAL_SCHEMA_VERSION,
        journal_seq: base.journal_seq,
        record_id: base.record_id,
        record_kind: JournalRecordKind::Hook,
        run_id: base.run_id.clone(),
        agent_id: base.agent_id.clone(),
        turn_id: base.turn_id.clone(),
        attempt_id: base.attempt_id.clone(),
        subject_ref: subject_ref.clone(),
        related_refs: Vec::new(),
        causal_refs: base.causal_refs,
        source: source.clone(),
        destination: base.destination.clone(),
        correlation_keys: Vec::new(),
        tags: vec!["hook".to_string()],
        delivery_semantics: "journal_backed".to_string(),
        event_index: EventIndexProjection {
            run_id: base.run_id,
            agent_id: base.agent_id,
            turn_id: base.turn_id,
            event_family: "hook".to_string(),
            event_kind: event_kind.into(),
            source,
            destination: base.destination,
            subject_ref: subject_ref.clone(),
            related_refs: Vec::new(),
            correlation_keys: Vec::new(),
            tags: vec!["hook".to_string()],
            privacy_class: base.privacy.clone(),
            delivery_semantics: "journal_backed".to_string(),
        },
        timestamp_millis: base.timestamp_millis,
        runtime_package_fingerprint: base.runtime_package_fingerprint,
        privacy: base.privacy,
        content_refs: Vec::new(),
        redaction_policy_id: base.redaction_policy_id,
        idempotency_key: None,
        dedupe_key: None,
        checkpoint_ref: base.checkpoint_ref,
        payload: JournalRecordPayload::Hook(record),
    }
}
