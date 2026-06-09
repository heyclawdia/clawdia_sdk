//! Toolkit pack assembly helpers. Use these modules to turn toolkit operations into
//! core package capabilities, sidecars, and routes. Pack assembly is data-only and
//! does not execute tools or mutate a runtime package until explicitly installed.
//! This file contains the bundle portion of that contract.
//!
use agent_sdk_core::{
    AgentError, DestinationKind, DestinationRef, PackageSidecarSnapshot, RetentionClass,
    RuntimePackageBuilder, ToolPackSnapshot, ToolRoute,
};

#[derive(Clone, Debug)]
/// Toolkit pack toolkit pack bundle value.
/// Use it to assemble exported tool-pack configuration; installation or activation effects are documented by the bundle executor.
pub struct ToolkitPackBundle {
    /// Snapshot used by this record or request.
    pub snapshot: ToolPackSnapshot,
    /// Sidecar used by this record or request.
    pub sidecar: PackageSidecarSnapshot,
    /// Capabilities frozen into the package or returned by an adapter health
    /// check.
    pub capabilities: Vec<agent_sdk_core::CapabilitySpec>,
    /// Collection of routes values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub routes: Vec<ToolRoute>,
}

impl ToolkitPackBundle {
    /// Constructs this value from snapshot. Use it when adapting
    /// canonical SDK records without introducing a second behavior
    /// path.
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
                description: tool.description.clone(),
                source: snapshot.source.clone(),
                destination: DestinationRef::with_kind(
                    DestinationKind::Tool,
                    format!("destination.{}", tool.canonical_tool_name.as_str()),
                ),
                executor_ref: Some(tool.executor_ref.clone()),
                policy_refs: tool.policy_refs.clone(),
                requires_approval: tool.requires_approval,
                sidecar_refs: vec![sidecar_ref.clone()],
                effect_class: tool.effect_class.clone(),
                risk_class: tool.risk_class.clone(),
                privacy: tool.privacy,
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

    /// Install into.
    /// This updates the provided RuntimePackageBuilder with bundle routes and sidecars; it does
    /// not activate tools by itself.
    pub fn install_into(&self, mut builder: RuntimePackageBuilder) -> RuntimePackageBuilder {
        builder = builder.sidecar(self.sidecar.clone());
        for capability in &self.capabilities {
            builder = builder.capability(capability.clone());
        }
        builder
    }
}
