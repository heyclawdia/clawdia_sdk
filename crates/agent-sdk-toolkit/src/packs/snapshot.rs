use agent_sdk_core::{
    CapabilityId, CapabilityNamespace, ExecutorRef, PolicyRef, PrivacyClass, SourceRef,
    ToolPackToolSnapshot,
    policy::{CapabilityPermission, EffectClass, RiskClass},
    tool_records::CanonicalToolName,
};

pub fn tool_snapshot(
    capability_id: &str,
    tool_name: &str,
    executor_ref: &str,
    schema_id: &str,
    policy_refs: Vec<PolicyRef>,
    required_permissions: Vec<CapabilityPermission>,
    effect_class: EffectClass,
    risk_class: RiskClass,
    _source: &SourceRef,
) -> ToolPackToolSnapshot {
    ToolPackToolSnapshot {
        capability_id: CapabilityId::new(capability_id),
        canonical_tool_name: CanonicalToolName::new(tool_name),
        namespace: CapabilityNamespace::new(format!("tool.{tool_name}")),
        schema_ref: agent_sdk_core::PackageSidecarRef::new(schema_id, "tool_schema", "v1"),
        executor_ref: ExecutorRef::new(executor_ref),
        policy_refs,
        required_permissions,
        effect_class,
        risk_class,
        redaction_policy_ref: PolicyRef::with_kind(
            agent_sdk_core::PolicyKind::Redaction,
            "policy.redaction.tool_result.refs_only",
        ),
        timeout_ms: 10_000,
        cancellation: "best_effort".to_string(),
        reconciliation: "effect_lineage_required".to_string(),
        privacy: PrivacyClass::ContentRefsOnly,
    }
}
