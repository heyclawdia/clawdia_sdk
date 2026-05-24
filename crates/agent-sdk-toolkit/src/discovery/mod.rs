use std::collections::BTreeMap;

use agent_sdk_core::{
    AgentError, CapabilityCatalogSnapshot, CapabilityId, CapabilitySourceKind, ExecutorRef,
    PackageDelta, PolicyKind, PolicyRef, ToolExecutionOutput, ToolExecutionRequest, ToolExecutor,
    ToolPackId, ToolPackKind, ToolPackSnapshot, TrustClass,
    domain::ContentRef,
    policy::{CapabilityPermission, EffectClass, RiskClass},
};
use serde::{Deserialize, Serialize};

use crate::{
    packs::{ToolkitPackBundle, tool_snapshot},
    testing::{InMemoryJsonArgumentStore, InMemoryToolkitContentStore},
};

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

#[derive(Clone)]
pub struct ToolDiscoveryExecutor {
    executor_ref: ExecutorRef,
    index: ToolDiscoveryIndex,
    arguments: InMemoryJsonArgumentStore,
    content: InMemoryToolkitContentStore,
}

impl ToolDiscoveryExecutor {
    pub fn new(
        index: ToolDiscoveryIndex,
        arguments: InMemoryJsonArgumentStore,
        content: InMemoryToolkitContentStore,
    ) -> Self {
        Self {
            executor_ref: ExecutorRef::new("executor.toolkit.tool_discovery.v1"),
            index,
            arguments,
            content,
        }
    }

    pub fn pack_bundle(
        source: agent_sdk_core::SourceRef,
        policy_ref: PolicyRef,
    ) -> Result<ToolkitPackBundle, AgentError> {
        let snapshot = ToolPackSnapshot::new(
            ToolPackId::new("toolpack.tool_discovery.v1"),
            ToolPackKind::ToolDiscovery,
            "v1",
            source.clone(),
        )
        .with_discovery(agent_sdk_core::ToolDiscoverySnapshot {
            discovery_index_id: "toolkit.discovery.default".to_string(),
            activation_policy_ref: policy_ref.clone(),
            package_delta_required: true,
        })
        .with_tool(tool_snapshot(
            "cap.toolkit.tool_discovery",
            "tool_discovery",
            "executor.toolkit.tool_discovery.v1",
            "schema.toolkit.tool_discovery.v1",
            vec![policy_ref],
            vec![CapabilityPermission::FilesystemRead],
            EffectClass::Read,
            RiskClass::Low,
            &source,
        ));
        ToolkitPackBundle::from_snapshot(snapshot)
    }
}

impl ToolExecutor for ToolDiscoveryExecutor {
    fn executor_ref(&self) -> &ExecutorRef {
        &self.executor_ref
    }

    fn execute(&self, request: &ToolExecutionRequest) -> Result<ToolExecutionOutput, AgentError> {
        let args_ref = request.effect_intent.content_refs.first().ok_or_else(|| {
            AgentError::missing_required_field("tool_discovery.argument_content_ref")
        })?;
        let discovery_request: ToolDiscoveryRequest = self.arguments.get(args_ref)?;
        let output = ToolDiscoveryOutput {
            query: discovery_request.query.clone(),
            candidates: self.index.search(&discovery_request.query),
            package_delta_required: true,
        };
        let content_ref = ContentRef::new(format!(
            "content.{}.tool_discovery",
            request.resolved_call.request.tool_call_id.as_str()
        ));
        self.content.put(content_ref.clone(), &output)?;
        let mut envelope = ToolExecutionOutput::completed(
            "tool discovery returned candidates without mutating active package",
        );
        envelope.content_refs.push(content_ref);
        Ok(envelope)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ToolDiscoveryRequest {
    pub query: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ToolDiscoveryOutput {
    pub query: String,
    pub candidates: Vec<ToolDiscoveryCandidate>,
    pub package_delta_required: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ToolDiscoveryCandidate {
    pub pack_id: String,
    pub tool_names: Vec<String>,
    pub package_delta_required: bool,
}

pub fn discovery_policy(id: &str) -> PolicyRef {
    PolicyRef::with_kind(PolicyKind::RuntimePackage, id)
}
