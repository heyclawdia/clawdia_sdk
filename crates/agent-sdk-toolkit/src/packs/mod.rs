use agent_sdk_core::{
    AgentError, CapabilityId, CapabilityNamespace, DestinationKind, DestinationRef, ExecutorRef,
    PackageSidecarSnapshot, PolicyRef, PrivacyClass, RetentionClass, RuntimePackageBuilder,
    SourceRef, ToolPackSnapshot, ToolPackToolSnapshot, ToolRoute,
    policy::{CapabilityPermission, EffectClass, RiskClass},
    tool_records::CanonicalToolName,
};

#[derive(Clone, Debug)]
pub struct ToolkitPackBundle {
    pub snapshot: ToolPackSnapshot,
    pub sidecar: PackageSidecarSnapshot,
    pub capabilities: Vec<agent_sdk_core::CapabilitySpec>,
    pub routes: Vec<ToolRoute>,
}

impl ToolkitPackBundle {
    pub fn from_snapshot(snapshot: ToolPackSnapshot) -> Result<Self, AgentError> {
        let sidecar_ref = snapshot.sidecar_ref()?;
        let sidecar = snapshot.package_sidecar_snapshot()?;
        let capabilities = snapshot.capability_specs()?;
        let routes = snapshot
            .tools
            .iter()
            .map(|tool| ToolRoute {
                capability_id: tool.capability_id.clone(),
                canonical_tool_name: tool.canonical_tool_name.clone(),
                namespace: tool.namespace.clone(),
                source: snapshot.source.clone(),
                destination: DestinationRef::with_kind(
                    DestinationKind::Tool,
                    format!("destination.{}", tool.canonical_tool_name.as_str()),
                ),
                executor_ref: Some(tool.executor_ref.clone()),
                policy_refs: tool.policy_refs.clone(),
                sidecar_refs: vec![sidecar_ref.clone()],
                effect_class: tool.effect_class.clone(),
                risk_class: tool.risk_class.clone(),
                privacy: tool.privacy.clone(),
                retention: RetentionClass::RunScoped,
            })
            .collect();
        Ok(Self {
            snapshot,
            sidecar,
            capabilities,
            routes,
        })
    }

    pub fn install_into(&self, mut builder: RuntimePackageBuilder) -> RuntimePackageBuilder {
        builder = builder.sidecar(self.sidecar.clone());
        for capability in &self.capabilities {
            builder = builder.capability(capability.clone());
        }
        builder
    }
}

pub fn tool_snapshot(
    capability_id: &str,
    tool_name: &str,
    executor_ref: &str,
    schema_id: &str,
    policy_refs: Vec<PolicyRef>,
    required_permissions: Vec<CapabilityPermission>,
    effect_class: EffectClass,
    risk_class: RiskClass,
    _source: &SourceRef,
) -> ToolPackToolSnapshot {
    ToolPackToolSnapshot {
        capability_id: CapabilityId::new(capability_id),
        canonical_tool_name: CanonicalToolName::new(tool_name),
        namespace: CapabilityNamespace::new(format!("tool.{tool_name}")),
        schema_ref: agent_sdk_core::PackageSidecarRef::new(schema_id, "tool_schema", "v1"),
        executor_ref: ExecutorRef::new(executor_ref),
        policy_refs,
        required_permissions,
        effect_class,
        risk_class,
        redaction_policy_ref: PolicyRef::with_kind(
            agent_sdk_core::PolicyKind::Redaction,
            "policy.redaction.tool_result.refs_only",
        ),
        timeout_ms: 10_000,
        cancellation: "best_effort".to_string(),
        reconciliation: "effect_lineage_required".to_string(),
        privacy: PrivacyClass::ContentRefsOnly,
    }
}
