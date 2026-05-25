//! Domain primitives for stable SDK vocabulary. Use these items for IDs, refs,
//! policy, privacy, trust, and errors that cross crate or host boundaries. They are
//! data-only and must not perform provider, filesystem, network, or UI side effects.
//! This file contains the refs portion of that contract.
//!
use core::fmt;
use serde::{Deserialize, Deserializer, Serialize, de::Error as DeError};

use crate::ids::{
    AgentId, AgentPoolId, ApprovalRequestId, ArtifactId, AttemptId, ContextItemId,
    ContextProjectionId, CorrelationEntry, EffectId, EventId, IdValidationError, LineageId,
    MessageId, RunId, RuntimePackageId, ToolCallId, TopicId, TurnId, WakeConditionId,
    validate_identifier,
};
use crate::privacy::{PrivacyClass, RetentionClass, TrustClass};

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite entity kind cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum EntityKind {
    /// Use this variant when the contract needs to represent run; selecting it has no side effect by itself.
    Run,
    /// Use this variant when the contract needs to represent turn; selecting it has no side effect by itself.
    Turn,
    /// Use this variant when the contract needs to represent attempt; selecting it has no side effect by itself.
    Attempt,
    /// Use this variant when the contract needs to represent agent; selecting it has no side effect by itself.
    Agent,
    /// Use this variant when the contract needs to represent agent pool; selecting it has no side effect by itself.
    AgentPool,
    /// Use this variant when the contract needs to represent topic; selecting it has no side effect by itself.
    Topic,
    /// Use this variant when the contract needs to represent event; selecting it has no side effect by itself.
    Event,
    /// Use this variant when the contract needs to represent message; selecting it has no side effect by itself.
    Message,
    /// Use this variant when the contract needs to represent wake condition; selecting it has no side effect by itself.
    WakeCondition,
    /// Use this variant when the contract needs to represent context contribution; selecting it has no side effect by itself.
    ContextContribution,
    /// Use this variant when the contract needs to represent context item; selecting it has no side effect by itself.
    ContextItem,
    /// Use this variant when the contract needs to represent context projection; selecting it has no side effect by itself.
    ContextProjection,
    /// Use this variant when the contract needs to represent content; selecting it has no side effect by itself.
    Content,
    /// Use this variant when the contract needs to represent artifact; selecting it has no side effect by itself.
    Artifact,
    /// Use this variant when the contract needs to represent capability; selecting it has no side effect by itself.
    Capability,
    /// Use this variant when the contract needs to represent package sidecar; selecting it has no side effect by itself.
    PackageSidecar,
    /// Use this variant when the contract needs to represent runtime package; selecting it has no side effect by itself.
    RuntimePackage,
    /// Use this variant when the contract needs to represent policy decision; selecting it has no side effect by itself.
    PolicyDecision,
    /// Use this variant when the contract needs to represent effect; selecting it has no side effect by itself.
    Effect,
    /// Use this variant when the contract needs to represent effect intent; selecting it has no side effect by itself.
    EffectIntent,
    /// Use this variant when the contract needs to represent effect result; selecting it has no side effect by itself.
    EffectResult,
    /// Use this variant when the contract needs to represent tool call; selecting it has no side effect by itself.
    ToolCall,
    /// Use this variant when the contract needs to represent approval request; selecting it has no side effect by itself.
    ApprovalRequest,
    /// Use this variant when the contract needs to represent stream rule; selecting it has no side effect by itself.
    StreamRule,
    /// Use this variant when the contract needs to represent realtime session; selecting it has no side effect by itself.
    RealtimeSession,
    /// Use this variant when the contract needs to represent hook; selecting it has no side effect by itself.
    Hook,
    /// Use this variant when the contract needs to represent execution environment; selecting it has no side effect by itself.
    ExecutionEnvironment,
    /// Use this variant when the contract needs to represent child artifact; selecting it has no side effect by itself.
    ChildArtifact,
    /// Use this variant when the contract needs to represent subagent run; selecting it has no side effect by itself.
    SubagentRun,
    /// Use this variant when the contract needs to represent extension action; selecting it has no side effect by itself.
    ExtensionAction,
    /// Use this variant when the contract needs to represent output delivery; selecting it has no side effect by itself.
    OutputDelivery,
}

#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
/// Defines the entity id SDK value.
/// Construction records local state only; documented runtimes, executors, or ports own side effects.
pub struct EntityId(String);

impl EntityId {
    /// Creates a new domain::refs value with explicit caller-provided
    /// inputs. This constructor is data-only and performs no I/O or
    /// external side effects.
    ///
    /// # Panics
    ///
    /// Panics if constructor invariants fail, such as invalid identifier
    /// text or constructor-specific bounds. Use a fallible constructor such as
    /// `try_new` when one is available for untrusted input.
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("EntityId must be valid")
    }

    /// Creates a new domain::refs value after validation. Returns an
    /// SDK error instead of panicking when the identifier or input does
    /// not satisfy the contract.
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

impl fmt::Debug for EntityId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("EntityId(redacted)")
    }
}

impl fmt::Display for EntityId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("EntityId(redacted)")
    }
}

impl From<&str> for EntityId {
    fn from(value: &str) -> Self {
        Self::try_new(value).expect("EntityId must be valid")
    }
}

impl<'de> Deserialize<'de> for EntityId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::try_new(value).map_err(D::Error::custom)
    }
}

impl From<RunId> for EntityId {
    fn from(value: RunId) -> Self {
        Self::new(value.as_str())
    }
}

impl From<TurnId> for EntityId {
    fn from(value: TurnId) -> Self {
        Self::new(value.as_str())
    }
}

impl From<AttemptId> for EntityId {
    fn from(value: AttemptId) -> Self {
        Self::new(value.as_str())
    }
}

impl From<AgentId> for EntityId {
    fn from(value: AgentId) -> Self {
        Self::new(value.as_str())
    }
}

impl From<AgentPoolId> for EntityId {
    fn from(value: AgentPoolId) -> Self {
        Self::new(value.as_str())
    }
}

impl From<TopicId> for EntityId {
    fn from(value: TopicId) -> Self {
        Self::new(value.as_str())
    }
}

impl From<EventId> for EntityId {
    fn from(value: EventId) -> Self {
        Self::new(value.as_str())
    }
}

impl From<MessageId> for EntityId {
    fn from(value: MessageId) -> Self {
        Self::new(value.as_str())
    }
}

impl From<WakeConditionId> for EntityId {
    fn from(value: WakeConditionId) -> Self {
        Self::new(value.as_str())
    }
}

impl From<ContextItemId> for EntityId {
    fn from(value: ContextItemId) -> Self {
        Self::new(value.as_str())
    }
}

impl From<ContextProjectionId> for EntityId {
    fn from(value: ContextProjectionId) -> Self {
        Self::new(value.as_str())
    }
}

impl From<ArtifactId> for EntityId {
    fn from(value: ArtifactId) -> Self {
        Self::new(value.as_str())
    }
}

impl From<RuntimePackageId> for EntityId {
    fn from(value: RuntimePackageId) -> Self {
        Self::new(value.as_str())
    }
}

impl From<EffectId> for EntityId {
    fn from(value: EffectId) -> Self {
        Self::new(value.as_str())
    }
}

impl From<ToolCallId> for EntityId {
    fn from(value: ToolCallId) -> Self {
        Self::new(value.as_str())
    }
}

impl From<ApprovalRequestId> for EntityId {
    fn from(value: ApprovalRequestId) -> Self {
        Self::new(value.as_str())
    }
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
/// Defines the entity ref SDK value.
/// Construction records local state only; documented runtimes, executors, or ports own side effects.
pub struct EntityRef {
    /// Kind/category for this record, capability, event, or detected
    /// resource.
    pub kind: EntityKind,
    /// Stable identifier for this record.
    pub id: EntityId,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Source label or ref for this item; it is metadata and does not fetch
    /// content by itself.
    pub source: Option<SourceRef>,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy: PrivacyClass,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: Option<String>,
}

impl EntityRef {
    /// Creates a new domain::refs value with explicit caller-provided
    /// inputs. This constructor is data-only and performs no I/O or
    /// external side effects.
    pub fn new(kind: EntityKind, id: impl Into<EntityId>) -> Self {
        Self {
            kind,
            id: id.into(),
            source: None,
            privacy: PrivacyClass::ContentRefsOnly,
            redacted_summary: None,
        }
    }

    /// Builds the run value with the documented defaults.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn run(id: RunId) -> Self {
        Self::new(EntityKind::Run, id)
    }

    /// Returns agent for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn agent(id: AgentId) -> Self {
        Self::new(EntityKind::Agent, id)
    }

    /// Builds the agent pool value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn agent_pool(id: AgentPoolId) -> Self {
        Self::new(EntityKind::AgentPool, id)
    }

    /// Returns an updated value with topic configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn topic(id: TopicId) -> Self {
        Self::new(EntityKind::Topic, id)
    }

    /// Builds the message value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn message(id: MessageId) -> Self {
        Self::new(EntityKind::Message, id)
    }

    /// Builds the wake condition value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn wake_condition(id: WakeConditionId) -> Self {
        Self::new(EntityKind::WakeCondition, id)
    }

    /// Returns this value as str. The accessor is side-effect free and
    /// keeps ownership with the caller.
    pub fn as_str(&self) -> &str {
        self.id.as_str()
    }
}

impl fmt::Debug for EntityRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EntityRef")
            .field("kind", &self.kind)
            .field("id", &"redacted")
            .field("source", &self.source)
            .field("privacy", &self.privacy)
            .field("redacted_summary", &self.redacted_summary)
            .finish()
    }
}

impl fmt::Display for EntityRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}:redacted", self.kind)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite source kind cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum SourceKind {
    /// Use this variant when the contract needs to represent sdk; selecting it has no side effect by itself.
    Sdk,
    /// Use this variant when the contract needs to represent host; selecting it has no side effect by itself.
    Host,
    /// Use this variant when the contract needs to represent user; selecting it has no side effect by itself.
    User,
    /// Use this variant when the contract needs to represent system; selecting it has no side effect by itself.
    System,
    /// Use this variant when the contract needs to represent developer; selecting it has no side effect by itself.
    Developer,
    /// Use this variant when the contract needs to represent memory; selecting it has no side effect by itself.
    Memory,
    /// Use this variant when the contract needs to represent compaction; selecting it has no side effect by itself.
    Compaction,
    /// Use this variant when the contract needs to represent hook; selecting it has no side effect by itself.
    Hook,
    /// Use this variant when the contract needs to represent extension; selecting it has no side effect by itself.
    Extension,
    /// Use this variant when the contract needs to represent tool; selecting it has no side effect by itself.
    Tool,
    /// Use this variant when the contract needs to represent remote channel; selecting it has no side effect by itself.
    RemoteChannel,
    /// Use this variant when the contract needs to represent scheduled task; selecting it has no side effect by itself.
    ScheduledTask,
    /// Use this variant when the contract needs to represent subagent; selecting it has no side effect by itself.
    Subagent,
    /// Use this variant when the contract needs to represent external runtime; selecting it has no side effect by itself.
    ExternalRuntime,
    /// Use this variant when the contract needs to represent replay; selecting it has no side effect by itself.
    Replay,
}

#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
/// Defines the source id SDK value.
/// Construction records local state only; documented runtimes, executors, or ports own side effects.
pub struct SourceId(String);

impl SourceId {
    /// Creates a new domain::refs value with explicit caller-provided
    /// inputs. This constructor is data-only and performs no I/O or
    /// external side effects.
    ///
    /// # Panics
    ///
    /// Panics if constructor invariants fail, such as invalid identifier
    /// text or constructor-specific bounds. Use a fallible constructor such as
    /// `try_new` when one is available for untrusted input.
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("SourceId must be valid")
    }

    /// Creates a new domain::refs value after validation. Returns an
    /// SDK error instead of panicking when the identifier or input does
    /// not satisfy the contract.
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

impl<'de> Deserialize<'de> for SourceId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::try_new(value).map_err(D::Error::custom)
    }
}

impl fmt::Debug for SourceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SourceId(redacted)")
    }
}

impl fmt::Display for SourceId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SourceId(redacted)")
    }
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
/// Defines the source ref SDK value.
/// Construction records local state only; documented runtimes, executors, or ports own side effects.
pub struct SourceRef {
    /// Kind/category for this record, capability, event, or detected
    /// resource.
    pub kind: SourceKind,
    /// Stable identifier for this record.
    pub id: SourceId,
    /// Trust class used when deciding whether context or capabilities may be
    /// admitted.
    pub trust: TrustClass,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy: PrivacyClass,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Collection of correlation values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub correlation: Vec<CorrelationEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: Option<String>,
}

impl SourceRef {
    /// Creates a new domain::refs value with explicit caller-provided
    /// inputs. This constructor is data-only and performs no I/O or
    /// external side effects.
    pub fn new(id: impl Into<String>) -> Self {
        Self::with_kind(SourceKind::Host, id)
    }

    /// Returns this value with its kind setting replaced. The method
    /// follows builder-style data construction and does not execute
    /// external work.
    pub fn with_kind(kind: SourceKind, id: impl Into<String>) -> Self {
        Self {
            kind,
            id: SourceId::new(id),
            trust: TrustClass::HostProvided,
            privacy: PrivacyClass::ContentRefsOnly,
            correlation: Vec::new(),
            redacted_summary: None,
        }
    }

    /// Returns this value as str. The accessor is side-effect free and
    /// keeps ownership with the caller.
    pub fn as_str(&self) -> &str {
        self.id.as_str()
    }
}

impl fmt::Debug for SourceRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SourceRef")
            .field("kind", &self.kind)
            .field("id", &"redacted")
            .field("trust", &self.trust)
            .field("privacy", &self.privacy)
            .field("correlation", &self.correlation)
            .field("redacted_summary", &self.redacted_summary)
            .finish()
    }
}

impl fmt::Display for SourceRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}:redacted", self.kind)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite destination kind cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum DestinationKind {
    /// Use this variant when the contract needs to represent agent; selecting it has no side effect by itself.
    Agent,
    /// Use this variant when the contract needs to represent agent pool; selecting it has no side effect by itself.
    AgentPool,
    /// Use this variant when the contract needs to represent topic; selecting it has no side effect by itself.
    Topic,
    /// Use this variant when the contract needs to represent provider; selecting it has no side effect by itself.
    Provider,
    /// Use this variant when the contract needs to represent context projection; selecting it has no side effect by itself.
    ContextProjection,
    /// Use this variant when the contract needs to represent journal; selecting it has no side effect by itself.
    Journal,
    /// Use this variant when the contract needs to represent event stream; selecting it has no side effect by itself.
    EventStream,
    /// Use this variant when the contract needs to represent telemetry; selecting it has no side effect by itself.
    Telemetry,
    /// Use this variant when the contract needs to represent output sink; selecting it has no side effect by itself.
    OutputSink,
    /// Use this variant when the contract needs to represent host; selecting it has no side effect by itself.
    Host,
    /// Use this variant when the contract needs to represent user; selecting it has no side effect by itself.
    User,
    /// Use this variant when the contract needs to represent remote channel; selecting it has no side effect by itself.
    RemoteChannel,
    /// Use this variant when the contract needs to represent tool; selecting it has no side effect by itself.
    Tool,
    /// Use this variant when the contract needs to represent extension; selecting it has no side effect by itself.
    Extension,
    /// Use this variant when the contract needs to represent subagent; selecting it has no side effect by itself.
    Subagent,
    /// Use this variant when the contract needs to represent external runtime; selecting it has no side effect by itself.
    ExternalRuntime,
}

#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
/// Defines the destination id SDK value.
/// Construction records local state only; documented runtimes, executors, or ports own side effects.
pub struct DestinationId(String);

impl DestinationId {
    /// Creates a new domain::refs value with explicit caller-provided
    /// inputs. This constructor is data-only and performs no I/O or
    /// external side effects.
    ///
    /// # Panics
    ///
    /// Panics if constructor invariants fail, such as invalid identifier
    /// text or constructor-specific bounds. Use a fallible constructor such as
    /// `try_new` when one is available for untrusted input.
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("DestinationId must be valid")
    }

    /// Creates a new domain::refs value after validation. Returns an
    /// SDK error instead of panicking when the identifier or input does
    /// not satisfy the contract.
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

impl<'de> Deserialize<'de> for DestinationId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::try_new(value).map_err(D::Error::custom)
    }
}

impl fmt::Debug for DestinationId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("DestinationId(redacted)")
    }
}

impl fmt::Display for DestinationId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("DestinationId(redacted)")
    }
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
/// Defines the destination ref SDK value.
/// Construction records local state only; documented runtimes, executors, or ports own side effects.
pub struct DestinationRef {
    /// Kind/category for this record, capability, event, or detected
    /// resource.
    pub kind: DestinationKind,
    /// Stable identifier for this record.
    pub id: DestinationId,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy: PrivacyClass,
    /// Retention class used by hosts and sinks when storing or exporting this
    /// item.
    pub retention: RetentionClass,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Collection of correlation values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub correlation: Vec<CorrelationEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: Option<String>,
}

impl DestinationRef {
    /// Creates a new domain::refs value with explicit caller-provided
    /// inputs. This constructor is data-only and performs no I/O or
    /// external side effects.
    pub fn new(id: impl Into<String>) -> Self {
        Self::with_kind(DestinationKind::Host, id)
    }

    /// Returns this value with its kind setting replaced. The method
    /// follows builder-style data construction and does not execute
    /// external work.
    pub fn with_kind(kind: DestinationKind, id: impl Into<String>) -> Self {
        Self {
            kind,
            id: DestinationId::new(id),
            privacy: PrivacyClass::ContentRefsOnly,
            retention: RetentionClass::HostPolicy,
            correlation: Vec::new(),
            redacted_summary: None,
        }
    }

    /// Returns this value as str. The accessor is side-effect free and
    /// keeps ownership with the caller.
    pub fn as_str(&self) -> &str {
        self.id.as_str()
    }
}

impl fmt::Debug for DestinationRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DestinationRef")
            .field("kind", &self.kind)
            .field("id", &"redacted")
            .field("privacy", &self.privacy)
            .field("retention", &self.retention)
            .field("correlation", &self.correlation)
            .field("redacted_summary", &self.redacted_summary)
            .finish()
    }
}

impl fmt::Display for DestinationRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}:redacted", self.kind)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite policy kind cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum PolicyKind {
    /// Use this variant when the contract needs to represent approval; selecting it has no side effect by itself.
    Approval,
    /// Use this variant when the contract needs to represent permission; selecting it has no side effect by itself.
    Permission,
    /// Use this variant when the contract needs to represent sandbox; selecting it has no side effect by itself.
    Sandbox,
    /// Use this variant when the contract needs to represent context; selecting it has no side effect by itself.
    Context,
    /// Use this variant when the contract needs to represent privacy; selecting it has no side effect by itself.
    Privacy,
    /// Use this variant when the contract needs to represent retention; selecting it has no side effect by itself.
    Retention,
    /// Use this variant when the contract needs to represent redaction; selecting it has no side effect by itself.
    Redaction,
    /// Use this variant when the contract needs to represent runtime package; selecting it has no side effect by itself.
    RuntimePackage,
    /// Use this variant when the contract needs to represent host; selecting it has no side effect by itself.
    Host,
}

#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
/// Defines the policy id SDK value.
/// Construction records local state only; documented runtimes, executors, or ports own side effects.
pub struct PolicyId(String);

impl PolicyId {
    /// Creates a new domain::refs value with explicit caller-provided
    /// inputs. This constructor is data-only and performs no I/O or
    /// external side effects.
    ///
    /// # Panics
    ///
    /// Panics if constructor invariants fail, such as invalid identifier
    /// text or constructor-specific bounds. Use a fallible constructor such as
    /// `try_new` when one is available for untrusted input.
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("PolicyId must be valid")
    }

    /// Creates a new domain::refs value after validation. Returns an
    /// SDK error instead of panicking when the identifier or input does
    /// not satisfy the contract.
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

impl<'de> Deserialize<'de> for PolicyId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::try_new(value).map_err(D::Error::custom)
    }
}

#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
/// Defines the adapter ref SDK value.
/// Construction records local state only; documented runtimes, executors, or ports own side effects.
pub struct AdapterRef(String);

impl AdapterRef {
    /// Creates a new domain::refs value with explicit caller-provided
    /// inputs. This constructor is data-only and performs no I/O or
    /// external side effects.
    ///
    /// # Panics
    ///
    /// Panics if constructor invariants fail, such as invalid identifier
    /// text or constructor-specific bounds. Use a fallible constructor such as
    /// `try_new` when one is available for untrusted input.
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("AdapterRef must be valid")
    }

    /// Creates a new domain::refs value after validation. Returns an
    /// SDK error instead of panicking when the identifier or input does
    /// not satisfy the contract.
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

impl fmt::Debug for AdapterRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("AdapterRef(redacted)")
    }
}

impl fmt::Display for AdapterRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("AdapterRef(redacted)")
    }
}

impl From<&str> for AdapterRef {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl<'de> Deserialize<'de> for AdapterRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::try_new(value).map_err(D::Error::custom)
    }
}

impl fmt::Debug for PolicyId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("PolicyId(redacted)")
    }
}

impl fmt::Display for PolicyId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("PolicyId(redacted)")
    }
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
/// Defines the policy ref SDK value.
/// Construction records local state only; documented runtimes, executors, or ports own side effects.
pub struct PolicyRef {
    /// Kind/category for this record, capability, event, or detected
    /// resource.
    pub kind: PolicyKind,
    /// Stable identifier for this record.
    pub id: PolicyId,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Version string for this capability, package, or protocol surface.
    /// Use it for compatibility checks during package or adapter resolution.
    pub version: Option<String>,
}

impl PolicyRef {
    /// Creates a new domain::refs value with explicit caller-provided
    /// inputs. This constructor is data-only and performs no I/O or
    /// external side effects.
    pub fn new(id: impl Into<String>) -> Self {
        Self::with_kind(PolicyKind::Host, id)
    }

    /// Returns this value with its kind setting replaced. The method
    /// follows builder-style data construction and does not execute
    /// external work.
    pub fn with_kind(kind: PolicyKind, id: impl Into<String>) -> Self {
        Self {
            kind,
            id: PolicyId::new(id),
            version: None,
        }
    }

    /// Returns this value as str. The accessor is side-effect free and
    /// keeps ownership with the caller.
    pub fn as_str(&self) -> &str {
        self.id.as_str()
    }
}

impl fmt::Debug for PolicyRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PolicyRef")
            .field("kind", &self.kind)
            .field("id", &"redacted")
            .field("version", &self.version)
            .finish()
    }
}

impl fmt::Display for PolicyRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}:redacted", self.kind)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Defines the lineage ref SDK value.
/// Construction records local state only; documented runtimes, executors, or ports own side effects.
pub struct LineageRef {
    /// Stable lineage id used for typed lineage, lookup, or dedupe.
    pub lineage_id: LineageId,
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
}
