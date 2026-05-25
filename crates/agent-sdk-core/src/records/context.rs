//! Durable and observable SDK records. Use these DTOs for events, journals, effects,
//! context, output, and feature evidence. Constructing records is data-only;
//! persistence, publication, and external actions happen through ports or application
//! coordinators. This file contains the context portion of that contract.
//!
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
/// Carries the context contribution id record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct ContextContributionId(String);

impl ContextContributionId {
    /// Creates a new records::context value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    ///
    /// # Panics
    ///
    /// Panics if constructor invariants fail, such as invalid identifier
    /// text or constructor-specific bounds. Use a fallible constructor such as
    /// `try_new` when one is available for untrusted input.
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("ContextContributionId must be valid")
    }

    /// Creates a new records::context value after validation. Returns
    /// an SDK error instead of panicking when the identifier or input
    /// does not satisfy the contract.
    pub fn try_new(value: impl Into<String>) -> Result<Self, IdValidationError> {
        let value = value.into();
        validate_identifier(&value)?;
        Ok(Self(value))
    }

    /// Returns this value as str. The accessor is side-effect free and
    /// keeps ownership with the caller.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite agent message role cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum AgentMessageRole {
    /// Use this variant when the contract needs to represent system; selecting it has no side effect by itself.
    System,
    /// Use this variant when the contract needs to represent developer; selecting it has no side effect by itself.
    Developer,
    /// Use this variant when the contract needs to represent user; selecting it has no side effect by itself.
    User,
    /// Use this variant when the contract needs to represent assistant; selecting it has no side effect by itself.
    Assistant,
    /// Use this variant when the contract needs to represent tool; selecting it has no side effect by itself.
    Tool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite agent message part cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum AgentMessagePart {
    /// Use this variant when the contract needs to represent text; selecting it has no side effect by itself.
    Text {
        /// Text used by this record or request.
        text: String,
    },
    /// Use this variant when the contract needs to represent content ref; selecting it has no side effect by itself.
    ContentRef {
        /// Content reference where payload bytes or structured tool output
        /// are stored.
        content_ref: ContentRef,
    },
    /// Use this variant when the contract needs to represent redacted; selecting it has no side effect by itself.
    Redacted {
        /// Redacted human-readable summary safe for events, telemetry, and
        /// logs.
        redacted_summary: String,
        /// Content reference where payload bytes or structured tool output
        /// are stored.
        content_ref: Option<ContentRef>,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the agent message record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct AgentMessage {
    /// Message identifier for transcript, projection, or provider-response
    /// lineage.
    pub message_id: MessageId,
    /// Role used by this record or request.
    pub role: AgentMessageRole,
    /// Collection of parts values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub parts: Vec<AgentMessagePart>,
    /// Typed producer ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub producer_ref: EntityRef,
    /// Typed source reference that records where this item originated.
    pub source_ref: SourceRef,
    /// Typed destination reference that records where this item is being sent
    /// or projected.
    pub destination_ref: Option<DestinationRef>,
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy_class: PrivacyClass,
    /// Retention class used by hosts and sinks when storing or exporting this
    /// item.
    pub retention_class: RetentionClass,
    /// Trust class used when deciding whether context or capabilities may be
    /// admitted.
    pub trust_class: TrustClass,
    /// Typed lineage refs references. Resolving them is separate from
    /// constructing this record.
    pub lineage_refs: Vec<LineageRef>,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
    /// Whether provider projected is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub provider_projected: bool,
}

impl AgentMessage {
    /// Builds the user text value with the documented defaults.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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
/// Enumerates the finite context contribution kind cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ContextContributionKind {
    /// Use this variant when the contract needs to represent user input; selecting it has no side effect by itself.
    UserInput,
    /// Use this variant when the contract needs to represent host context; selecting it has no side effect by itself.
    HostContext,
    /// Use this variant when the contract needs to represent memory recall; selecting it has no side effect by itself.
    MemoryRecall,
    /// Use this variant when the contract needs to represent tool result; selecting it has no side effect by itself.
    ToolResult,
    /// Use this variant when the contract needs to represent skill result; selecting it has no side effect by itself.
    SkillResult,
    /// Use this variant when the contract needs to represent file context; selecting it has no side effect by itself.
    FileContext,
    /// Use this variant when the contract needs to represent remote channel; selecting it has no side effect by itself.
    RemoteChannel,
    /// Use this variant when the contract needs to represent agent pool message; selecting it has no side effect by itself.
    AgentPoolMessage,
    /// Use this variant when the contract needs to represent subagent handoff; selecting it has no side effect by itself.
    SubagentHandoff,
    /// Use this variant when the contract needs to represent compaction summary; selecting it has no side effect by itself.
    CompactionSummary,
    /// Use this variant when the contract needs to represent system instruction; selecting it has no side effect by itself.
    SystemInstruction,
    /// Use this variant when the contract needs to represent output schema hint; selecting it has no side effect by itself.
    OutputSchemaHint,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the context contribution record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct ContextContribution {
    /// Stable contribution id used for typed lineage, lookup, or dedupe.
    pub contribution_id: ContextContributionId,
    /// Kind/category for this record, capability, event, or detected
    /// resource.
    pub kind: ContextContributionKind,
    /// Typed producer ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub producer_ref: EntityRef,
    /// Typed source reference that records where this item originated.
    pub source_ref: SourceRef,
    /// Content reference where payload bytes or structured tool output are
    /// stored.
    pub content_ref: Option<ContentRef>,
    /// Redacted summary for display, logs, events, or telemetry.
    /// It should describe the value without exposing raw private content.
    pub inline_redacted_summary: Option<String>,
    /// Source refs this value was derived from.
    /// Use them to trace provenance without embedding raw source content.
    pub derived_from: Vec<EntityRef>,
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy_class: PrivacyClass,
    /// Retention class used by hosts and sinks when storing or exporting this
    /// item.
    pub retention_class: RetentionClass,
    /// Trust class used when deciding whether context or capabilities may be
    /// admitted.
    pub trust_class: TrustClass,
    /// Optional budget hint value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub budget_hint: Option<ContextBudgetHint>,
    /// Whether required is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub required: bool,
    /// Whether protected is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub protected: bool,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

impl ContextContribution {
    /// Creates a new records::context value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
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

    /// Returns this value with its content ref setting replaced. The
    /// method follows builder-style data construction and does not
    /// execute external work.
    pub fn with_content_ref(mut self, content_ref: ContentRef) -> Self {
        self.content_ref = Some(content_ref);
        self
    }

    /// Returns an updated value with required configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Returns an updated value with protected configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn protected(mut self) -> Self {
        self.protected = true;
        self
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the context budget hint record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct ContextBudgetHint {
    /// Optional token estimate value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub token_estimate: Option<u32>,
    /// Byte size or byte limit for byte estimate.
    /// Use it to enforce bounded reads, writes, summaries, or parser output.
    pub byte_estimate: Option<u64>,
    /// Priority used by this record or request.
    pub priority: u16,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite projection role cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ProjectionRole {
    /// Use this variant when the contract needs to represent system; selecting it has no side effect by itself.
    System,
    /// Use this variant when the contract needs to represent developer; selecting it has no side effect by itself.
    Developer,
    /// Use this variant when the contract needs to represent user; selecting it has no side effect by itself.
    User,
    /// Use this variant when the contract needs to represent assistant context; selecting it has no side effect by itself.
    AssistantContext,
    /// Use this variant when the contract needs to represent tool result; selecting it has no side effect by itself.
    ToolResult,
    /// Use this variant when the contract needs to represent schema hint; selecting it has no side effect by itself.
    SchemaHint,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite context selection reason cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ContextSelectionReason {
    /// Use this variant when the contract needs to represent required; selecting it has no side effect by itself.
    Required,
    /// Use this variant when the contract needs to represent pinned; selecting it has no side effect by itself.
    Pinned,
    /// Use this variant when the contract needs to represent relevant; selecting it has no side effect by itself.
    Relevant,
    /// Use this variant when the contract needs to represent recent; selecting it has no side effect by itself.
    Recent,
    /// Use this variant when the contract needs to represent tool result required; selecting it has no side effect by itself.
    ToolResultRequired,
    /// Use this variant when the contract needs to represent compacted; selecting it has no side effect by itself.
    Compacted,
    /// Use this variant when the contract needs to represent redacted; selecting it has no side effect by itself.
    Redacted,
    /// Use this variant when the contract needs to represent omitted budget; selecting it has no side effect by itself.
    OmittedBudget,
    /// Use this variant when the contract needs to represent omitted policy; selecting it has no side effect by itself.
    OmittedPolicy,
    /// Use this variant when the contract needs to represent omitted duplicate; selecting it has no side effect by itself.
    OmittedDuplicate,
    /// Use this variant when the contract needs to represent omitted trust; selecting it has no side effect by itself.
    OmittedTrust,
    /// Use this variant when the contract needs to represent omitted stale; selecting it has no side effect by itself.
    OmittedStale,
    /// Use this variant when the contract needs to represent omitted missing ref; selecting it has no side effect by itself.
    OmittedMissingRef,
    /// Use this variant when the contract needs to represent protected omitted by policy; selecting it has no side effect by itself.
    ProtectedOmittedByPolicy,
}

impl ContextSelectionReason {
    /// Reports whether this value is included. The check is pure and
    /// does not mutate SDK or host state.
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
/// Carries the context selection decision record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct ContextSelectionDecision {
    /// Stable contribution id used for typed lineage, lookup, or dedupe.
    pub contribution_id: Option<ContextContributionId>,
    /// Stable context item id used for typed lineage, lookup, or dedupe.
    pub context_item_id: Option<ContextItemId>,
    /// Redacted explanation for a denial, failure, status, or package delta.
    pub reason: ContextSelectionReason,
    /// Typed producer ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub producer_ref: EntityRef,
    /// Typed source reference that records where this item originated.
    pub source_ref: SourceRef,
    /// Content reference where payload bytes or structured tool output are
    /// stored.
    pub content_ref: Option<ContentRef>,
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy_class: PrivacyClass,
    /// Retention class used by hosts and sinks when storing or exporting this
    /// item.
    pub retention_class: RetentionClass,
    /// Trust class used when deciding whether context or capabilities may be
    /// admitted.
    pub trust_class: TrustClass,
    /// Whether required is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub required: bool,
    /// Whether protected is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub protected: bool,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

impl ContextSelectionDecision {
    /// Builds the selected value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
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

    /// Builds the omitted value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
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
/// Carries the context item record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct ContextItem {
    /// Stable context item id used for typed lineage, lookup, or dedupe.
    pub context_item_id: ContextItemId,
    /// Stable contribution id used for typed lineage, lookup, or dedupe.
    pub contribution_id: Option<ContextContributionId>,
    /// Kind/category for this record, capability, event, or detected
    /// resource.
    pub kind: ContextContributionKind,
    /// Typed producer ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub producer_ref: EntityRef,
    /// Typed source reference that records where this item originated.
    pub source_ref: SourceRef,
    /// Typed destination reference that records where this item is being sent
    /// or projected.
    pub destination_ref: DestinationRef,
    /// Content reference where payload bytes or structured tool output are
    /// stored.
    pub content_ref: Option<ContentRef>,
    /// Redacted summary for display, logs, events, or telemetry.
    /// It should describe the value without exposing raw private content.
    pub inline_redacted_summary: Option<String>,
    /// Source refs this value was derived from.
    /// Use them to trace provenance without embedding raw source content.
    pub derived_from: Vec<EntityRef>,
    /// Projection controls for exposing data to a provider or subscriber.
    /// Use it to keep provider-visible data separate from private SDK state.
    pub projection_role: ProjectionRole,
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy_class: PrivacyClass,
    /// Retention class used by hosts and sinks when storing or exporting this
    /// item.
    pub retention_class: RetentionClass,
    /// Trust class used when deciding whether context or capabilities may be
    /// admitted.
    pub trust_class: TrustClass,
    /// Optional budget hint value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub budget_hint: Option<ContextBudgetHint>,
    /// Selection used by this record or request.
    pub selection: ContextSelectionDecision,
    /// Typed lineage refs references. Resolving them is separate from
    /// constructing this record.
    pub lineage_refs: Vec<LineageRef>,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

impl ContextItem {
    /// Builds the admit value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
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

    /// Builds the provider part value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
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
/// Carries the projected context part record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct ProjectedContextPart {
    /// Stable context item id used for typed lineage, lookup, or dedupe.
    pub context_item_id: ContextItemId,
    /// Role used by this record or request.
    pub role: ProjectionRole,
    /// Content reference where payload bytes or structured tool output are
    /// stored.
    pub content_ref: Option<ContentRef>,
    /// Optional text value.
    /// When absent, callers should use the documented default or skip that optional behavior.
    pub text: Option<String>,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
    /// Raw content or raw-content control for this value.
    /// Use it only when policy explicitly allows raw content capture or delivery.
    pub raw_content_included: bool,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy_class: PrivacyClass,
    /// Retention class used by hosts and sinks when storing or exporting this
    /// item.
    pub retention_class: RetentionClass,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the context budget summary record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct ContextBudgetSummary {
    /// Maximum allowed tokens.
    /// Use it to keep execution, output, or diagnostics bounded.
    pub max_tokens: Option<u32>,
    /// Used tokens used by this record or request.
    pub used_tokens: u32,
    /// Maximum allowed items.
    /// Use it to keep execution, output, or diagnostics bounded.
    pub max_items: Option<u32>,
    /// Included items used by this record or request.
    pub included_items: u32,
    /// Omitted items used by this record or request.
    pub omitted_items: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the context projection audit record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct ContextProjectionAudit {
    /// Stable projection id used for typed lineage, lookup, or dedupe.
    pub projection_id: ContextProjectionId,
    /// Identifiers used to select or correlate source message values.
    /// Use them for typed lookup, filtering, or lineage instead of stringly typed matching.
    pub source_message_ids: Vec<MessageId>,
    /// Count of candidate items observed or included in this record.
    pub candidate_count: u32,
    /// Count of included items observed or included in this record.
    pub included_count: u32,
    /// Count of omitted items observed or included in this record.
    pub omitted_count: u32,
    /// Count of compacted items observed or included in this record.
    pub compacted_count: u32,
    /// Count of redacted items observed or included in this record.
    pub redacted_count: u32,
    /// Count of policy denied items observed or included in this record.
    pub policy_denied_count: u32,
    /// Count of budget denied items observed or included in this record.
    pub budget_denied_count: u32,
    /// Count of missing ref items observed or included in this record.
    pub missing_ref_count: u32,
    /// Count of protected omitted items observed or included in this record.
    pub protected_omitted_count: u32,
    /// Collection of decisions values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub decisions: Vec<ContextSelectionDecision>,
    /// Budget used by this record or request.
    pub budget: ContextBudgetSummary,
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
    /// Stable redaction policy id used for typed lineage, lookup, or dedupe.
    pub redaction_policy_id: PolicyRef,
    /// Fingerprint of the runtime package snapshot in force when this value was produced.
    /// Use it for replay, dedupe, and package-lineage checks; the field is evidence and does
    /// not execute package behavior.
    pub runtime_package_fingerprint: String,
}

impl ContextProjectionAudit {
    /// Constructs this value from decisions. Use it when adapting
    /// canonical SDK records without introducing a second behavior
    /// path.
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

    /// Returns has blocking missing required ref for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn has_blocking_missing_required_ref(&self) -> bool {
        self.decisions.iter().any(|decision| {
            decision.required && decision.reason == ContextSelectionReason::OmittedMissingRef
        })
    }

    /// Returns has protected omission for this records::context value without
    /// performing external I/O.
    pub fn has_protected_omission(&self) -> bool {
        self.protected_omitted_count > 0
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries the context projection record payload for journal, event, or fixture surfaces.
/// Creating or cloning it only preserves serialized SDK state; append, publish, replay, or export effects are documented on the runtime and port methods that store it.
pub struct ContextProjection {
    /// Stable projection id used for typed lineage, lookup, or dedupe.
    pub projection_id: ContextProjectionId,
    /// Collection of source messages values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub source_messages: Vec<AgentMessage>,
    /// Bounded items included in this record. Limits and truncation are
    /// represented by companion metadata when applicable.
    pub items: Vec<ContextItem>,
    /// Collection of projected parts values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub projected_parts: Vec<ProjectedContextPart>,
    /// Audit used by this record or request.
    pub audit: ContextProjectionAudit,
    /// Provider destination used by this record or request.
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
    /// Finishes builder validation and returns the configured value.
    /// This is data-only unless the surrounding builder explicitly
    /// documents adapter or store access.
    #[expect(
        clippy::too_many_arguments,
        reason = "ContextProjection::build intentionally captures all projection inputs; a projection builder should be designed as a separate API pass"
    )]
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

    /// Computes or returns provider visible content ids for the
    /// records::context contract without external I/O or side effects.
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

/// Returns sdk context policy ref derived from the supplied state.
/// This is data-only and does not perform I/O, call host ports, append journals, publish
/// events, or start processes.
pub fn sdk_context_policy_ref() -> PolicyRef {
    PolicyRef::with_kind(PolicyKind::Context, "policy.context.sdk.default")
}

/// Returns sdk source ref derived from the supplied state.
/// This is data-only and does not perform I/O, call host ports, append journals, publish
/// events, or start processes.
pub fn sdk_source_ref() -> SourceRef {
    SourceRef::with_kind(SourceKind::Sdk, "source.sdk.context")
}

/// Returns contribution entity ref derived from the supplied state.
/// This is data-only and does not perform I/O, call host ports, append journals, publish
/// events, or start processes.
pub fn contribution_entity_ref(contribution_id: &ContextContributionId) -> EntityRef {
    EntityRef::new(EntityKind::ContextContribution, contribution_id.as_str())
}
