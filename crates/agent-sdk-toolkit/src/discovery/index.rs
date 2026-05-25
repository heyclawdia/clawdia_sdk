use std::collections::BTreeMap;

use agent_sdk_core::{
    AgentError, CapabilityCatalogSnapshot, CapabilityId, CapabilitySourceKind, PackageDelta,
    PolicyRef, ToolPackSnapshot, TrustClass,
};

use super::types::ToolDiscoveryCandidate;

#[derive(Clone, Debug, Default)]
pub struct ToolDiscoveryIndex {
    candidates: BTreeMap<String, ToolPackSnapshot>,
}

impl ToolDiscoveryIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, snapshot: ToolPackSnapshot) {
        self.candidates
            .insert(snapshot.pack_id.as_str().to_string(), snapshot);
    }

    pub fn search(&self, query: &str) -> Vec<ToolDiscoveryCandidate> {
        self.candidates
            .iter()
            .filter(|(id, snapshot)| {
                id.contains(query)
                    || snapshot
                        .tools
                        .iter()
                        .any(|tool| tool.canonical_tool_name.as_str().contains(query))
            })
            .map(|(id, snapshot)| ToolDiscoveryCandidate {
                pack_id: id.clone(),
                tool_names: snapshot
                    .tools
                    .iter()
                    .map(|tool| tool.canonical_tool_name.as_str().to_string())
                    .collect(),
                package_delta_required: true,
            })
            .collect()
    }

    pub fn activation_delta(
        &self,
        pack_id: &str,
        package: &agent_sdk_core::RuntimePackage,
        requested_by: agent_sdk_core::SourceRef,
        activation_policy_ref: PolicyRef,
    ) -> Result<PackageDelta, AgentError> {
        let snapshot = self
            .candidates
            .get(pack_id)
            .cloned()
            .ok_or_else(|| AgentError::missing_required_field("tool_discovery.candidate"))?;
        let capabilities = snapshot.capability_specs()?;
        let catalog = CapabilityCatalogSnapshot {
            catalog_id: format!("catalog.discovery.{pack_id}"),
            source_kind: CapabilitySourceKind::DiscoveryIndex,
            source_ref: requested_by.clone(),
            version: Some("v1".to_string()),
            content_hash: Some(snapshot.content_hash()?),
            trust_state: TrustClass::SdkGenerated,
            activation_policy_ref,
            candidates: capabilities
                .iter()
                .map(|capability| capability.capability_id.clone())
                .collect::<Vec<CapabilityId>>(),
        };
        Ok(PackageDelta {
            previous_fingerprint: package.fingerprint()?,
            requested_by,
            reason: "activate hidden tool discovery candidate for next package".to_string(),
            activated_capabilities: capabilities,
            deactivated_capability_ids: Vec::new(),
            catalogs: vec![catalog],
            sidecars: vec![snapshot.package_sidecar_snapshot()?],
        })
    }
}
