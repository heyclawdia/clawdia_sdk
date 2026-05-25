//! Runtime-package records and builders. Use these items to describe the immutable
//! per-run package that freezes provider route, capabilities, policies, sidecars,
//! catalogs, and fingerprints. Builders are data-only and must not perform discovery
//! or execution side effects. This file contains the subagent portion of that
//! contract.
//!
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    capability::{CapabilityId, CapabilityKind, CapabilitySpec, PackageSidecarRef},
    domain::{AgentError, AgentId, ContentRef as ContentRefId, PolicyKind, PolicyRef},
    package::{
        AgentSnapshot, PackageSidecarSnapshot, ProviderRouteSnapshot, RuntimePackage,
        RuntimePackageFingerprint, RuntimePackageId,
    },
};

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
/// Enumerates the finite context handoff policy cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ContextHandoffPolicy {
    /// Use this variant when the contract needs to represent none; selecting it has no side effect by itself.
    #[default]
    None,
    /// Use this variant when the contract needs to represent summary only; selecting it has no side effect by itself.
    SummaryOnly {
        /// Typed summary ref reference. Resolving or executing it is a
        /// separate policy-gated step.
        summary_ref: ContentRefId,
        /// Maximum allowed tokens.
        /// Use it to keep execution, output, or diagnostics bounded.
        max_tokens: u32,
        /// Policy reference that must be resolved by the host or runtime
        /// before execution.
        policy_ref: PolicyRef,
    },
    /// Use this variant when the contract needs to represent selected refs; selecting it has no side effect by itself.
    SelectedRefs {
        /// References associated with refs.
        /// Resolve them through the appropriate registry or content store before using
        /// referenced data.
        refs: Vec<ContentRefId>,
        /// Policy reference that must be resolved by the host or runtime
        /// before execution.
        policy_ref: PolicyRef,
    },
    /// Use this variant when the contract needs to represent full history with policy; selecting it has no side effect by itself.
    FullHistoryWithPolicy {
        /// Policy reference that must be resolved by the host or runtime
        /// before execution.
        policy_ref: PolicyRef,
        /// Whether child context handoff must include provider-projection audit evidence.
        /// Use this to fail closed when subagent context would otherwise cross a policy
        /// boundary without audit proof.
        projection_audit_required: bool,
    },
}

impl ContextHandoffPolicy {
    /// Validates the package::subagent invariants and returns a typed
    /// error on failure. Validation is pure and does not perform I/O,
    /// dispatch, journal appends, or adapter calls.
    pub fn validate(&self) -> Result<(), AgentError> {
        match self {
            Self::None => Ok(()),
            Self::SummaryOnly {
                max_tokens,
                policy_ref,
                ..
            } => {
                if *max_tokens == 0 {
                    return Err(AgentError::contract_violation(
                        "summary handoff requires a positive token budget",
                    ));
                }
                validate_non_host_policy(policy_ref, "summary handoff")
            }
            Self::SelectedRefs { refs, policy_ref } => {
                if refs.is_empty() {
                    return Err(AgentError::contract_violation(
                        "selected refs handoff requires at least one content ref",
                    ));
                }
                validate_non_host_policy(policy_ref, "selected refs handoff")
            }
            Self::FullHistoryWithPolicy {
                policy_ref,
                projection_audit_required,
            } => {
                if !projection_audit_required {
                    return Err(AgentError::contract_violation(
                        "full history handoff requires a projection audit",
                    ));
                }
                validate_non_host_policy(policy_ref, "full history handoff")
            }
        }
    }

    /// Returns policy refs for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn policy_refs(&self) -> Vec<PolicyRef> {
        match self {
            Self::None => Vec::new(),
            Self::SummaryOnly { policy_ref, .. }
            | Self::SelectedRefs { policy_ref, .. }
            | Self::FullHistoryWithPolicy { policy_ref, .. } => vec![policy_ref.clone()],
        }
    }

    /// Returns selected content refs for callers that need to inspect the contract state.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn selected_content_refs(&self) -> Vec<ContentRefId> {
        match self {
            Self::None | Self::FullHistoryWithPolicy { .. } => Vec::new(),
            Self::SummaryOnly { summary_ref, .. } => vec![summary_ref.clone()],
            Self::SelectedRefs { refs, .. } => refs.clone(),
        }
    }

    /// Returns variant name for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn variant_name(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::SummaryOnly { .. } => "summary_only",
            Self::SelectedRefs { .. } => "selected_refs",
            Self::FullHistoryWithPolicy { .. } => "full_history_with_policy",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
/// Enumerates the finite route inheritance mode cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum RouteInheritanceMode {
    /// Use this variant when the contract needs to represent inherit parent; selecting it has no side effect by itself.
    InheritParent,
    /// Use this variant when the contract needs to represent explicit override only; selecting it has no side effect by itself.
    ExplicitOverrideOnly,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
/// Enumerates the finite subagent route policy cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum SubagentRoutePolicy {
    /// Use this variant when the contract needs to represent inherit parent; selecting it has no side effect by itself.
    InheritParent,
    /// Use this variant when the contract needs to represent use allowed override; selecting it has no side effect by itself.
    UseAllowedOverride {
        /// Stable route id used for typed lineage, lookup, or dedupe.
        route_id: String,
        /// Stable model id used for typed lineage, lookup, or dedupe.
        model_id: String,
    },
}

impl SubagentRoutePolicy {
    /// Returns selected route for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn selected_route(
        &self,
        parent: &ProviderRouteSnapshot,
        child_policy: &ChildRuntimePackagePolicy,
    ) -> Result<ProviderRouteSnapshot, AgentError> {
        match self {
            Self::InheritParent => Ok(parent.clone()),
            Self::UseAllowedOverride { route_id, model_id } => {
                if !child_policy.allowed_route_overrides.contains(route_id) {
                    return Err(AgentError::contract_violation(
                        "child provider route override is not allowed by package policy",
                    ));
                }
                if route_id.is_empty() || model_id.is_empty() {
                    return Err(AgentError::missing_required_field(
                        "subagent.route_override.route_id_or_model_id",
                    ));
                }
                Ok(ProviderRouteSnapshot::new(route_id, model_id))
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the child runtime package policy portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ChildRuntimePackagePolicy {
    /// Source parent package used by this record or request.
    pub source_parent_package: RuntimePackageFingerprint,
    /// Inherit provider route used by this record or request.
    pub inherit_provider_route: RouteInheritanceMode,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Allowlist for this policy or contract.
    /// Validation uses it to reject undeclared or policy-denied values.
    pub allowed_route_overrides: Vec<String>,
    /// Whether strip recursive subagents is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub strip_recursive_subagents: bool,
    /// Allowlist for this policy or contract.
    /// Validation uses it to reject undeclared or policy-denied values.
    pub strip_disallowed_tools: bool,
    /// Child lifecycle bounds used by this record or request.
    pub child_lifecycle_bounds: PolicyRef,
    /// Typed redaction policy ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub redaction_policy_ref: PolicyRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Identifiers used to select or correlate parent control tool values.
    /// Use them for typed lookup, filtering, or lineage instead of stringly typed matching.
    pub parent_control_tool_ids: Vec<CapabilityId>,
}

impl ChildRuntimePackagePolicy {
    /// Returns an updated value with strip recursive defaults configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn strip_recursive_defaults(source_parent_package: RuntimePackageFingerprint) -> Self {
        Self {
            source_parent_package,
            inherit_provider_route: RouteInheritanceMode::InheritParent,
            allowed_route_overrides: Vec::new(),
            strip_recursive_subagents: true,
            strip_disallowed_tools: true,
            child_lifecycle_bounds: PolicyRef::with_kind(
                PolicyKind::RuntimePackage,
                "policy.child.parent_owned",
            ),
            redaction_policy_ref: PolicyRef::with_kind(
                PolicyKind::Redaction,
                "policy.redaction.subagent.default",
            ),
            parent_control_tool_ids: default_parent_control_tool_ids(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
/// Enumerates the finite subagent tool policy cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum SubagentToolPolicy {
    /// Use this variant when the contract needs to represent inherit allowlist; selecting it has no side effect by itself.
    InheritAllowlist,
    /// Use this variant when the contract needs to represent read only; selecting it has no side effect by itself.
    #[default]
    ReadOnly,
    /// Use this variant when the contract needs to represent no tools; selecting it has no side effect by itself.
    NoTools,
    /// Use this variant when the contract needs to represent custom allowlist; selecting it has no side effect by itself.
    CustomAllowlist {
        /// Capability identifiers affected by this package or sidecar record.
        capability_ids: Vec<CapabilityId>,
    },
}

impl SubagentToolPolicy {
    fn retains(&self, capability: &CapabilitySpec) -> bool {
        match self {
            Self::InheritAllowlist | Self::ReadOnly => true,
            Self::NoTools => capability.kind != CapabilityKind::Tool,
            Self::CustomAllowlist { capability_ids } => {
                capability.kind != CapabilityKind::Tool
                    || capability_ids.contains(&capability.capability_id)
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the depth budget portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct DepthBudget {
    /// Current depth used by this record or request.
    pub current_depth: u32,
    /// Maximum allowed depth.
    /// Use it to keep execution, output, or diagnostics bounded.
    pub max_depth: u32,
    /// Maximum allowed children.
    /// Use it to keep execution, output, or diagnostics bounded.
    pub max_children: u32,
}

impl DepthBudget {
    /// Returns max depth for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn max_depth(max_depth: u32) -> Self {
        Self {
            current_depth: 0,
            max_depth,
            max_children: 1,
        }
    }

    /// Validates the package::subagent invariants and returns a typed
    /// error on failure. Validation is pure and does not perform I/O,
    /// dispatch, journal appends, or adapter calls.
    pub fn validate_child_start(&self) -> Result<(), AgentError> {
        if self.max_depth == 0 || self.current_depth >= self.max_depth {
            return Err(AgentError::contract_violation(
                "subagent depth budget exhausted before child start",
            ));
        }
        if self.max_children == 0 {
            return Err(AgentError::contract_violation(
                "subagent child count budget exhausted before child start",
            ));
        }
        Ok(())
    }

    /// Returns child budget for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn child_budget(&self) -> Self {
        Self {
            current_depth: self.current_depth + 1,
            max_depth: self.max_depth,
            max_children: self.max_children,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the child runtime package portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ChildRuntimePackage {
    /// Package used by this record or request.
    pub package: RuntimePackage,
    /// Deterministic fingerprint for package, event, telemetry, or validation
    /// evidence.
    pub fingerprint: RuntimePackageFingerprint,
    /// Strip manifest used by this record or request.
    pub strip_manifest: ChildPackageStripManifest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the child package strip manifest portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct ChildPackageStripManifest {
    /// Deterministic parent package fingerprint used for stale checks,
    /// package evidence, or replay comparisons.
    pub parent_package_fingerprint: RuntimePackageFingerprint,
    /// Stable child agent id used for typed lineage, lookup, or dedupe.
    pub child_agent_id: AgentId,
    /// Stable selected provider route id used for typed lineage, lookup, or
    /// dedupe.
    pub selected_provider_route_id: String,
    /// Handoff policy variant used by this record or request.
    pub handoff_policy_variant: String,
    /// Tool policy used by this record or request.
    pub tool_policy: SubagentToolPolicy,
    /// Whether recursive subagent strip is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub recursive_subagent_strip: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Identifiers used to select or correlate stripped capability values.
    /// Use them for typed lookup, filtering, or lineage instead of stringly typed matching.
    pub stripped_capability_ids: Vec<CapabilityId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Identifiers used to select or correlate retained capability values.
    /// Use them for typed lookup, filtering, or lineage instead of stringly typed matching.
    pub retained_capability_ids: Vec<CapabilityId>,
    /// Typed lifecycle policy ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub lifecycle_policy_ref: PolicyRef,
    /// Typed redaction policy ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub redaction_policy_ref: PolicyRef,
}

impl ChildPackageStripManifest {
    /// Computes the stable content hash for this package::subagent
    /// value. The computation is deterministic and side-effect free so
    /// it can be used in package, journal, or test evidence.
    pub fn content_hash(&self) -> Result<String, AgentError> {
        let bytes = serde_json::to_vec(self).map_err(|error| {
            AgentError::contract_violation(format!(
                "child package strip manifest serialization failed: {error}"
            ))
        })?;
        Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
    }
}

/// Builds the build child runtime package value.
/// This is data construction and performs no I/O, journal append, event publication, or process
pub fn build_child_runtime_package(
    parent: &RuntimePackage,
    child_agent_id: AgentId,
    route_policy: &SubagentRoutePolicy,
    handoff_policy: &ContextHandoffPolicy,
    child_policy: &ChildRuntimePackagePolicy,
    tool_policy: &SubagentToolPolicy,
) -> Result<ChildRuntimePackage, AgentError> {
    parent.validate()?;
    handoff_policy.validate()?;

    if !child_policy.strip_recursive_subagents {
        return Err(AgentError::contract_violation(
            "recursive subagent tools are denied by the core SDK contract",
        ));
    }

    let parent_fingerprint = parent.fingerprint()?;
    if parent_fingerprint != child_policy.source_parent_package {
        return Err(AgentError::contract_violation(
            "child package policy source parent fingerprint does not match parent package",
        ));
    }

    let selected_route = route_policy.selected_route(&parent.provider_route, child_policy)?;
    let mut child = parent.clone();
    child.package_id = child_package_id(&child_agent_id, &parent_fingerprint);
    child.agent = AgentSnapshot {
        agent_id: child_agent_id.clone(),
        name: child_agent_id.as_str().to_string(),
        default_behavior_refs: parent.agent.default_behavior_refs.clone(),
    };
    child.provider_route = selected_route.clone();
    child.child_lifecycle.policy_ref = child_policy.child_lifecycle_bounds.clone();
    child.child_lifecycle.detach_policy_ref = child_policy.child_lifecycle_bounds.clone();

    let mut stripped = Vec::new();
    let mut retained = Vec::new();
    child.capabilities.retain(|capability| {
        let strip = is_recursive_subagent_capability(capability, child_policy)
            || (child_policy.strip_disallowed_tools && !tool_policy.retains(capability));
        if strip {
            stripped.push(capability.capability_id.clone());
            false
        } else {
            retained.push(capability.capability_id.clone());
            true
        }
    });

    stripped.sort();
    retained.sort();
    let manifest = ChildPackageStripManifest {
        parent_package_fingerprint: parent_fingerprint,
        child_agent_id,
        selected_provider_route_id: selected_route.route_id.clone(),
        handoff_policy_variant: handoff_policy.variant_name().to_string(),
        tool_policy: tool_policy.clone(),
        recursive_subagent_strip: true,
        stripped_capability_ids: stripped,
        retained_capability_ids: retained,
        lifecycle_policy_ref: child_policy.child_lifecycle_bounds.clone(),
        redaction_policy_ref: child_policy.redaction_policy_ref.clone(),
    };

    child.sidecars.push(PackageSidecarSnapshot {
        sidecar_id: "sidecar.subagent.child_package_strip_manifest".to_string(),
        kind: "subagent_child_package_strip_manifest".to_string(),
        version: "v1".to_string(),
        refs: vec![PackageSidecarRef::new(
            "sidecar.subagent.child_package_strip_manifest",
            "subagent_child_package_strip_manifest",
            "v1",
        )],
        policy_refs: vec![
            child_policy.child_lifecycle_bounds.clone(),
            child_policy.redaction_policy_ref.clone(),
        ],
        content_hash: manifest.content_hash()?,
    });
    child
        .policies
        .policy_refs
        .push(child_policy.redaction_policy_ref.clone());
    child.fingerprint_manifest = child.computed_fingerprint_manifest();
    child.validate()?;
    let fingerprint = child.fingerprint()?;

    Ok(ChildRuntimePackage {
        package: child,
        fingerprint,
        strip_manifest: manifest,
    })
}

fn is_recursive_subagent_capability(
    capability: &CapabilitySpec,
    policy: &ChildRuntimePackagePolicy,
) -> bool {
    capability.kind == CapabilityKind::AgentAsTool
        || policy
            .parent_control_tool_ids
            .contains(&capability.capability_id)
}

fn default_parent_control_tool_ids() -> Vec<CapabilityId> {
    [
        "tool.subagent_send_message",
        "tool.subagent_reply_to_clarification",
        "tool.subagent_ask_parent",
        "tool.subagent_read_parent_messages",
        "tool.subagent_monitor",
    ]
    .into_iter()
    .map(CapabilityId::new)
    .collect()
}

fn child_package_id(
    child_agent_id: &AgentId,
    parent_fingerprint: &RuntimePackageFingerprint,
) -> RuntimePackageId {
    let digest = Sha256::digest(parent_fingerprint.as_str().as_bytes());
    let suffix = format!("{:x}", digest);
    RuntimePackageId::new(format!(
        "package.subagent.{}.{}",
        child_agent_id.as_str(),
        &suffix[..16]
    ))
}

fn validate_non_host_policy(policy_ref: &PolicyRef, label: &str) -> Result<(), AgentError> {
    if policy_ref.kind == PolicyKind::Host {
        Err(AgentError::contract_violation(format!(
            "{label} requires an explicit SDK policy ref"
        )))
    } else {
        Ok(())
    }
}
