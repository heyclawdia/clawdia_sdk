use agent_sdk_core::{
    AgentError, ExecutorRef, PolicyRef, ToolExecutionOutput, ToolExecutionRequest, ToolExecutor,
    ToolPackId, ToolPackKind, ToolPackSnapshot,
    domain::ContentRef,
    policy::{CapabilityPermission, EffectClass, RiskClass},
};

use crate::{
    packs::{ToolkitPackBundle, tool_snapshot},
    testing::{InMemoryJsonArgumentStore, InMemoryToolkitContentStore},
};

use super::{
    index::ToolDiscoveryIndex,
    types::{ToolDiscoveryOutput, ToolDiscoveryRequest},
};

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
