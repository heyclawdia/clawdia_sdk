use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use serde::{Deserialize, Serialize};

use crate::{
    capability::{CapabilityId, CapabilityNamespace, ExecutorRef, PackageSidecarRef},
    domain::{
        AgentError, AgentErrorKind, ContentRef, DedupeKey, DestinationRef, EffectId,
        IdempotencyKey, PolicyRef, PrivacyClass, RetentionClass, RetryClassification, SourceRef,
        ToolCallId,
    },
    effect::{EffectIntent, EffectResult, EffectTerminalStatus},
    package::RuntimePackage,
    policy::{EffectClass, PolicyOutcome, PolicyStage, RiskClass},
    tool_records::CanonicalToolName,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ToolRegistrySnapshot {
    pub runtime_package_fingerprint: String,
    pub routes: Vec<ToolRoute>,
}

impl ToolRegistrySnapshot {
    pub fn from_runtime_package(
        package: &RuntimePackage,
        routes: impl IntoIterator<Item = ToolRoute>,
    ) -> Result<Self, AgentError> {
        let executable_routes = package.executable_routes()?;
        let executable_ids = executable_routes
            .iter()
            .map(|route| (route.capability_id.clone(), route.executor_ref.clone()))
            .collect::<BTreeMap<_, _>>();
        let package_policy_refs = executable_routes
            .iter()
            .map(|route| (route.capability_id.clone(), route.policy_ref.clone()))
            .collect::<BTreeMap<_, _>>();

        let mut seen_names = BTreeSet::new();
        let mut snapshot_routes = Vec::new();
        for route in routes {
            route.validate()?;
            if !seen_names.insert(route.canonical_tool_name.clone()) {
                return Err(AgentError::contract_violation(
                    "tool registry snapshot has duplicate canonical tool name",
                ));
            }

            let Some(package_executor_ref) = executable_ids.get(&route.capability_id) else {
                return Err(AgentError::new(
                    AgentErrorKind::InvalidPackage,
                    RetryClassification::HostConfigurationNeeded,
                    "tool route is not executable in the runtime package snapshot",
                ));
            };
            if route.executor_ref.as_ref() != Some(package_executor_ref) {
                return Err(AgentError::contract_violation(
                    "tool route executor_ref must match runtime package executable route",
                ));
            }

            let Some(package_policy_ref) = package_policy_refs.get(&route.capability_id) else {
                return Err(AgentError::contract_violation(
                    "tool route policy_ref missing from runtime package executable route",
                ));
            };
            if !route.policy_refs.contains(package_policy_ref) {
                return Err(AgentError::contract_violation(
                    "tool route policy_refs must include runtime package policy_ref",
                ));
            }

            snapshot_routes.push(route);
        }

        snapshot_routes.sort_by_key(|route| route.canonical_tool_name.as_str().to_string());
        Ok(Self {
            runtime_package_fingerprint: package.fingerprint()?.as_str().to_string(),
            routes: snapshot_routes,
        })
    }

    pub fn find_by_name(&self, name: &CanonicalToolName) -> Option<&ToolRoute> {
        self.routes
            .iter()
            .find(|route| &route.canonical_tool_name == name)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ToolRoute {
    pub capability_id: CapabilityId,
    pub canonical_tool_name: CanonicalToolName,
    pub namespace: CapabilityNamespace,
    pub source: SourceRef,
    pub destination: DestinationRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executor_ref: Option<ExecutorRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policy_refs: Vec<PolicyRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sidecar_refs: Vec<PackageSidecarRef>,
    pub effect_class: EffectClass,
    pub risk_class: RiskClass,
    pub privacy: PrivacyClass,
    pub retention: RetentionClass,
}

impl ToolRoute {
    pub fn validate(&self) -> Result<(), AgentError> {
        if self.executor_ref.is_none() {
            return Err(AgentError::missing_required_field(
                "tool_route.executor_ref",
            ));
        }
        if self.policy_refs.is_empty() {
            return Err(AgentError::missing_required_field("tool_route.policy_refs"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct ToolRouter {
    snapshot: ToolRegistrySnapshot,
}

impl ToolRouter {
    pub fn new(snapshot: ToolRegistrySnapshot) -> Self {
        Self { snapshot }
    }

    pub fn snapshot(&self) -> &ToolRegistrySnapshot {
        &self.snapshot
    }

    pub fn resolve(&self, request: ToolCallRequest) -> Result<ResolvedToolCall, AgentError> {
        let route = self
            .snapshot
            .find_by_name(&request.canonical_tool_name)
            .cloned()
            .ok_or_else(|| {
                AgentError::new(
                    AgentErrorKind::PolicyDenial,
                    RetryClassification::HostConfigurationNeeded,
                    "tool call did not resolve against runtime package tool registry snapshot",
                )
            })?;

        Ok(ResolvedToolCall { request, route })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ToolCallRequest {
    pub tool_call_id: ToolCallId,
    pub canonical_tool_name: CanonicalToolName,
    pub source: SourceRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requested_args_refs: Vec<ContentRef>,
    pub redacted_args_summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<IdempotencyKey>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dedupe_key: Option<DedupeKey>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResolvedToolCall {
    pub request: ToolCallRequest,
    pub route: ToolRoute,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ToolExecutionRequest {
    pub resolved_call: ResolvedToolCall,
    pub effect_intent: EffectIntent,
    pub strategy: ToolExecutionStrategy,
}

pub trait ToolExecutor: Send + Sync {
    fn executor_ref(&self) -> &ExecutorRef;

    fn execute(&self, request: &ToolExecutionRequest) -> Result<ToolExecutionOutput, AgentError>;
}

#[derive(Clone, Default)]
pub struct ToolExecutorRegistry {
    executors: BTreeMap<String, Arc<dyn ToolExecutor>>,
}

impl ToolExecutorRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, executor: Arc<dyn ToolExecutor>) -> Result<(), AgentError> {
        let executor_ref = executor.executor_ref().as_str().to_string();
        if executor_ref.is_empty() {
            return Err(AgentError::missing_required_field(
                "tool_executor.executor_ref",
            ));
        }
        self.executors.insert(executor_ref, executor);
        Ok(())
    }

    pub fn get(&self, executor_ref: &ExecutorRef) -> Option<Arc<dyn ToolExecutor>> {
        self.executors.get(executor_ref.as_str()).cloned()
    }

    pub fn len(&self) -> usize {
        self.executors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.executors.is_empty()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ToolExecutionOutput {
    pub terminal_status: EffectTerminalStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content_refs: Vec<ContentRef>,
    pub redacted_summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_operation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reconciliation_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_ref: Option<String>,
}

impl ToolExecutionOutput {
    pub fn completed(redacted_summary: impl Into<String>) -> Self {
        Self {
            terminal_status: EffectTerminalStatus::Completed,
            content_refs: Vec::new(),
            redacted_summary: redacted_summary.into(),
            external_operation_id: None,
            reconciliation_ref: None,
            error_ref: None,
        }
    }

    pub fn failed(redacted_summary: impl Into<String>, error_ref: impl Into<String>) -> Self {
        Self {
            terminal_status: EffectTerminalStatus::Failed,
            content_refs: Vec::new(),
            redacted_summary: redacted_summary.into(),
            external_operation_id: None,
            reconciliation_ref: None,
            error_ref: Some(error_ref.into()),
        }
    }

    pub fn to_effect_result(&self, effect_id: EffectId) -> EffectResult {
        EffectResult {
            effect_id,
            terminal_status: self.terminal_status.clone(),
            external_operation_id: self.external_operation_id.clone(),
            reconciliation_ref: self.reconciliation_ref.clone(),
            error_ref: self.error_ref.clone(),
            content_refs: self.content_refs.clone(),
            redacted_summary: self.redacted_summary.clone(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolExecutionStrategy {
    Sequential,
    BoundedConcurrent { max_in_flight: usize },
    OrderedBatch { max_in_flight: usize },
}

impl Default for ToolExecutionStrategy {
    fn default() -> Self {
        Self::Sequential
    }
}

pub trait ToolPolicyPort: Send + Sync {
    fn evaluate_pre_tool(&self, call: &ResolvedToolCall) -> Result<PolicyOutcome, AgentError>;

    fn evaluate_post_tool(
        &self,
        call: &ResolvedToolCall,
        output: &ToolExecutionOutput,
    ) -> Result<PolicyOutcome, AgentError>;
}

#[derive(Clone, Debug, Default)]
pub struct AllowToolPolicy;

impl ToolPolicyPort for AllowToolPolicy {
    fn evaluate_pre_tool(&self, call: &ResolvedToolCall) -> Result<PolicyOutcome, AgentError> {
        Ok(allowed_tool_policy_outcome(
            call.request.source.clone(),
            call.route.destination.clone(),
            call.route.policy_refs.clone(),
        ))
    }

    fn evaluate_post_tool(
        &self,
        call: &ResolvedToolCall,
        _output: &ToolExecutionOutput,
    ) -> Result<PolicyOutcome, AgentError> {
        let mut outcome = allowed_tool_policy_outcome(
            call.request.source.clone(),
            call.route.destination.clone(),
            call.route.policy_refs.clone(),
        );
        outcome.stage = PolicyStage::PostTool;
        Ok(outcome)
    }
}

pub fn allowed_tool_policy_outcome(
    source: SourceRef,
    destination: DestinationRef,
    policy_refs: Vec<PolicyRef>,
) -> PolicyOutcome {
    PolicyOutcome {
        stage: PolicyStage::PreTool,
        decision: crate::policy::PolicyDecision::allow("tool.policy.allowed"),
        subject: None,
        source: Some(source),
        destination: Some(destination),
        policy_refs,
        privacy: PrivacyClass::Internal,
        retention: RetentionClass::RunScoped,
    }
}
