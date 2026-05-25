//! Tool routing, execution, and policy ports. Use this module when runtime-package
//! capabilities become executable tool calls. Executor implementations may perform
//! side effects and must return effect-compatible output records.
//!
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
/// Carries tool registry snapshot data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ToolRegistrySnapshot {
    /// Fingerprint of the runtime package snapshot in force when this value was produced.
    /// Use it for replay, dedupe, and package-lineage checks; the field is evidence and does
    /// not execute package behavior.
    pub runtime_package_fingerprint: String,
    /// Collection of routes values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub routes: Vec<ToolRoute>,
}

impl ToolRegistrySnapshot {
    /// Constructs this value from runtime package. Use it when adapting
    /// canonical SDK records without introducing a second behavior
    /// path.
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

    /// Reads the stored find by name without registry or runtime work.
    /// This reads tool registry metadata and does not execute a tool.
    pub fn find_by_name(&self, name: &CanonicalToolName) -> Option<&ToolRoute> {
        self.routes
            .iter()
            .find(|route| &route.canonical_tool_name == name)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries tool route data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ToolRoute {
    /// Stable capability identifier used for package projection and
    /// executable routing.
    pub capability_id: CapabilityId,
    /// Canonical tool name used by this record or request.
    pub canonical_tool_name: CanonicalToolName,
    /// Namespace that groups this capability or identifier.
    /// Use it to avoid collisions between packages, hosts, and extensions.
    pub namespace: CapabilityNamespace,
    /// Source label or ref for this item; it is metadata and does not fetch
    /// content by itself.
    pub source: SourceRef,
    /// Destination label or ref for this item; it is metadata and does not
    /// deliver content by itself.
    pub destination: DestinationRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Typed executor ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub executor_ref: Option<ExecutorRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Policy references that govern admission, projection, execution, or
    /// delivery.
    pub policy_refs: Vec<PolicyRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// References to typed package sidecars needed by this capability.
    pub sidecar_refs: Vec<PackageSidecarRef>,
    /// Classification value for effect class.
    /// Policy and projection paths use it for finite routing decisions.
    pub effect_class: EffectClass,
    /// Risk classification for the operation or capability.
    /// Policy uses it to decide whether approval, sandboxing, or denial is required.
    pub risk_class: RiskClass,
    /// Privacy class used for projection, telemetry, and raw-content access
    /// decisions.
    pub privacy: PrivacyClass,
    /// Retention class used by hosts and sinks when storing or exporting this
    /// item.
    pub retention: RetentionClass,
}

impl ToolRoute {
    /// Validates the ports::tool invariants and returns a typed error
    /// on failure. Validation is pure and does not perform I/O,
    /// dispatch, journal appends, or adapter calls.
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
/// Carries tool router data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ToolRouter {
    snapshot: ToolRegistrySnapshot,
}

impl ToolRouter {
    /// Creates a new ports::tool value with explicit caller-provided
    /// inputs. This constructor is data-only and performs no I/O or
    /// external side effects.
    pub fn new(snapshot: ToolRegistrySnapshot) -> Self {
        Self { snapshot }
    }

    /// Snapshot.
    /// This reads tool registry metadata and does not execute a tool.
    pub fn snapshot(&self) -> &ToolRegistrySnapshot {
        &self.snapshot
    }

    /// Resolves resolve through the configured ports::tool boundary. Concrete
    /// implementations own any backing-store, filesystem, or network side
    /// effects.
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
/// Carries tool call request data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ToolCallRequest {
    /// Stable tool call id used for typed lineage, lookup, or dedupe.
    pub tool_call_id: ToolCallId,
    /// Canonical tool name used by this record or request.
    pub canonical_tool_name: CanonicalToolName,
    /// Source label or ref for this item; it is metadata and does not fetch
    /// content by itself.
    pub source: SourceRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Typed requested args refs references. Resolving them is separate from
    /// constructing this record.
    pub requested_args_refs: Vec<ContentRef>,
    /// Redacted summary for display, logs, events, or telemetry.
    /// It should describe the value without exposing raw private content.
    pub redacted_args_summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Idempotency setting or key for deduping retries.
    /// Use it to prevent duplicate side effects during replay or repair.
    pub idempotency_key: Option<IdempotencyKey>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Dedupe policy or key for a side-effecting operation.
    /// Replay and repair use it to avoid sending or executing the same effect twice.
    pub dedupe_key: Option<DedupeKey>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries resolved tool call data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ResolvedToolCall {
    /// Request DTO or resolved call that triggered this operation.
    pub request: ToolCallRequest,
    /// Route used by this record or request.
    pub route: ToolRoute,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries tool execution request data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ToolExecutionRequest {
    /// Resolved call used by this record or request.
    pub resolved_call: ResolvedToolCall,
    /// Effect intent used by this record or request.
    pub effect_intent: EffectIntent,
    /// Strategy used by this record or request.
    pub strategy: ToolExecutionStrategy,
}

/// Port or behavior contract for tool executor. Implementors should
/// preserve policy, redaction, idempotency, and replay expectations
/// from the surrounding module. Implementations may perform side
/// effects only as described by the trait methods.
pub trait ToolExecutor: Send + Sync {
    /// Returns executor ref for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    fn executor_ref(&self) -> &ExecutorRef;

    /// Executes one policy-approved tool request through this executor.
    /// Implementations may run host code or external adapters, but the runtime
    /// owns intent/result journaling and approval checks around this call.
    fn execute(&self, request: &ToolExecutionRequest) -> Result<ToolExecutionOutput, AgentError>;
}

#[derive(Clone, Default)]
/// Carries tool executor registry data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ToolExecutorRegistry {
    executors: BTreeMap<String, Arc<dyn ToolExecutor>>,
}

impl ToolExecutorRegistry {
    /// Creates a new ports::tool value with explicit caller-provided
    /// inputs. This constructor is data-only and performs no I/O or
    /// external side effects.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds data to this in-memory ports::tool collection. It does not
    /// perform external I/O, execute tools, or append journals.
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

    /// Looks up an entry in this local store without registry or runtime work.
    /// This reads tool registry metadata and does not execute a tool.
    pub fn get(&self, executor_ref: &ExecutorRef) -> Option<Arc<dyn ToolExecutor>> {
        self.executors.get(executor_ref.as_str()).cloned()
    }

    /// Reads the stored len without registry or runtime work.
    /// This reads tool registry metadata and does not execute a tool.
    pub fn len(&self) -> usize {
        self.executors.len()
    }

    /// Reports whether this value is empty. The check is pure and does
    /// not mutate SDK or host state.
    pub fn is_empty(&self) -> bool {
        self.executors.is_empty()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries tool execution output data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ToolExecutionOutput {
    /// Terminal status used by this record or request.
    pub terminal_status: EffectTerminalStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Content references associated with this record; resolving them is a
    /// separate policy-gated step.
    pub content_refs: Vec<ContentRef>,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stable external operation id used for typed lineage, lookup, or
    /// dedupe.
    pub external_operation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Typed reconciliation ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub reconciliation_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Typed error ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub error_ref: Option<String>,
}

impl ToolExecutionOutput {
    /// Returns an updated value with completed configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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

    /// Returns an updated value with failed configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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

    /// Converts this value into effect result data.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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
/// Enumerates the finite tool execution strategy cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ToolExecutionStrategy {
    /// Use this variant when the contract needs to represent sequential; selecting it has no side effect by itself.
    Sequential,
    /// Use this variant when the contract needs to represent bounded concurrent; selecting it has no side effect by itself.
    BoundedConcurrent {
        /// Maximum concurrent operations allowed by this strategy.
        max_in_flight: usize,
    },
    /// Use this variant when the contract needs to represent ordered batch; selecting it has no side effect by itself.
    OrderedBatch {
        /// Maximum concurrent operations allowed by this strategy.
        max_in_flight: usize,
    },
}

impl Default for ToolExecutionStrategy {
    fn default() -> Self {
        Self::Sequential
    }
}

/// Port or behavior contract for tool policy port. Implementors should
/// preserve policy, redaction, idempotency, and replay expectations
/// from the surrounding module. Implementations may perform side
/// effects only as described by the trait methods.
pub trait ToolPolicyPort: Send + Sync {
    /// Returns evaluate pre tool for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    fn evaluate_pre_tool(&self, call: &ResolvedToolCall) -> Result<PolicyOutcome, AgentError>;

    /// Returns evaluate post tool for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    fn evaluate_post_tool(
        &self,
        call: &ResolvedToolCall,
        output: &ToolExecutionOutput,
    ) -> Result<PolicyOutcome, AgentError>;
}

#[derive(Clone, Debug, Default)]
/// Carries allow tool policy data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
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

/// Computes or returns allowed tool policy outcome for the ports::tool
/// contract without external I/O or side effects.
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
