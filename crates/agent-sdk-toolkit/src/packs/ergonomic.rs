//! Ergonomic tool declaration wrappers for toolkit-owned tool packs.
//! These helpers are data-only: they assemble core tool-pack snapshots,
//! capabilities, and routes, but never execute a tool or bypass core policy,
//! journal, event, or effect contracts.
//!
use agent_sdk_core::{
    AgentError, CapabilityId, CapabilityNamespace, CapabilityPermission, ExecutorRef,
    PackageSidecarRef, PolicyKind, PolicyRef, PrivacyClass, SourceRef, ToolPackId, ToolPackKind,
    ToolPackSnapshot, ToolPackToolSnapshot, TrustClass,
    policy::{EffectClass, RiskClass},
    tool_records::CanonicalToolName,
};

use crate::packs::ToolkitPackBundle;

#[derive(Clone, Debug, Eq, PartialEq)]
/// Selects how a declared toolkit tool is expected to complete.
/// This is package metadata only; execution still goes through the core
/// `ToolExecutor` and `ToolExecutionCoordinator`.
pub enum ToolkitToolExecutionMode {
    /// Synchronous or short-lived tool execution.
    Sync,
    /// Longer-running or externally continued execution.
    Async,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Data-only declaration for a synchronous toolkit tool.
/// Lower it into a `ToolkitPackBundle`; do not execute it directly.
pub struct Tool {
    snapshot: ToolPackToolSnapshot,
    mode: ToolkitToolExecutionMode,
}

impl Tool {
    /// Starts a data-only builder for a synchronous toolkit tool.
    pub fn builder(
        tool_name: impl Into<String>,
        executor_ref: impl Into<String>,
        schema_id: impl Into<String>,
        policy_ref: PolicyRef,
    ) -> ToolBuilder {
        ToolBuilder::new(tool_name, executor_ref, schema_id, policy_ref)
    }

    /// Returns the canonical tool name exposed to providers.
    pub fn canonical_tool_name(&self) -> &CanonicalToolName {
        &self.snapshot.canonical_tool_name
    }

    /// Returns the declared executor ref without resolving or executing it.
    pub fn executor_ref(&self) -> &ExecutorRef {
        &self.snapshot.executor_ref
    }

    /// Returns the declaration mode.
    pub fn mode(&self) -> &ToolkitToolExecutionMode {
        &self.mode
    }

    /// Returns the core tool-pack snapshot this wrapper lowers to.
    pub fn snapshot(&self) -> &ToolPackToolSnapshot {
        &self.snapshot
    }

    /// Consumes this wrapper into the core tool-pack snapshot.
    pub fn into_snapshot(self) -> ToolPackToolSnapshot {
        self.snapshot
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Data-only declaration for an async or externally continued toolkit tool.
/// The SDK still requires a core `ToolExecutor` and journaled terminal result
/// before provider continuation.
pub struct AsyncTool {
    snapshot: ToolPackToolSnapshot,
    mode: ToolkitToolExecutionMode,
}

impl AsyncTool {
    /// Starts a data-only builder for an async toolkit tool.
    pub fn builder(
        tool_name: impl Into<String>,
        executor_ref: impl Into<String>,
        schema_id: impl Into<String>,
        policy_ref: PolicyRef,
    ) -> ToolBuilder {
        ToolBuilder::new(tool_name, executor_ref, schema_id, policy_ref).async_mode()
    }

    /// Returns the canonical tool name exposed to providers.
    pub fn canonical_tool_name(&self) -> &CanonicalToolName {
        &self.snapshot.canonical_tool_name
    }

    /// Returns the declared executor ref without resolving or executing it.
    pub fn executor_ref(&self) -> &ExecutorRef {
        &self.snapshot.executor_ref
    }

    /// Returns the declaration mode.
    pub fn mode(&self) -> &ToolkitToolExecutionMode {
        &self.mode
    }

    /// Returns the core tool-pack snapshot this wrapper lowers to.
    pub fn snapshot(&self) -> &ToolPackToolSnapshot {
        &self.snapshot
    }

    /// Consumes this wrapper into the core tool-pack snapshot.
    pub fn into_snapshot(self) -> ToolPackToolSnapshot {
        self.snapshot
    }
}

#[derive(Clone, Debug)]
/// Builder for ergonomic toolkit tool declarations.
/// The builder requires an explicit effect and risk class before it can
/// produce a `Tool` or `AsyncTool`.
pub struct ToolBuilder {
    tool_name: String,
    executor_ref: String,
    schema_id: String,
    redacted_schema: Option<serde_json::Value>,
    policy_refs: Vec<PolicyRef>,
    requires_approval: bool,
    capability_id: Option<CapabilityId>,
    namespace: Option<CapabilityNamespace>,
    required_permissions: Vec<CapabilityPermission>,
    effect_class: Option<EffectClass>,
    risk_class: Option<RiskClass>,
    redaction_policy_ref: PolicyRef,
    timeout_ms: u64,
    cancellation: String,
    reconciliation: String,
    privacy: PrivacyClass,
    mode: ToolkitToolExecutionMode,
}

impl ToolBuilder {
    /// Creates a new data-only tool declaration builder.
    pub fn new(
        tool_name: impl Into<String>,
        executor_ref: impl Into<String>,
        schema_id: impl Into<String>,
        policy_ref: PolicyRef,
    ) -> Self {
        Self {
            tool_name: tool_name.into(),
            executor_ref: executor_ref.into(),
            schema_id: schema_id.into(),
            redacted_schema: None,
            policy_refs: vec![policy_ref],
            requires_approval: false,
            capability_id: None,
            namespace: None,
            required_permissions: Vec::new(),
            effect_class: None,
            risk_class: None,
            redaction_policy_ref: PolicyRef::with_kind(
                PolicyKind::Redaction,
                "policy.redaction.tool_result.refs_only",
            ),
            timeout_ms: 10_000,
            cancellation: "best_effort".to_string(),
            reconciliation: "effect_lineage_required".to_string(),
            privacy: PrivacyClass::ContentRefsOnly,
            mode: ToolkitToolExecutionMode::Sync,
        }
    }

    /// Marks this declaration as a read-like tool.
    pub fn read_only(mut self) -> Self {
        self.effect_class = Some(EffectClass::Read);
        self.risk_class = Some(RiskClass::Low);
        self
    }

    /// Marks this declaration as a write-like tool.
    pub fn write_effect(mut self) -> Self {
        self.effect_class = Some(EffectClass::Write);
        self.risk_class = Some(RiskClass::High);
        self
    }

    /// Sets an explicit effect and risk class.
    pub fn effect(mut self, effect_class: EffectClass, risk_class: RiskClass) -> Self {
        self.effect_class = Some(effect_class);
        self.risk_class = Some(risk_class);
        self
    }

    /// Sets the stable capability id used by the runtime package.
    pub fn capability_id(mut self, capability_id: CapabilityId) -> Self {
        self.capability_id = Some(capability_id);
        self
    }

    /// Sets the capability namespace.
    pub fn namespace(mut self, namespace: CapabilityNamespace) -> Self {
        self.namespace = Some(namespace);
        self
    }

    /// Adds a required permission to the provider-visible tool snapshot.
    pub fn required_permission(mut self, permission: CapabilityPermission) -> Self {
        self.required_permissions.push(permission);
        self
    }

    /// Attaches a provider-safe JSON schema body to this tool declaration.
    pub fn redacted_schema(mut self, schema: serde_json::Value) -> Self {
        self.redacted_schema = Some(schema);
        self
    }

    /// Adds another policy ref that must travel with the tool declaration.
    pub fn policy_ref(mut self, policy_ref: PolicyRef) -> Self {
        self.policy_refs.push(policy_ref);
        self
    }

    /// Requires host approval before the core runtime releases the executor.
    pub fn require_approval(mut self) -> Self {
        self.requires_approval = true;
        self.policy_refs.push(PolicyRef::with_kind(
            PolicyKind::Approval,
            format!("policy.approval.{}", self.tool_name),
        ));
        self.risk_class = Some(RiskClass::High);
        self
    }

    /// Sets the redaction policy ref for tool results.
    pub fn redaction_policy(mut self, policy_ref: PolicyRef) -> Self {
        self.redaction_policy_ref = policy_ref;
        self
    }

    /// Sets the execution timeout metadata.
    pub fn timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Sets the cancellation metadata.
    pub fn cancellation(mut self, cancellation: impl Into<String>) -> Self {
        self.cancellation = cancellation.into();
        self
    }

    /// Sets the reconciliation metadata.
    pub fn reconciliation(mut self, reconciliation: impl Into<String>) -> Self {
        self.reconciliation = reconciliation.into();
        self
    }

    /// Sets the privacy class used for the tool snapshot and route.
    pub fn privacy(mut self, privacy: PrivacyClass) -> Self {
        self.privacy = privacy;
        self
    }

    /// Marks the declaration as async metadata.
    pub fn async_mode(mut self) -> Self {
        self.mode = ToolkitToolExecutionMode::Async;
        self.timeout_ms = self.timeout_ms.max(60_000);
        self.cancellation = "cooperative".to_string();
        self
    }

    /// Builds a synchronous tool declaration.
    pub fn build(self) -> Result<Tool, AgentError> {
        let mode = self.mode.clone();
        Ok(Tool {
            snapshot: self.build_snapshot()?,
            mode,
        })
    }

    /// Builds an async tool declaration.
    pub fn build_async(self) -> Result<AsyncTool, AgentError> {
        let builder = self.async_mode();
        let mode = builder.mode.clone();
        Ok(AsyncTool {
            snapshot: builder.build_snapshot()?,
            mode,
        })
    }

    fn build_snapshot(self) -> Result<ToolPackToolSnapshot, AgentError> {
        let effect_class = self
            .effect_class
            .ok_or_else(|| AgentError::missing_required_field("tool.effect_class"))?;
        let risk_class = self
            .risk_class
            .ok_or_else(|| AgentError::missing_required_field("tool.risk_class"))?;
        let capability_id = self
            .capability_id
            .unwrap_or_else(|| CapabilityId::new(format!("cap.tool.{}", self.tool_name)));
        let namespace = self
            .namespace
            .unwrap_or_else(|| CapabilityNamespace::new(format!("tool.{}", self.tool_name)));
        Ok(ToolPackToolSnapshot {
            capability_id,
            canonical_tool_name: CanonicalToolName::new(self.tool_name),
            namespace,
            schema_ref: PackageSidecarRef::new(self.schema_id, "tool_schema", "v1"),
            redacted_schema: self.redacted_schema,
            executor_ref: ExecutorRef::new(self.executor_ref),
            policy_refs: self.policy_refs,
            requires_approval: self.requires_approval,
            required_permissions: self.required_permissions,
            effect_class,
            risk_class,
            redaction_policy_ref: self.redaction_policy_ref,
            timeout_ms: self.timeout_ms,
            cancellation: self.cancellation,
            reconciliation: self.reconciliation,
            privacy: self.privacy,
        })
    }
}

#[derive(Clone, Debug)]
/// Builder for a toolkit pack assembled from ergonomic `Tool` and
/// `AsyncTool` declarations. Registering declarations with `listen` is
/// data-only; execution still requires a core runtime and executor registry.
pub struct ToolPackBuilder {
    pack_id: ToolPackId,
    kind: ToolPackKind,
    version: String,
    source: SourceRef,
    trust: TrustClass,
    tools: Vec<ToolPackToolSnapshot>,
}

impl ToolPackBuilder {
    /// Creates a data-only toolkit pack builder.
    pub fn new(
        pack_id: ToolPackId,
        kind: ToolPackKind,
        version: impl Into<String>,
        source: SourceRef,
    ) -> Self {
        Self {
            pack_id,
            kind,
            version: version.into(),
            source,
            trust: TrustClass::SdkGenerated,
            tools: Vec::new(),
        }
    }

    /// Sets the trust class for the generated tool-pack snapshot.
    pub fn trust(mut self, trust: TrustClass) -> Self {
        self.trust = trust;
        self
    }

    /// Listens to a synchronous tool declaration by adding it to the
    /// generated tool-pack snapshot. This does not start an executor.
    pub fn listen(mut self, tool: Tool) -> Self {
        self.tools.push(tool.into_snapshot());
        self
    }

    /// Listens to an async tool declaration by adding it to the generated
    /// tool-pack snapshot. This does not start a task or runtime.
    pub fn listen_async(mut self, tool: AsyncTool) -> Self {
        self.tools.push(tool.into_snapshot());
        self
    }

    /// Builds the toolkit pack bundle that core can install into a
    /// `RuntimePackageBuilder`.
    pub fn build(self) -> Result<ToolkitPackBundle, AgentError> {
        let mut snapshot =
            ToolPackSnapshot::new(self.pack_id, self.kind, self.version, self.source)
                .with_trust(self.trust);
        for tool in self.tools {
            snapshot = snapshot.with_tool(tool);
        }
        ToolkitPackBundle::from_snapshot(snapshot)
    }
}
