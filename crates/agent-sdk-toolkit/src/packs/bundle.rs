use agent_sdk_core::{
    AgentError, DestinationKind, DestinationRef, PackageSidecarSnapshot, RetentionClass,
    RuntimePackageBuilder, ToolPackSnapshot, ToolRoute,
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
