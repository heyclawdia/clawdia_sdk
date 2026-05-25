use agent_sdk_core::{
    AgentError, ExecutorRef, PolicyRef, ResourceReadRequest, ResourceRouteSnapshot, ResourceRouter,
    ToolExecutionOutput, ToolExecutionRequest, ToolExecutor, ToolPackId, ToolPackKind,
    ToolPackSnapshot,
    domain::ContentRef,
    policy::{CapabilityPermission, EffectClass, RiskClass},
};

use crate::{
    packs::{ToolkitPackBundle, tool_snapshot},
    testing::{InMemoryJsonArgumentStore, InMemoryToolkitContentStore},
};

use super::types::ResourceReaderRequest;

#[derive(Clone)]
pub struct ResourceReaderExecutor {
    executor_ref: ExecutorRef,
    router: ResourceRouter,
    arguments: InMemoryJsonArgumentStore,
    content: InMemoryToolkitContentStore,
}

impl ResourceReaderExecutor {
    pub fn new(
        router: ResourceRouter,
        arguments: InMemoryJsonArgumentStore,
        content: InMemoryToolkitContentStore,
    ) -> Self {
        Self {
            executor_ref: ExecutorRef::new("executor.toolkit.resource_reader.v1"),
            router,
            arguments,
            content,
        }
    }

    pub fn pack_bundle(
        source: agent_sdk_core::SourceRef,
        policy_ref: PolicyRef,
    ) -> Result<ToolkitPackBundle, AgentError> {
        let snapshot = ToolPackSnapshot::new(
            ToolPackId::new("toolpack.resource_reader.v1"),
            ToolPackKind::ResourceReaders,
            "v1",
            source.clone(),
        )
        .with_resource_route(ResourceRouteSnapshot {
            scheme: "memory".to_string(),
            source: source.clone(),
            permission_policy_ref: policy_ref.clone(),
            parser_version: "toolkit.resource.memory.v1".to_string(),
            max_bytes: 64 * 1024,
            privacy: agent_sdk_core::PrivacyClass::ContentRefsOnly,
        })
        .with_tool(tool_snapshot(
            "cap.toolkit.resource_reader",
            "resource_read",
            "executor.toolkit.resource_reader.v1",
            "schema.toolkit.resource_reader.v1",
            vec![policy_ref],
            vec![CapabilityPermission::MemoryRead],
            EffectClass::Read,
            RiskClass::Medium,
            &source,
        ));
        ToolkitPackBundle::from_snapshot(snapshot)
    }
}

impl ToolExecutor for ResourceReaderExecutor {
    fn executor_ref(&self) -> &ExecutorRef {
        &self.executor_ref
    }

    fn execute(&self, request: &ToolExecutionRequest) -> Result<ToolExecutionOutput, AgentError> {
        let args_ref = request.effect_intent.content_refs.first().ok_or_else(|| {
            AgentError::missing_required_field("resource_read.argument_content_ref")
        })?;
        let resource_request: ResourceReaderRequest = self.arguments.get(args_ref)?;
        let resolution = self.router.resolve(&ResourceReadRequest {
            uri: resource_request.uri,
            source: request.resolved_call.request.source.clone(),
            policy_refs: request.resolved_call.route.policy_refs.clone(),
            max_bytes: resource_request.max_bytes,
        })?;
        let content_ref = ContentRef::new(format!(
            "content.{}.resource",
            request.resolved_call.request.tool_call_id.as_str()
        ));
        self.content.put(content_ref.clone(), &resolution)?;
        let mut output = ToolExecutionOutput::completed("resource reader returned content ref");
        output.content_refs.push(content_ref);
        Ok(output)
    }
}
