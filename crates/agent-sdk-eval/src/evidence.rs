//! Evidence bundles derived from core traces.

use serde::{Deserialize, Serialize};

use agent_sdk_core::{
    AgentError, EntityKind, EntityRef, PolicyRef, PrivacyClass, RetentionClass, RunTrace,
    SessionTimeline, TurnTrace,
};

use crate::EvaluationScope;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Role one evidence item plays in an evaluation bundle.
pub enum EvidenceRole {
    /// User input or source message.
    Input,
    /// Context projection or admitted context item.
    Context,
    /// Tool call evidence.
    Tool,
    /// Model attempt evidence.
    Model,
    /// Terminal output or result marker evidence.
    Output,
    /// Effect intent/result evidence.
    Effect,
    /// Policy or runtime constraint evidence.
    Policy,
    /// Expected outcome evidence supplied by a test or reviewer.
    ExpectedOutcome,
    /// Baseline or comparison evidence.
    Baseline,
    /// Other product-neutral evidence.
    Other,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// One model-visible or evaluator-visible evidence ref.
pub struct EvidenceItem {
    /// Entity ref used for cited support validation.
    pub evidence_ref: EntityRef,
    /// Role this item plays in the evaluation.
    pub role: EvidenceRole,
    /// Bounded summary safe for evaluator prompts and logs.
    pub redacted_summary: String,
    /// Privacy class for projection and storage decisions.
    pub privacy_class: PrivacyClass,
    /// Retention class for downstream storage decisions.
    pub retention_class: RetentionClass,
    /// Refs this item was derived from.
    pub derived_from: Vec<EntityRef>,
}

impl EvidenceItem {
    /// Creates an evidence item with content-ref-only privacy defaults.
    pub fn new(
        evidence_ref: EntityRef,
        role: EvidenceRole,
        redacted_summary: impl Into<String>,
    ) -> Self {
        Self {
            evidence_ref,
            role,
            redacted_summary: redacted_summary.into(),
            privacy_class: PrivacyClass::ContentRefsOnly,
            retention_class: RetentionClass::RunScoped,
            derived_from: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Result of validating evaluator-cited support refs against available evidence.
pub struct SupportRefValidation {
    /// Cited refs that matched an available evidence item.
    pub accepted_refs: Vec<EntityRef>,
    /// Cited refs that were not available to the evaluator.
    pub rejected_refs: Vec<EntityRef>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Evidence supplied to an evaluator.
pub struct EvidenceBundle {
    /// Scope this bundle covers.
    pub scope: EvaluationScope,
    /// Evidence refs available for cited support.
    pub items: Vec<EvidenceItem>,
    /// Optional outcome ref being evaluated.
    pub outcome_ref: Option<EntityRef>,
    /// Bounded bundle summary safe for prompts and logs.
    pub redacted_summary: String,
    /// Policy refs that governed this evidence projection.
    pub policy_refs: Vec<PolicyRef>,
    /// Privacy class for the bundle.
    pub privacy_class: PrivacyClass,
    /// Retention class for the bundle.
    pub retention_class: RetentionClass,
}

impl EvidenceBundle {
    /// Creates an empty evidence bundle.
    pub fn new(scope: EvaluationScope, redacted_summary: impl Into<String>) -> Self {
        Self {
            scope,
            items: Vec::new(),
            outcome_ref: None,
            redacted_summary: redacted_summary.into(),
            policy_refs: Vec::new(),
            privacy_class: PrivacyClass::ContentRefsOnly,
            retention_class: RetentionClass::RunScoped,
        }
    }

    /// Builds an evidence bundle from a core turn trace.
    pub fn from_turn_trace(trace: &TurnTrace) -> Result<Self, AgentError> {
        let turn_id = trace.turn_id.clone().ok_or_else(|| {
            AgentError::contract_violation("turn trace is missing turn id for evaluation")
        })?;
        let mut bundle = Self::new(
            EvaluationScope::Turn {
                session_id: trace.session_id.clone(),
                turn_id: turn_id.clone(),
            },
            "turn trace evidence",
        );
        bundle.outcome_ref = trace.run_ids.first().cloned().map(EntityRef::run);
        bundle.push(EvidenceItem::new(
            EntityRef::new(EntityKind::Turn, turn_id),
            EvidenceRole::Input,
            "turn envelope",
        ));
        for run_id in &trace.run_ids {
            bundle.push(EvidenceItem::new(
                EntityRef::run(run_id.clone()),
                EvidenceRole::Output,
                "run associated with turn",
            ));
        }
        for attempt_id in &trace.attempt_ids {
            bundle.push(EvidenceItem::new(
                EntityRef::new(EntityKind::Attempt, attempt_id.clone()),
                EvidenceRole::Model,
                "model attempt",
            ));
        }
        for message_id in &trace.message_ids {
            bundle.push(EvidenceItem::new(
                EntityRef::message(message_id.clone()),
                EvidenceRole::Input,
                "message envelope",
            ));
        }
        for projection_id in &trace.context_projection_ids {
            bundle.push(EvidenceItem::new(
                EntityRef::new(EntityKind::ContextProjection, projection_id.clone()),
                EvidenceRole::Context,
                "context projection",
            ));
        }
        for effect_id in &trace.effect_ids {
            bundle.push(EvidenceItem::new(
                EntityRef::new(EntityKind::Effect, effect_id.clone()),
                EvidenceRole::Effect,
                "effect evidence",
            ));
        }
        for tool_call_id in &trace.tool_call_ids {
            bundle.push(EvidenceItem::new(
                EntityRef::new(EntityKind::ToolCall, tool_call_id.clone()),
                EvidenceRole::Tool,
                "tool call evidence",
            ));
        }
        Ok(bundle)
    }

    /// Builds an evidence bundle from a core run trace.
    pub fn from_run_trace(trace: &RunTrace) -> Result<Self, AgentError> {
        let run_id = trace.run_id.clone().ok_or_else(|| {
            AgentError::contract_violation("run trace is missing run id for evaluation")
        })?;
        let mut bundle = Self::new(
            EvaluationScope::Run {
                run_id: run_id.clone(),
            },
            "run trace evidence",
        );
        bundle.outcome_ref = Some(EntityRef::run(run_id.clone()));
        bundle.push(EvidenceItem::new(
            EntityRef::run(run_id),
            EvidenceRole::Output,
            "run envelope",
        ));
        for turn in &trace.turn_traces {
            let turn_bundle = Self::from_turn_trace(turn)?;
            for item in turn_bundle.items {
                bundle.push(item);
            }
        }
        Ok(bundle)
    }

    /// Builds an evidence bundle from a core session timeline.
    pub fn from_session_timeline(timeline: &SessionTimeline) -> Result<Self, AgentError> {
        let mut bundle = Self::new(
            EvaluationScope::Session {
                session_id: timeline.session_id.clone(),
            },
            "session timeline evidence",
        );
        for turn in &timeline.turns {
            let turn_bundle = Self::from_turn_trace(turn)?;
            if bundle.outcome_ref.is_none() {
                bundle.outcome_ref = turn_bundle.outcome_ref.clone();
            }
            for item in turn_bundle.items {
                bundle.push(item);
            }
        }
        Ok(bundle)
    }

    /// Returns this bundle with an item appended, deduped by entity ref.
    pub fn with_item(mut self, item: EvidenceItem) -> Self {
        self.push(item);
        self
    }

    /// Validates cited support refs against this bundle.
    pub fn validate_support_refs(
        &self,
        support_refs: impl IntoIterator<Item = EntityRef>,
        max_support_refs: usize,
    ) -> SupportRefValidation {
        let mut accepted_refs = Vec::new();
        let mut rejected_refs = Vec::new();
        for cited_ref in support_refs.into_iter().take(max_support_refs) {
            if let Some(available_ref) = self
                .items
                .iter()
                .map(|item| &item.evidence_ref)
                .find(|available_ref| same_entity_ref(available_ref, &cited_ref))
            {
                push_unique(&mut accepted_refs, available_ref.clone());
            } else {
                push_unique(&mut rejected_refs, cited_ref);
            }
        }
        SupportRefValidation {
            accepted_refs,
            rejected_refs,
        }
    }

    fn push(&mut self, item: EvidenceItem) {
        if !self
            .items
            .iter()
            .any(|existing| same_entity_ref(&existing.evidence_ref, &item.evidence_ref))
        {
            self.items.push(item);
        }
    }
}

fn push_unique(items: &mut Vec<EntityRef>, value: EntityRef) {
    if !items
        .iter()
        .any(|existing| same_entity_ref(existing, &value))
    {
        items.push(value);
    }
}

pub(crate) fn same_entity_ref(left: &EntityRef, right: &EntityRef) -> bool {
    left.kind == right.kind && left.id.as_str() == right.id.as_str()
}
