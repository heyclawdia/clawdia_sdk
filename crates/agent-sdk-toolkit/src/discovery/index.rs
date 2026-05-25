//! Tool discovery index and activation delta helpers. Use this module to search
//! hidden toolkit candidates and construct package deltas for later activation.
//! Searching is data-only; applying the returned delta is a separate package step.
//!
use std::collections::BTreeMap;

use agent_sdk_core::{
    AgentError, CapabilityCatalogSnapshot, CapabilityId, CapabilitySourceKind, PackageDelta,
    PolicyRef, ToolPackSnapshot, TrustClass,
};

use super::types::ToolDiscoveryCandidate;

#[derive(Clone, Debug, Default)]
/// Discovery tool discovery index request or result value.
/// Creating the value does not register tools; discovery executors document catalog and package-bundle effects.
pub struct ToolDiscoveryIndex {
    candidates: BTreeMap<String, ToolPackSnapshot>,
}

impl ToolDiscoveryIndex {
    /// Creates a new discovery::index value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds data to this in-memory discovery::index collection. It does not
    /// perform external I/O, execute tools, or append journals.
    pub fn insert(&mut self, snapshot: ToolPackSnapshot) {
        self.candidates
            .insert(snapshot.pack_id.as_str().to_string(), snapshot);
    }

    /// Searches the in-memory discovery index for pack IDs or tool names that
    /// contain the query string. This is read-only and does not activate the
    /// returned candidates.
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

    /// Builds the package delta needed to activate one discovered candidate.
    /// The active runtime package is not mutated until the caller applies the
    /// returned delta through `RuntimePackage::apply_delta`.
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
