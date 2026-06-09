use agent_sdk_core::{
    AgentId, CapabilityId, DestinationKind, DestinationRef, EffectId, EffectKind, EntityKind,
    EntityRef, ExecutorRef, PackageSidecarRef, PolicyKind, PolicyRef, PrivacyClass,
    ProviderRouteSnapshot, ResourceReadRequest, ResourceRouter, ResourceScheme, RuntimePackage,
    RuntimePackageId, SourceKind, SourceRef, ToolPackEffectLineage, ToolPackId, ToolPackKind,
    ToolPackSnapshot, ToolPackToolSnapshot, WorkspaceMutationLineage,
    domain::ContentRef as ContentRefId,
    policy::{CapabilityPermission, EffectClass, RiskClass},
    tool_records::CanonicalToolName,
};

#[test]
fn runtime_package_accepts_external_tool_pack_contract_without_core_tool_impl() {
    let snapshot = external_workspace_read_pack("toolpack.workspace.readonly.v1");
    let capabilities = snapshot
        .capability_specs()
        .expect("tool pack lowers to capabilities");
    let sidecar = snapshot
        .package_sidecar_snapshot()
        .expect("sidecar snapshot");
    let package = package_builder("package.toolpack.boundary")
        .sidecar(sidecar)
        .capability(capabilities[0].clone())
        .build()
        .expect("core package accepts external tool pack sidecar");

    assert_eq!(package.provider_tool_specs().unwrap().len(), 1);
    assert_eq!(package.executable_routes().unwrap().len(), 1);
    assert_eq!(
        package
            .sidecar("toolpack.workspace.readonly.v1")
            .unwrap()
            .kind,
        "tool_pack"
    );

    for source in [
        include_str!("../../src/package/tool_pack.rs"),
        include_str!("../../src/records/tool_pack.rs"),
        include_str!("../../src/ports/tool_pack.rs"),
    ] {
        for forbidden in ["std::fs", "std::process", "Command::new", "walkdir"] {
            assert!(
                !source.contains(forbidden),
                "{forbidden} belongs in optional toolkit crates, not agent-sdk-core"
            );
        }
    }
}

#[test]
fn tool_pack_fingerprint_changes_when_executor_policy_or_sidecar_changes() {
    let baseline = package_for_pack(external_workspace_read_pack(
        "toolpack.workspace.readonly.v1",
    ))
    .fingerprint()
    .expect("baseline fingerprint");

    let mut executor_changed = external_workspace_read_pack("toolpack.workspace.readonly.v1");
    executor_changed.tools[0].executor_ref = ExecutorRef::new("executor.external.read.v2");
    let executor_fingerprint = package_for_pack(executor_changed)
        .fingerprint()
        .expect("executor fingerprint");
    assert_ne!(baseline, executor_fingerprint);

    let mut policy_changed = external_workspace_read_pack("toolpack.workspace.readonly.v1");
    policy_changed.tools[0].policy_refs = vec![PolicyRef::with_kind(
        PolicyKind::Approval,
        "policy.approval.workspace_read.strict",
    )];
    let policy_fingerprint = package_for_pack(policy_changed)
        .fingerprint()
        .expect("policy fingerprint");
    assert_ne!(baseline, policy_fingerprint);

    let sidecar_changed = package_for_pack(external_workspace_read_pack(
        "toolpack.workspace.readonly.v2",
    ))
    .fingerprint()
    .expect("sidecar fingerprint");
    assert_ne!(baseline, sidecar_changed);
}

#[test]
fn resource_uri_router_is_scheme_scoped_and_fails_closed_without_resolver() {
    let mut router = ResourceRouter::new();
    router.register_static(
        ResourceScheme::new("memory"),
        ContentRefId::new("content.memory.summary"),
        source("source.host.memory"),
        policy("policy.permission.memory_read"),
    );

    let resolved = router
        .resolve(&ResourceReadRequest {
            uri: "memory://summary/1".to_string(),
            source: source("source.model.tool_call"),
            policy_refs: vec![policy("policy.permission.memory_read")],
            max_bytes: 4096,
        })
        .expect("memory URI resolves through registered resolver");
    assert_eq!(
        resolved.content_ref,
        ContentRefId::new("content.memory.summary")
    );
    assert_eq!(resolved.scheme.as_str(), "memory");

    let error = router
        .resolve(&ResourceReadRequest {
            uri: "artifact://missing".to_string(),
            source: source("source.model.tool_call"),
            policy_refs: vec![policy("policy.permission.artifact_read")],
            max_bytes: 4096,
        })
        .expect_err("unregistered scheme fails closed");
    assert_eq!(error.kind(), agent_sdk_core::AgentErrorKind::PolicyDenial);
}

#[test]
fn tool_pack_lineage_records_effect_metadata_without_undo_promise() {
    let lineage = ToolPackEffectLineage {
        pack_id: ToolPackId::new("toolpack.workspace.edit.v1"),
        tool_name: CanonicalToolName::new("workspace_edit_apply"),
        effect_id: EffectId::new("effect.file.write.1"),
        effect_kind: EffectKind::FileWrite,
        subject_ref: EntityRef::new(EntityKind::ToolCall, "tool.call.edit.1"),
        source: source("source.sdk.toolkit"),
        destination: DestinationRef::with_kind(DestinationKind::Tool, "destination.tool.edit"),
        policy_refs: vec![policy("policy.approval.workspace_write")],
        content_refs: vec![ContentRefId::new("content.diff.preview")],
        mutation: Some(WorkspaceMutationLineage {
            path: "docs/notes.md".to_string(),
            before_hash: Some("sha256:before".to_string()),
            after_hash: Some("sha256:after".to_string()),
            diff_ref: Some(ContentRefId::new("content.diff.preview")),
            inverse_candidate_ref: Some(ContentRefId::new("content.inverse.patch")),
            non_reversible_reason: None,
        }),
        redacted_summary: "anchored edit applied with before/after hashes".to_string(),
    };

    assert!(lineage.inverse_candidate_is_advisory());
    assert_eq!(lineage.effect_kind, EffectKind::FileWrite);
    assert_eq!(
        lineage.policy_refs[0].as_str(),
        "policy.approval.workspace_write"
    );
}

fn package_for_pack(snapshot: ToolPackSnapshot) -> RuntimePackage {
    let capabilities = snapshot.capability_specs().expect("capabilities lower");
    package_builder("package.toolpack.fingerprint")
        .sidecar(snapshot.package_sidecar_snapshot().expect("sidecar"))
        .capability(capabilities[0].clone())
        .build()
        .expect("package builds")
}

fn package_builder(package_id: &str) -> agent_sdk_core::RuntimePackageBuilder {
    RuntimePackage::builder(RuntimePackageId::new(package_id))
        .agent(agent_sdk_core::AgentSnapshot {
            agent_id: AgentId::new("agent.toolpack.boundary"),
            name: "tool pack boundary".to_string(),
            default_behavior_refs: Vec::new(),
        })
        .provider_route(ProviderRouteSnapshot::new("provider.fake", "model.fake"))
}

fn external_workspace_read_pack(sidecar_id: &str) -> ToolPackSnapshot {
    ToolPackSnapshot::new(
        ToolPackId::new(sidecar_id),
        ToolPackKind::WorkspaceReadOnly,
        "v1",
        source("source.external.toolpack"),
    )
    .with_trust(agent_sdk_core::TrustClass::Trusted)
    .with_tool(ToolPackToolSnapshot {
        capability_id: CapabilityId::new("cap.external.workspace_read"),
        canonical_tool_name: CanonicalToolName::new("workspace_read"),
        namespace: agent_sdk_core::CapabilityNamespace::new("tool.workspace_read"),
        description: None,
        schema_ref: PackageSidecarRef::new("schema.workspace_read.v1", "tool_schema", "v1"),
        redacted_schema: None,
        executor_ref: ExecutorRef::new("executor.external.read.v1"),
        policy_refs: vec![policy("policy.approval.workspace_read")],
        requires_approval: false,
        required_permissions: vec![CapabilityPermission::FilesystemRead],
        effect_class: EffectClass::Read,
        risk_class: RiskClass::Low,
        redaction_policy_ref: policy("policy.redaction.tool_result"),
        timeout_ms: 1_000,
        cancellation: "best_effort".to_string(),
        reconciliation: "read_hash_and_bounds".to_string(),
        privacy: PrivacyClass::ContentRefsOnly,
    })
}

fn source(id: &str) -> SourceRef {
    SourceRef::with_kind(SourceKind::Sdk, id)
}

fn policy(id: &str) -> PolicyRef {
    let kind = if id.contains("approval") {
        PolicyKind::Approval
    } else if id.contains("redaction") {
        PolicyKind::Redaction
    } else {
        PolicyKind::Permission
    };
    PolicyRef::with_kind(kind, id)
}
