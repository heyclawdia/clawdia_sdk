//! Tool discovery helpers for optional toolkit capabilities. Use these modules to
//! index hidden candidates, return model-facing discovery results, and construct
//! package deltas for host-approved activation. Searching is data-only; package
//! mutation happens only when a delta is applied. This file contains the executor
//! portion of that contract.
//!
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
/// Discovery tool discovery executor request or result value.
/// Creating the value does not register tools; discovery executors document catalog and package-bundle effects.
pub struct ToolDiscoveryExecutor {
    executor_ref: ExecutorRef,
    index: ToolDiscoveryIndex,
    arguments: InMemoryJsonArgumentStore,
    content: InMemoryToolkitContentStore,
}

impl ToolDiscoveryExecutor {
    /// Creates a new discovery::executor value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
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

    /// Returns the pack bundle currently held by this value.
    /// This returns the toolkit pack bundle that registers the executor routes; it does not
    /// activate the pack itself.
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
