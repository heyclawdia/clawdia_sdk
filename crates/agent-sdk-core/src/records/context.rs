use serde::{Deserialize, Serialize};

use crate::{
    content::ContentRef,
    domain::{
        AgentError, ContentId, ContextItemId, ContextProjectionId, DestinationRef, EntityKind,
        EntityRef, LineageRef, MessageId, PolicyKind, PolicyRef, PrivacyClass, RetentionClass,
        SourceKind, SourceRef, TrustClass,
    },
    error::{AgentErrorKind, RetryClassification},
    ids::{IdValidationError, validate_identifier},
};

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct ContextContributionId(String);

impl ContextContributionId {
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("ContextContributionId must be valid")
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentMessageRole {
    System,
    Developer,
    User,
    Assistant,
    Tool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentMessagePart {
    Text {
        text: String,
    },
    ContentRef {
        content_ref: ContentRef,
    },
    Redacted {
        redacted_summary: String,
        content_ref: Option<ContentRef>,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AgentMessage {
    pub message_id: MessageId,
    pub role: AgentMessageRole,
    pub parts: Vec<AgentMessagePart>,
    pub producer_ref: EntityRef,
    pub source_ref: SourceRef,
    pub destination_ref: Option<DestinationRef>,
    pub policy_refs: Vec<PolicyRef>,
    pub privacy_class: PrivacyClass,
    pub retention_class: RetentionClass,
    pub trust_class: TrustClass,
    pub lineage_refs: Vec<LineageRef>,
    pub redacted_summary: String,
    pub provider_projected: bool,
}

impl AgentMessage {
    pub fn user_text(
        message_id: MessageId,
        text: impl Into<String>,
        source_ref: SourceRef,
        policy_ref: PolicyRef,
    ) -> Self {
        let text = text.into();
        Self {
            message_id: message_id.clone(),
            role: AgentMessageRole::User,
            parts: vec![AgentMessagePart::Text { text }],
            producer_ref: EntityRef::message(message_id),
            source_ref,
            destination_ref: None,
            policy_refs: vec![policy_ref],
            privacy_class: PrivacyClass::ContentRefsOnly,
            retention_class: RetentionClass::RunScoped,
            trust_class: TrustClass::UserProvided,
            lineage_refs: Vec::new(),
            redacted_summary: "user text message".to_string(),
            provider_projected: false,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextContributionKind {
    UserInput,
    HostContext,
    MemoryRecall,
    ToolResult,
    SkillResult,
    FileContext,
    RemoteChannel,
    AgentPoolMessage,
    SubagentHandoff,
    CompactionSummary,
    SystemInstruction,
    OutputSchemaHint,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ContextContribution {
    pub contribution_id: ContextContributionId,
    pub kind: ContextContributionKind,
    pub producer_ref: EntityRef,
    pub source_ref: SourceRef,
    pub content_ref: Option<ContentRef>,
    pub inline_redacted_summary: Option<String>,
    pub derived_from: Vec<EntityRef>,
    pub policy_refs: Vec<PolicyRef>,
    pub privacy_class: PrivacyClass,
    pub retention_class: RetentionClass,
    pub trust_class: TrustClass,
    pub budget_hint: Option<ContextBudgetHint>,
    pub required: bool,
    pub protected: bool,
    pub redacted_summary: String,
}

impl ContextContribution {
    pub fn new(
        contribution_id: ContextContributionId,
        kind: ContextContributionKind,
        producer_ref: EntityRef,
        source_ref: SourceRef,
        policy_ref: PolicyRef,
        redacted_summary: impl Into<String>,
    ) -> Self {
        Self {
            contribution_id,
            kind,
            producer_ref,
            source_ref,
            content_ref: None,
            inline_redacted_summary: None,
            derived_from: Vec::new(),
            policy_refs: vec![policy_ref],
            privacy_class: PrivacyClass::ContentRefsOnly,
            retention_class: RetentionClass::RunScoped,
            trust_class: TrustClass::HostProvided,
            budget_hint: None,
            required: false,
            protected: false,
            redacted_summary: redacted_summary.into(),
        }
    }

    pub fn with_content_ref(mut self, content_ref: ContentRef) -> Self {
        self.content_ref = Some(content_ref);
        self
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    pub fn protected(mut self) -> Self {
        self.protected = true;
        self
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ContextBudgetHint {
    pub token_estimate: Option<u32>,
    pub byte_estimate: Option<u64>,
    pub priority: u16,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionRole {
    System,
    Developer,
    User,
    AssistantContext,
    ToolResult,
    SchemaHint,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextSelectionReason {
    Required,
    Pinned,
    Relevant,
    Recent,
    ToolResultRequired,
    Compacted,
    Redacted,
    OmittedBudget,
    OmittedPolicy,
    OmittedDuplicate,
    OmittedTrust,
    OmittedStale,
    OmittedMissingRef,
    ProtectedOmittedByPolicy,
}

impl ContextSelectionReason {
    pub fn is_included(&self) -> bool {
        matches!(
            self,
            Self::Required
                | Self::Pinned
                | Self::Relevant
                | Self::Recent
                | Self::ToolResultRequired
                | Self::Compacted
                | Self::Redacted
        )
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ContextSelectionDecision {
    pub contribution_id: Option<ContextContributionId>,
    pub context_item_id: Option<ContextItemId>,
    pub reason: ContextSelectionReason,
    pub producer_ref: EntityRef,
    pub source_ref: SourceRef,
    pub content_ref: Option<ContentRef>,
    pub policy_refs: Vec<PolicyRef>,
    pub privacy_class: PrivacyClass,
    pub retention_class: RetentionClass,
    pub trust_class: TrustClass,
    pub required: bool,
    pub protected: bool,
    pub redacted_summary: String,
}

impl ContextSelectionDecision {
    pub fn selected(contribution: &ContextContribution, context_item_id: ContextItemId) -> Self {
        Self::from_contribution(
            contribution,
            Some(context_item_id),
            if contribution.required {
                ContextSelectionReason::Required
            } else {
                ContextSelectionReason::Relevant
            },
        )
    }

    pub fn omitted(contribution: &ContextContribution, reason: ContextSelectionReason) -> Self {
        Self::from_contribution(contribution, None, reason)
    }

    fn from_contribution(
        contribution: &ContextContribution,
        context_item_id: Option<ContextItemId>,
        reason: ContextSelectionReason,
    ) -> Self {
        Self {
            contribution_id: Some(contribution.contribution_id.clone()),
            context_item_id,
            reason,
            producer_ref: contribution.producer_ref.clone(),
            source_ref: contribution.source_ref.clone(),
            content_ref: contribution.content_ref.clone(),
            policy_refs: contribution.policy_refs.clone(),
            privacy_class: contribution.privacy_class,
            retention_class: contribution.retention_class,
            trust_class: contribution.trust_class,
            required: contribution.required,
            protected: contribution.protected,
            redacted_summary: contribution.redacted_summary.clone(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ContextItem {
    pub context_item_id: ContextItemId,
    pub contribution_id: Option<ContextContributionId>,
    pub kind: ContextContributionKind,
    pub producer_ref: EntityRef,
    pub source_ref: SourceRef,
    pub destination_ref: DestinationRef,
    pub content_ref: Option<ContentRef>,
    pub inline_redacted_summary: Option<String>,
    pub derived_from: Vec<EntityRef>,
    pub projection_role: ProjectionRole,
    pub policy_refs: Vec<PolicyRef>,
    pub privacy_class: PrivacyClass,
    pub retention_class: RetentionClass,
    pub trust_class: TrustClass,
    pub budget_hint: Option<ContextBudgetHint>,
    pub selection: ContextSelectionDecision,
    pub lineage_refs: Vec<LineageRef>,
    pub redacted_summary: String,
}

impl ContextItem {
    pub fn admit(
        contribution: ContextContribution,
        context_item_id: ContextItemId,
        destination_ref: DestinationRef,
        projection_role: ProjectionRole,
    ) -> Self {
        let selection = ContextSelectionDecision::selected(&contribution, context_item_id.clone());
        Self {
            context_item_id,
            contribution_id: Some(contribution.contribution_id),
            kind: contribution.kind,
            producer_ref: contribution.producer_ref,
            source_ref: contribution.source_ref,
            destination_ref,
            content_ref: contribution.content_ref,
            inline_redacted_summary: contribution.inline_redacted_summary,
            derived_from: contribution.derived_from,
            projection_role,
            policy_refs: contribution.policy_refs,
            privacy_class: contribution.privacy_class,
            retention_class: contribution.retention_class,
            trust_class: contribution.trust_class,
            budget_hint: contribution.budget_hint,
            selection,
            lineage_refs: Vec::new(),
            redacted_summary: contribution.redacted_summary,
        }
    }

    pub fn provider_part(&self) -> ProjectedContextPart {
        ProjectedContextPart {
            context_item_id: self.context_item_id.clone(),
            role: self.projection_role.clone(),
            content_ref: self.content_ref.clone(),
            text: self.inline_redacted_summary.clone(),
            redacted_summary: self.redacted_summary.clone(),
            raw_content_included: false,
            privacy_class: self.privacy_class,
            retention_class: self.retention_class,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProjectedContextPart {
    pub context_item_id: ContextItemId,
    pub role: ProjectionRole,
    pub content_ref: Option<ContentRef>,
    pub text: Option<String>,
    pub redacted_summary: String,
    pub raw_content_included: bool,
    pub privacy_class: PrivacyClass,
    pub retention_class: RetentionClass,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ContextBudgetSummary {
    pub max_tokens: Option<u32>,
    pub used_tokens: u32,
    pub max_items: Option<u32>,
    pub included_items: u32,
    pub omitted_items: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ContextProjectionAudit {
    pub projection_id: ContextProjectionId,
    pub source_message_ids: Vec<MessageId>,
    pub candidate_count: u32,
    pub included_count: u32,
    pub omitted_count: u32,
    pub compacted_count: u32,
    pub redacted_count: u32,
    pub policy_denied_count: u32,
    pub budget_denied_count: u32,
    pub missing_ref_count: u32,
    pub protected_omitted_count: u32,
    pub decisions: Vec<ContextSelectionDecision>,
    pub budget: ContextBudgetSummary,
    pub policy_refs: Vec<PolicyRef>,
    pub redaction_policy_id: PolicyRef,
    pub runtime_package_fingerprint: String,
}

impl ContextProjectionAudit {
    pub fn from_decisions(
        projection_id: ContextProjectionId,
        source_message_ids: Vec<MessageId>,
        decisions: Vec<ContextSelectionDecision>,
        mut budget: ContextBudgetSummary,
        redaction_policy_id: PolicyRef,
        runtime_package_fingerprint: impl Into<String>,
    ) -> Self {
        let mut audit = Self {
            projection_id,
            source_message_ids,
            candidate_count: decisions.len() as u32,
            included_count: 0,
            omitted_count: 0,
            compacted_count: 0,
            redacted_count: 0,
            policy_denied_count: 0,
            budget_denied_count: 0,
            missing_ref_count: 0,
            protected_omitted_count: 0,
            decisions,
            budget: ContextBudgetSummary::default(),
            policy_refs: Vec::new(),
            redaction_policy_id,
            runtime_package_fingerprint: runtime_package_fingerprint.into(),
        };

        for decision in &audit.decisions {
            if decision.reason.is_included() {
                audit.included_count += 1;
            } else {
                audit.omitted_count += 1;
            }
            match decision.reason {
                ContextSelectionReason::Compacted => audit.compacted_count += 1,
                ContextSelectionReason::Redacted => audit.redacted_count += 1,
                ContextSelectionReason::OmittedPolicy
                | ContextSelectionReason::ProtectedOmittedByPolicy => {
                    audit.policy_denied_count += 1
                }
                ContextSelectionReason::OmittedBudget => audit.budget_denied_count += 1,
                ContextSelectionReason::OmittedMissingRef => audit.missing_ref_count += 1,
                _ => {}
            }
            if decision.protected && !decision.reason.is_included() {
                audit.protected_omitted_count += 1;
            }
            for policy_ref in &decision.policy_refs {
                if !audit
                    .policy_refs
                    .iter()
                    .any(|existing| existing == policy_ref)
                {
                    audit.policy_refs.push(policy_ref.clone());
                }
            }
        }

        budget.included_items = audit.included_count;
        budget.omitted_items = audit.omitted_count;
        audit.budget = budget;
        audit
    }

    pub fn has_blocking_missing_required_ref(&self) -> bool {
        self.decisions.iter().any(|decision| {
            decision.required && decision.reason == ContextSelectionReason::OmittedMissingRef
        })
    }

    pub fn has_protected_omission(&self) -> bool {
        self.protected_omitted_count > 0
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ContextProjection {
    pub projection_id: ContextProjectionId,
    pub source_messages: Vec<AgentMessage>,
    pub items: Vec<ContextItem>,
    pub projected_parts: Vec<ProjectedContextPart>,
    pub audit: ContextProjectionAudit,
    pub provider_destination: DestinationRef,
}

impl Default for ContextProjection {
    fn default() -> Self {
        let projection_id = ContextProjectionId::new("context.projection.default");
        let destination = DestinationRef::with_kind(
            crate::domain::DestinationKind::Provider,
            "destination.provider.default",
        );
        let redaction = PolicyRef::with_kind(PolicyKind::Redaction, "policy.redaction.default");
        Self {
            projection_id: projection_id.clone(),
            source_messages: Vec::new(),
            items: Vec::new(),
            projected_parts: Vec::new(),
            audit: ContextProjectionAudit::from_decisions(
                projection_id,
                Vec::new(),
                Vec::new(),
                ContextBudgetSummary::default(),
                redaction,
                "runtime.package.default",
            ),
            provider_destination: destination,
        }
    }
}

impl ContextProjection {
    pub fn build(
        projection_id: ContextProjectionId,
        source_messages: Vec<AgentMessage>,
        items: Vec<ContextItem>,
        omitted: Vec<ContextSelectionDecision>,
        provider_destination: DestinationRef,
        budget: ContextBudgetSummary,
        redaction_policy_id: PolicyRef,
        runtime_package_fingerprint: impl Into<String>,
    ) -> Result<Self, AgentError> {
        let mut decisions = Vec::with_capacity(items.len() + omitted.len());
        decisions.extend(items.iter().map(|item| item.selection.clone()));
        decisions.extend(omitted);
        let audit = ContextProjectionAudit::from_decisions(
            projection_id.clone(),
            source_messages
                .iter()
                .map(|message| message.message_id.clone())
                .collect(),
            decisions,
            budget,
            redaction_policy_id,
            runtime_package_fingerprint,
        );
        if audit.has_blocking_missing_required_ref() || audit.has_protected_omission() {
            return Err(AgentError::new(
                AgentErrorKind::ProjectionFailure,
                RetryClassification::RepairNeeded,
                "context projection blocked by required missing ref or protected omission",
            ));
        }

        let projected_parts = items.iter().map(ContextItem::provider_part).collect();
        Ok(Self {
            projection_id,
            source_messages,
            items,
            projected_parts,
            audit,
            provider_destination,
        })
    }

    pub fn provider_visible_content_ids(&self) -> Vec<ContentId> {
        self.projected_parts
            .iter()
            .filter_map(|part| {
                part.content_ref
                    .as_ref()
                    .map(|content_ref| content_ref.content_id.clone())
            })
            .collect()
    }
}

pub fn sdk_context_policy_ref() -> PolicyRef {
    PolicyRef::with_kind(PolicyKind::Context, "policy.context.sdk.default")
}

pub fn sdk_source_ref() -> SourceRef {
    SourceRef::with_kind(SourceKind::Sdk, "source.sdk.context")
}

pub fn contribution_entity_ref(contribution_id: &ContextContributionId) -> EntityRef {
    EntityRef::new(EntityKind::ContextContribution, contribution_id.as_str())
}
