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
pub enum EntityKind {
    Run,
    Turn,
    Attempt,
    Agent,
    AgentPool,
    Topic,
    Event,
    Message,
    WakeCondition,
    ContextContribution,
    ContextItem,
    ContextProjection,
    Content,
    Artifact,
    Capability,
    PackageSidecar,
    RuntimePackage,
    PolicyDecision,
    Effect,
    EffectIntent,
    EffectResult,
    ToolCall,
    ApprovalRequest,
    StreamRule,
    RealtimeSession,
    Hook,
    ExecutionEnvironment,
    ChildArtifact,
    SubagentRun,
    ExtensionAction,
    OutputDelivery,
}

#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct EntityId(String);

impl EntityId {
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("EntityId must be valid")
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
pub struct EntityRef {
    pub kind: EntityKind,
    pub id: EntityId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceRef>,
    pub privacy: PrivacyClass,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redacted_summary: Option<String>,
}

impl EntityRef {
    pub fn new(kind: EntityKind, id: impl Into<EntityId>) -> Self {
        Self {
            kind,
            id: id.into(),
            source: None,
            privacy: PrivacyClass::ContentRefsOnly,
            redacted_summary: None,
        }
    }

    pub fn run(id: RunId) -> Self {
        Self::new(EntityKind::Run, id)
    }

    pub fn agent(id: AgentId) -> Self {
        Self::new(EntityKind::Agent, id)
    }

    pub fn agent_pool(id: AgentPoolId) -> Self {
        Self::new(EntityKind::AgentPool, id)
    }

    pub fn topic(id: TopicId) -> Self {
        Self::new(EntityKind::Topic, id)
    }

    pub fn message(id: MessageId) -> Self {
        Self::new(EntityKind::Message, id)
    }

    pub fn wake_condition(id: WakeConditionId) -> Self {
        Self::new(EntityKind::WakeCondition, id)
    }

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
pub enum SourceKind {
    Sdk,
    Host,
    User,
    System,
    Developer,
    Memory,
    Compaction,
    Hook,
    Extension,
    Tool,
    RemoteChannel,
    ScheduledTask,
    Subagent,
    ExternalRuntime,
    Replay,
}

#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct SourceId(String);

impl SourceId {
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("SourceId must be valid")
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
pub struct SourceRef {
    pub kind: SourceKind,
    pub id: SourceId,
    pub trust: TrustClass,
    pub privacy: PrivacyClass,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub correlation: Vec<CorrelationEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redacted_summary: Option<String>,
}

impl SourceRef {
    pub fn new(id: impl Into<String>) -> Self {
        Self::with_kind(SourceKind::Host, id)
    }

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
pub enum DestinationKind {
    Agent,
    AgentPool,
    Topic,
    Provider,
    ContextProjection,
    Journal,
    EventStream,
    Telemetry,
    OutputSink,
    Host,
    User,
    RemoteChannel,
    Tool,
    Extension,
    Subagent,
    ExternalRuntime,
}

#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct DestinationId(String);

impl DestinationId {
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("DestinationId must be valid")
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
pub struct DestinationRef {
    pub kind: DestinationKind,
    pub id: DestinationId,
    pub privacy: PrivacyClass,
    pub retention: RetentionClass,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub correlation: Vec<CorrelationEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redacted_summary: Option<String>,
}

impl DestinationRef {
    pub fn new(id: impl Into<String>) -> Self {
        Self::with_kind(DestinationKind::Host, id)
    }

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
pub enum PolicyKind {
    Approval,
    Permission,
    Sandbox,
    Context,
    Privacy,
    Retention,
    Redaction,
    RuntimePackage,
    Host,
}

#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct PolicyId(String);

impl PolicyId {
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("PolicyId must be valid")
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
pub struct AdapterRef(String);

impl AdapterRef {
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("AdapterRef must be valid")
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
pub struct PolicyRef {
    pub kind: PolicyKind,
    pub id: PolicyId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

impl PolicyRef {
    pub fn new(id: impl Into<String>) -> Self {
        Self::with_kind(PolicyKind::Host, id)
    }

    pub fn with_kind(kind: PolicyKind, id: impl Into<String>) -> Self {
        Self {
            kind,
            id: PolicyId::new(id),
            version: None,
        }
    }

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
pub struct LineageRef {
    pub lineage_id: LineageId,
    pub source: SourceRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<DestinationRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policy_refs: Vec<PolicyRef>,
}
