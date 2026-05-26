//! Durable and observable SDK records. Use these DTOs for events, journals, effects,
//! context, output, and feature evidence. Constructing records is data-only;
//! persistence, publication, and external actions happen through ports or application
//! coordinators. This file contains the hooks portion of that contract.
//!
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

/// Constant value for the records::hooks contract. Use it to keep SDK
/// records and tests aligned on the same stable value.
pub const HOOK_RECORD_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the hook record record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct HookRecord {
    /// Wire schema version used for compatibility checks.
    pub schema_version: u16,
    /// Stable hook id used for typed lineage, lookup, or dedupe.
    pub hook_id: HookId,
    /// Point used by this record or request.
    pub point: HookPoint,
    /// Payload carried by this record.
    /// Use the surrounding policy and redaction fields to decide whether it can be exposed.
    pub payload: HookRecordPayload,
}

impl HookRecord {
    /// Registered.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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

    /// Builds the invocation started value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
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

    /// Returns an updated value with completed configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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

    /// Builds the timeout record or result value.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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

    /// Cancelled.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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

    /// Returns an updated value with failed configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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

    /// Returns the response decision currently held by this value.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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

    /// Builds a rejected response decision journal record.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    #[expect(
        clippy::too_many_arguments,
        reason = "hook response records are durable audit DTOs and keep their lineage fields explicit until a record-builder pass"
    )]
    pub fn rejected_response_journal_record(
        journal_seq: u64,
        record_id: impl Into<String>,
        run_id: RunId,
        agent_id: AgentId,
        source: SourceRef,
        spec: &HookSpec,
        invocation_id: impl Into<String>,
        decision: HookResponseDecision,
        response_class: HookResponseClass,
        runtime_package_fingerprint: impl Into<String>,
    ) -> (Self, JournalRecord) {
        let hook_record =
            Self::response_decision(spec, invocation_id, decision, response_class, Vec::new());
        let mut base = JournalRecordBase::new(
            journal_seq,
            format!("{}.response", record_id.into()),
            run_id,
            agent_id,
            source,
        );
        base.runtime_package_fingerprint = runtime_package_fingerprint.into();
        base.privacy = PrivacyClass::ContentRefsOnly;
        base.redaction_policy_id = "policy.redaction.hook.default".to_string();
        let journal_record = hook_journal_record(
            base,
            hook_record.clone(),
            SourceRef::with_kind(SourceKind::Hook, spec.hook_id.as_str()),
            "hook_response_decision",
        );
        (hook_record, journal_record)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
/// Enumerates the finite hook record payload cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum HookRecordPayload {
    /// Use this variant when the contract needs to represent registered; selecting it has no side effect by itself.
    Registered {
        /// Deterministic spec hash used for stale checks, package evidence,
        /// or replay comparisons.
        spec_hash: String,
        /// Typed executor ref reference. Resolving or executing it is a
        /// separate policy-gated step.
        executor_ref: String,
        /// Policy reference that must be resolved by the host or runtime
        /// before execution.
        policy_ref: PolicyRef,
    },
    /// Use this variant when the contract needs to represent invocation started; selecting it has no side effect by itself.
    InvocationStarted {
        /// Stable invocation id used for typed lineage, lookup, or dedupe.
        invocation_id: String,
    },
    /// Use this variant when the contract needs to represent completed; selecting it has no side effect by itself.
    Completed {
        /// Stable invocation id used for typed lineage, lookup, or dedupe.
        invocation_id: String,
        /// elapsed ms duration in milliseconds.
        elapsed_ms: u64,
    },
    /// Use this variant when the contract needs to represent timed out; selecting it has no side effect by itself.
    TimedOut {
        /// Stable invocation id used for typed lineage, lookup, or dedupe.
        invocation_id: String,
        /// elapsed ms duration in milliseconds.
        elapsed_ms: u64,
        /// Failure policy used by this record or request.
        failure_policy: String,
    },
    /// Use this variant when the contract needs to represent cancelled; selecting it has no side effect by itself.
    Cancelled {
        /// Stable invocation id used for typed lineage, lookup, or dedupe.
        invocation_id: String,
    },
    /// Use this variant when the contract needs to represent failed; selecting it has no side effect by itself.
    Failed {
        /// Stable invocation id used for typed lineage, lookup, or dedupe.
        invocation_id: String,
        /// Failure policy used by this record or request.
        failure_policy: String,
        /// Redacted human-readable summary safe for events, telemetry, and
        /// logs.
        redacted_summary: String,
    },
    /// Use this variant when the contract needs to represent response decision; selecting it has no side effect by itself.
    ResponseDecision {
        /// Stable invocation id used for typed lineage, lookup, or dedupe.
        invocation_id: String,
        /// Decision used by this record or request.
        decision: HookResponseDecision,
        /// Classification value for response class.
        /// Policy and projection paths use it for finite routing decisions.
        response_class: HookResponseClass,
        /// Decision explaining why a hook response mutation class was accepted or rejected.
        /// Use it as audit evidence when applying hook responses that can affect run state.
        mutation_right_decision: String,
        /// Typed target domain refs references. Resolving them is separate
        /// from constructing this record.
        target_domain_refs: Vec<EntityRef>,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite hook response decision cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum HookResponseDecision {
    /// Use this variant when the contract needs to represent accepted journaled before apply; selecting it has no side effect by itself.
    AcceptedJournaledBeforeApply,
    /// Use this variant when the contract needs to represent rejected mutation right; selecting it has no side effect by itself.
    RejectedMutationRight,
    /// Use this variant when the contract needs to represent rejected point matrix; selecting it has no side effect by itself.
    RejectedPointMatrix,
    /// Use this variant when the contract needs to represent rejected policy; selecting it has no side effect by itself.
    RejectedPolicy,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the hook mutation journal plan record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct HookMutationJournalPlan {
    /// Hook record used by this record or request.
    pub hook_record: HookRecord,
    /// Effect intent used by this record or request.
    pub effect_intent: EffectIntent,
    /// Hook journal record used by this record or request.
    pub hook_journal_record: JournalRecord,
    /// Intent journal record used by this record or request.
    pub intent_journal_record: JournalRecord,
    /// Result journal record used by this record or request.
    pub result_journal_record: JournalRecord,
}

impl HookMutationJournalPlan {
    /// Builds the accepted response value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    #[expect(
        clippy::too_many_arguments,
        reason = "hook response records are durable audit DTOs and keep their lineage fields explicit until a record-builder pass"
    )]
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

/// Builds the hook entity ref value.
/// This is data construction and performs no I/O, journal append, event publication, or process
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
            privacy_class: base.privacy,
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
