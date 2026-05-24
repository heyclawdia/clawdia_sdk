use std::sync::Arc;

use agent_sdk_core::{
    AgentError, ExecutorRef, PolicyKind, PolicyRef, ResourceReadRequest, ResourceResolution,
    ResourceResolver, ResourceRouteSnapshot, ResourceRouter, ResourceScheme, RetentionClass,
    ToolExecutionOutput, ToolExecutionRequest, ToolExecutor, ToolPackId, ToolPackKind,
    ToolPackSnapshot,
    domain::ContentRef,
    policy::{CapabilityPermission, EffectClass, RiskClass},
};
use serde::{Deserialize, Serialize};

use crate::{
    packs::{ToolkitPackBundle, tool_snapshot},
    testing::{InMemoryJsonArgumentStore, InMemoryToolkitContentStore},
};

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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResourceReaderRequest {
    pub uri: String,
    pub max_bytes: u64,
}

#[derive(Clone)]
pub struct InMemoryResourceResolver {
    scheme: ResourceScheme,
    content_ref: ContentRef,
    source: agent_sdk_core::SourceRef,
    policy_ref: PolicyRef,
}

impl InMemoryResourceResolver {
    pub fn new(
        scheme: &str,
        content_ref: ContentRef,
        source: agent_sdk_core::SourceRef,
        policy_ref: PolicyRef,
    ) -> Arc<Self> {
        Arc::new(Self {
            scheme: ResourceScheme::new(scheme),
            content_ref,
            source,
            policy_ref,
        })
    }
}

impl ResourceResolver for InMemoryResourceResolver {
    fn scheme(&self) -> &ResourceScheme {
        &self.scheme
    }

    fn resolve(&self, request: &ResourceReadRequest) -> Result<ResourceResolution, AgentError> {
        Ok(ResourceResolution {
            uri: request.uri.clone(),
            scheme: self.scheme.clone(),
            content_ref: self.content_ref.clone(),
            source: self.source.clone(),
            policy_refs: vec![self.policy_ref.clone()],
            byte_len: 0,
            truncated: false,
            parser_version: "toolkit.in_memory_resource.v1".to_string(),
            privacy: agent_sdk_core::PrivacyClass::ContentRefsOnly,
            retention: RetentionClass::RunScoped,
            redacted_summary: "in-memory resource resolved to content ref".to_string(),
        })
    }
}

pub fn memory_read_policy(id: &str) -> PolicyRef {
    PolicyRef::with_kind(PolicyKind::Permission, id)
}
