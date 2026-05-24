use core::fmt;

use serde::{Deserialize, Deserializer, Serialize, de::Error as DeError};

use crate::{
    domain::{
        AdapterRef, AgentError, IdValidationError, PolicyKind, PolicyRef, PrivacyClass, SourceRef,
    },
    ids::validate_identifier,
};

#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct CapabilityId(String);

impl CapabilityId {
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("CapabilityId must be valid")
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

impl From<&str> for CapabilityId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl<'de> Deserialize<'de> for CapabilityId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::try_new(value).map_err(D::Error::custom)
    }
}

impl fmt::Debug for CapabilityId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CapabilityId(redacted)")
    }
}

impl fmt::Display for CapabilityId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CapabilityId(redacted)")
    }
}

#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct CapabilityNamespace(String);

impl CapabilityNamespace {
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("CapabilityNamespace must be valid")
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

impl<'de> Deserialize<'de> for CapabilityNamespace {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::try_new(value).map_err(D::Error::custom)
    }
}

impl fmt::Debug for CapabilityNamespace {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CapabilityNamespace(redacted)")
    }
}

#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct CapabilityVersion(String);

impl CapabilityVersion {
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("CapabilityVersion must be valid")
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

impl<'de> Deserialize<'de> for CapabilityVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::try_new(value).map_err(D::Error::custom)
    }
}

impl fmt::Debug for CapabilityVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CapabilityVersion(redacted)")
    }
}

#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct ExecutorRef(String);

impl ExecutorRef {
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("ExecutorRef must be valid")
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

impl<'de> Deserialize<'de> for ExecutorRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::try_new(value).map_err(D::Error::custom)
    }
}

impl fmt::Debug for ExecutorRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ExecutorRef(redacted)")
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityKind {
    Tool,
    McpTool,
    McpResource,
    ToolDiscoveryCandidate,
    AgentAsTool,
    ExtensionAction,
    StreamControl,
    RealtimeAction,
}

impl CapabilityKind {
    pub fn is_reserved(&self) -> bool {
        !matches!(self, Self::Tool)
    }

    pub fn owner_role(&self) -> &'static str {
        match self {
            Self::Tool => "02-runtime-package-p0-fake-tool",
            Self::McpTool | Self::McpResource | Self::ToolDiscoveryCandidate => {
                "04-tools-approval-toolpacks"
            }
            Self::AgentAsTool => "07-subagents-coordination",
            Self::ExtensionAction => "08-extension-sdk-packaging",
            Self::StreamControl | Self::RealtimeAction => "05-streaming-realtime-rules",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilitySourceKind {
    SdkBuiltIn,
    HostProvided,
    ToolPack,
    McpServer,
    Extension,
    Subagent,
    DiscoveryIndex,
    TestOnlyFake,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CapabilitySource {
    pub kind: CapabilitySourceKind,
    pub source_ref: SourceRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter_ref: Option<AdapterRef>,
}

impl CapabilitySource {
    pub fn test_fake(source_ref: SourceRef) -> Self {
        Self {
            kind: CapabilitySourceKind::TestOnlyFake,
            source_ref,
            adapter_ref: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityVisibility {
    Active,
    Hidden,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionMode {
    NotProjected,
    DescriptorOnly,
    ProviderToolSchema { schema_ref: PackageSidecarRef },
    ProducesContextItems { allowed_kinds: Vec<String> },
    ProjectsContextRefs { allowed_ref_kinds: Vec<String> },
}

impl ProjectionMode {
    pub fn is_projected(&self) -> bool {
        !matches!(self, Self::NotProjected)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PackageSidecarRef {
    pub sidecar_id: String,
    pub kind: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
}

impl PackageSidecarRef {
    pub fn new(
        sidecar_id: impl Into<String>,
        kind: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self {
            sidecar_id: sidecar_id.into(),
            kind: kind.into(),
            version: version.into(),
            content_hash: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityReadinessStatus {
    Active,
    ReservedInactive,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CapabilityReadiness {
    pub status: CapabilityReadinessStatus,
    pub owner_role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub typed_sidecar_contract: Option<String>,
    #[serde(default)]
    pub fingerprint_fields: Vec<String>,
    #[serde(default)]
    pub emitted_events: Vec<String>,
    #[serde(default)]
    pub journal_records: Vec<String>,
    #[serde(default)]
    pub acceptance_tests: Vec<String>,
}

impl CapabilityReadiness {
    pub fn active_tool() -> Self {
        Self {
            status: CapabilityReadinessStatus::Active,
            owner_role: CapabilityKind::Tool.owner_role().to_string(),
            typed_sidecar_contract: Some(
                "capability.tool.schema_ref.executor_ref.policy_ref".to_string(),
            ),
            fingerprint_fields: vec![
                "capability_id".to_string(),
                "kind".to_string(),
                "namespace".to_string(),
                "version".to_string(),
                "projection".to_string(),
                "executor_ref".to_string(),
                "policy_ref".to_string(),
                "sidecar_refs".to_string(),
                "source".to_string(),
            ],
            emitted_events: vec![
                "capability_loaded".to_string(),
                "tool_requested".to_string(),
                "tool_completed".to_string(),
            ],
            journal_records: vec![
                "package_catalog_snapshot".to_string(),
                "package_delta".to_string(),
                "tool_execution_intent".to_string(),
                "tool_execution_result".to_string(),
            ],
            acceptance_tests: vec![
                "provider_visible_capability_requires_executor_and_policy_refs".to_string(),
                "projection_and_execution_hashes_match".to_string(),
            ],
        }
    }

    pub fn reserved(kind: &CapabilityKind) -> Self {
        Self {
            status: CapabilityReadinessStatus::ReservedInactive,
            owner_role: kind.owner_role().to_string(),
            typed_sidecar_contract: None,
            fingerprint_fields: Vec::new(),
            emitted_events: Vec::new(),
            journal_records: Vec::new(),
            acceptance_tests: Vec::new(),
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self.status, CapabilityReadinessStatus::Active)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CapabilitySpec {
    pub capability_id: CapabilityId,
    pub kind: CapabilityKind,
    pub source: CapabilitySource,
    pub namespace: CapabilityNamespace,
    pub version: CapabilityVersion,
    pub visibility: CapabilityVisibility,
    pub projection: ProjectionMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executor_ref: Option<ExecutorRef>,
    pub policy_ref: PolicyRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sidecar_refs: Vec<PackageSidecarRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub isolation_ref: Option<PackageSidecarRef>,
    pub privacy: PrivacyClass,
    pub readiness: CapabilityReadiness,
}

impl CapabilitySpec {
    pub fn fake_tool(
        capability_id: impl Into<CapabilityId>,
        name: impl Into<String>,
        schema_ref: PackageSidecarRef,
        executor_ref: ExecutorRef,
        policy_ref: PolicyRef,
        source: SourceRef,
    ) -> Self {
        let name = name.into();
        Self {
            capability_id: capability_id.into(),
            kind: CapabilityKind::Tool,
            source: CapabilitySource::test_fake(source),
            namespace: CapabilityNamespace::new(format!("tool.{name}")),
            version: CapabilityVersion::new("v1"),
            visibility: CapabilityVisibility::Active,
            projection: ProjectionMode::ProviderToolSchema { schema_ref },
            executor_ref: Some(executor_ref),
            policy_ref,
            sidecar_refs: Vec::new(),
            isolation_ref: None,
            privacy: PrivacyClass::ContentRefsOnly,
            readiness: CapabilityReadiness::active_tool(),
        }
    }

    pub fn reserved_inactive(
        capability_id: impl Into<CapabilityId>,
        kind: CapabilityKind,
        policy_ref: PolicyRef,
        source: SourceRef,
    ) -> Self {
        let owner = kind.owner_role().replace('-', ".");
        Self {
            capability_id: capability_id.into(),
            kind: kind.clone(),
            source: CapabilitySource::test_fake(source),
            namespace: CapabilityNamespace::new(format!("reserved.{owner}")),
            version: CapabilityVersion::new("reserved"),
            visibility: CapabilityVisibility::Hidden,
            projection: ProjectionMode::NotProjected,
            executor_ref: None,
            policy_ref,
            sidecar_refs: Vec::new(),
            isolation_ref: None,
            privacy: PrivacyClass::Internal,
            readiness: CapabilityReadiness::reserved(&kind),
        }
    }

    pub fn provider_visible(&self) -> bool {
        self.visibility == CapabilityVisibility::Active && self.projection.is_projected()
    }

    pub fn executable(&self) -> bool {
        self.executor_ref.is_some()
    }

    pub fn project_for_provider(&self) -> Result<Option<ProviderCapabilityProjection>, AgentError> {
        self.validate()?;
        if !self.provider_visible() {
            return Ok(None);
        }
        Ok(Some(ProviderCapabilityProjection {
            capability_id: self.capability_id.clone(),
            namespace: self.namespace.clone(),
            projection: self.projection.clone(),
            policy_ref: self.policy_ref.clone(),
        }))
    }

    pub fn executable_route(&self) -> Result<Option<ExecutableCapabilityRoute>, AgentError> {
        self.validate()?;
        let Some(executor_ref) = self.executor_ref.clone() else {
            return Ok(None);
        };
        Ok(Some(ExecutableCapabilityRoute {
            capability_id: self.capability_id.clone(),
            executor_ref,
            policy_ref: self.policy_ref.clone(),
        }))
    }

    pub fn validate(&self) -> Result<(), AgentError> {
        if self.policy_ref.as_str().is_empty() {
            return Err(AgentError::missing_required_field("capability.policy_ref"));
        }
        if self.kind.is_reserved() && self.readiness.is_active() {
            return Err(AgentError::contract_violation(format!(
                "reserved capability {} cannot be active until its owner supplies sidecar, fingerprint, event, journal, and acceptance-test evidence",
                self.capability_id.as_str()
            )));
        }
        if self.kind.is_reserved() && self.projection.is_projected() {
            return Err(AgentError::contract_violation(format!(
                "reserved capability {} cannot be projected while inactive",
                self.capability_id.as_str()
            )));
        }
        if self.kind.is_reserved() && self.executor_ref.is_some() {
            return Err(AgentError::contract_violation(format!(
                "reserved capability {} cannot execute while inactive",
                self.capability_id.as_str()
            )));
        }
        if self.provider_visible() && self.executor_ref.is_none() {
            return Err(AgentError::missing_required_field(
                "provider_visible_capability.executor_ref",
            ));
        }
        if self.provider_visible() && self.policy_ref.kind == PolicyKind::Host {
            return Err(AgentError::contract_violation(
                "provider-visible capability must use an explicit non-host policy ref",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProviderCapabilityProjection {
    pub capability_id: CapabilityId,
    pub namespace: CapabilityNamespace,
    pub projection: ProjectionMode,
    pub policy_ref: PolicyRef,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExecutableCapabilityRoute {
    pub capability_id: CapabilityId,
    pub executor_ref: ExecutorRef,
    pub policy_ref: PolicyRef,
}
