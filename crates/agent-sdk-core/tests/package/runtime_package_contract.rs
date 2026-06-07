use agent_sdk_core::{
    AgentId, CapabilityCatalogSnapshot, CapabilityId, CapabilityKind, CapabilitySourceKind,
    CapabilitySpec, CapabilityVisibility, ExecutorRef, PackageDelta, PackageSidecarRef,
    PackageSidecarSnapshot, PolicyKind, PolicyRef, ProjectionMode, ProviderRouteSnapshot,
    RuntimePackage, RuntimePackageConformanceReport, RuntimePackageId, SourceKind, SourceRef,
    TrustClass, VolatileRuntimeFields,
};
use serde_json::{Value, json};

fn fixture(path: &str) -> Value {
    serde_json::from_str(path).expect("fixture parses")
}

fn package_policy(id: &str) -> PolicyRef {
    PolicyRef::with_kind(PolicyKind::RuntimePackage, id)
}

fn approval_policy(id: &str) -> PolicyRef {
    PolicyRef::with_kind(PolicyKind::Approval, id)
}

fn source(id: &str) -> SourceRef {
    SourceRef::with_kind(SourceKind::Sdk, id)
}

fn schema_ref(version: &str) -> PackageSidecarRef {
    let mut sidecar =
        PackageSidecarRef::new("sidecar.schema.workspace_read", "tool_schema", version);
    sidecar.content_hash = Some(format!("sha256:schema.{version}"));
    sidecar
}

fn named_sidecar_ref(sidecar_id: &str) -> PackageSidecarRef {
    let mut sidecar = PackageSidecarRef::new(sidecar_id, "tool_schema", "v1");
    sidecar.content_hash = Some(format!("sha256:{sidecar_id}"));
    sidecar
}

fn workspace_read_tool(schema_version: &str, executor_ref: &str) -> CapabilitySpec {
    CapabilitySpec::fake_tool(
        "cap.workspace_read",
        "workspace_read",
        schema_ref(schema_version),
        ExecutorRef::new(executor_ref),
        approval_policy("policy.approval.workspace_read"),
        source("source.sdk.toolpack"),
    )
}

fn package_with_tool() -> RuntimePackage {
    RuntimePackage::builder(RuntimePackageId::new("package.contract"))
        .agent(agent_sdk_core::AgentSnapshot {
            agent_id: AgentId::new("agent.contract"),
            name: "contract agent".to_string(),
            default_behavior_refs: vec![package_policy("policy.agent.default")],
        })
        .provider_route(ProviderRouteSnapshot::new("provider.fake", "model.fake.p0"))
        .sidecar(PackageSidecarSnapshot {
            sidecar_id: "sidecar.schema.workspace_read".to_string(),
            kind: "tool_schema".to_string(),
            version: "v1".to_string(),
            refs: vec![schema_ref("v1")],
            policy_refs: vec![approval_policy("policy.approval.workspace_read")],
            content_hash: "sha256:schema.v1".to_string(),
            redacted_payload: None,
        })
        .capability(workspace_read_tool("v1", "executor.workspace_read.v1"))
        .policy(package_policy("policy.package.default"))
        .build()
        .expect("package builds")
}

#[test]
fn runtime_package_fingerprint_is_deterministic_and_fixture_backed() {
    let package = package_with_tool();
    let first = package.fingerprint().expect("fingerprint");
    let second = package.fingerprint().expect("fingerprint");
    assert_eq!(first, second);
    assert!(
        first
            .as_str()
            .starts_with("sha256:runtime-package-canonical-v1:")
    );

    let snapshot = serde_json::to_value(package.canonical_snapshot().expect("snapshot")).unwrap();
    let expected_snapshot = fixture(include_str!(
        "../fixtures/package/runtime-package-canonical-v1.json"
    ));
    assert_eq!(
        agent_sdk_core::testing::normalize_json_value(snapshot),
        agent_sdk_core::testing::normalize_json_value(expected_snapshot)
    );

    let fingerprint_fixture = fixture(include_str!(
        "../fixtures/package/runtime-package-fingerprint.json"
    ));
    assert_eq!(
        json!({
            "schema_version": 1,
            "fingerprint": first.as_str(),
        }),
        fingerprint_fixture
    );
}

#[test]
fn execution_affecting_changes_update_fingerprint_but_volatile_fields_do_not() {
    let package = package_with_tool();
    let baseline = package.fingerprint().expect("fingerprint");

    let schema_changed = RuntimePackage::builder(RuntimePackageId::new("package.contract"))
        .agent(package.agent.clone())
        .provider_route(package.provider_route.clone())
        .capability(workspace_read_tool("v2", "executor.workspace_read.v1"))
        .policy(package_policy("policy.package.default"))
        .build()
        .expect("schema package builds")
        .fingerprint()
        .expect("schema fingerprint");
    assert_ne!(baseline, schema_changed);

    let route_changed = RuntimePackage::builder(RuntimePackageId::new("package.contract"))
        .agent(package.agent.clone())
        .provider_route(ProviderRouteSnapshot::new(
            "provider.fake",
            "model.fake.other",
        ))
        .capability(workspace_read_tool("v1", "executor.workspace_read.v2"))
        .policy(package_policy("policy.package.default"))
        .build()
        .expect("route package builds")
        .fingerprint()
        .expect("route fingerprint");
    assert_ne!(baseline, route_changed);

    let mut volatile_changed = package.clone();
    volatile_changed.volatile = VolatileRuntimeFields {
        run_id: Some("run.dynamic".to_string()),
        event_id: Some("event.dynamic".to_string()),
        timestamp_ms: Some(123),
        adapter_health: Some("healthy-now".to_string()),
        temporary_path: Some("/tmp/dynamic".to_string()),
    };
    assert_eq!(
        baseline,
        volatile_changed
            .fingerprint()
            .expect("volatile fingerprint")
    );
}

#[test]
fn runtime_package_fingerprint_canonicalizes_nested_execution_lists() {
    let mut first = package_with_tool();
    let mut second = package_with_tool();

    first.capabilities[0].sidecar_refs = vec![
        named_sidecar_ref("sidecar.z"),
        named_sidecar_ref("sidecar.a"),
    ];
    second.capabilities[0].sidecar_refs = vec![
        named_sidecar_ref("sidecar.a"),
        named_sidecar_ref("sidecar.z"),
    ];

    first.sidecars[0].refs = vec![
        named_sidecar_ref("sidecar.ref.z"),
        named_sidecar_ref("sidecar.ref.a"),
    ];
    second.sidecars[0].refs = vec![
        named_sidecar_ref("sidecar.ref.a"),
        named_sidecar_ref("sidecar.ref.z"),
    ];

    first.sidecars[0].policy_refs = vec![
        approval_policy("policy.approval.z"),
        approval_policy("policy.approval.a"),
    ];
    second.sidecars[0].policy_refs = vec![
        approval_policy("policy.approval.a"),
        approval_policy("policy.approval.z"),
    ];

    let mut first_catalog = CapabilityCatalogSnapshot {
        catalog_id: "catalog.discovery.1".to_string(),
        source_kind: CapabilitySourceKind::DiscoveryIndex,
        source_ref: source("source.sdk.discovery"),
        version: Some("v1".to_string()),
        content_hash: Some("sha256:catalog.discovery.1".to_string()),
        trust_state: TrustClass::SdkGenerated,
        activation_policy_ref: package_policy("policy.activation.discovery"),
        candidates: vec![CapabilityId::new("cap.z"), CapabilityId::new("cap.a")],
    };
    let mut second_catalog = first_catalog.clone();
    second_catalog.candidates.reverse();

    first.catalogs = vec![first_catalog.clone()];
    second.catalogs = vec![second_catalog.clone()];

    assert_eq!(
        first.fingerprint().expect("first fingerprint"),
        second.fingerprint().expect("second fingerprint")
    );

    first_catalog
        .candidates
        .sort_by_key(|id| id.as_str().to_string());
    assert_eq!(
        first
            .canonical_snapshot()
            .expect("canonical snapshot")
            .catalogs[0]
            .candidates,
        first_catalog.candidates
    );
}

#[test]
fn projection_and_execution_routes_derive_from_one_package_authority() {
    let package = package_with_tool();
    let projected = package.provider_tool_specs().expect("provider specs");
    let executable = package.executable_routes().expect("executable routes");

    assert_eq!(projected.len(), 1);
    assert_eq!(executable.len(), 1);
    assert_eq!(projected[0].capability_id, executable[0].capability_id);
    assert_eq!(projected[0].policy_ref, executable[0].policy_ref);

    let mut broken = package.clone();
    broken.capabilities[0].executor_ref = None;
    let error = broken
        .validate()
        .expect_err("projected tool without executor fails");
    assert_eq!(error.kind(), agent_sdk_core::AgentErrorKind::InvalidPackage);
}

#[test]
fn reserved_variants_are_inactive_and_cannot_project_or_execute() {
    let inactive = CapabilitySpec::reserved_inactive(
        "cap.stream.reserved",
        CapabilityKind::StreamControl,
        package_policy("policy.stream.reserved"),
        source("source.sdk.reserved"),
    );
    let package = RuntimePackage::builder(RuntimePackageId::new("package.reserved"))
        .agent(agent_sdk_core::AgentSnapshot {
            agent_id: AgentId::new("agent.reserved"),
            name: "reserved agent".to_string(),
            default_behavior_refs: Vec::new(),
        })
        .provider_route(ProviderRouteSnapshot::new("provider.fake", "model.fake.p0"))
        .capability(inactive)
        .build()
        .expect("inactive reserved package builds");

    assert_eq!(package.provider_tool_specs().unwrap().len(), 0);
    assert_eq!(package.executable_routes().unwrap().len(), 0);
    assert_eq!(
        package
            .canonical_snapshot()
            .unwrap()
            .fingerprint_manifest
            .reserved_feature_status[0]
            .owner_role,
        "05-streaming-realtime-rules"
    );

    let mut projected_reserved = package.capabilities[0].clone();
    projected_reserved.visibility = CapabilityVisibility::Active;
    projected_reserved.projection = ProjectionMode::DescriptorOnly;
    let error = projected_reserved
        .project_for_provider()
        .expect_err("reserved projection fails");
    assert_eq!(
        error.kind(),
        agent_sdk_core::AgentErrorKind::InvalidStateTransition
    );

    let mut executable_reserved = package.capabilities[0].clone();
    executable_reserved.executor_ref = Some(ExecutorRef::new("executor.reserved"));
    let error = executable_reserved
        .executable_route()
        .expect_err("reserved execution fails");
    assert_eq!(
        error.kind(),
        agent_sdk_core::AgentErrorKind::InvalidStateTransition
    );
}

#[test]
fn catalog_snapshot_and_package_delta_create_next_snapshot() {
    let package = package_with_tool();
    let hidden_tool = CapabilitySpec::fake_tool(
        "cap.hidden_search",
        "hidden_search",
        schema_ref("v1"),
        ExecutorRef::new("executor.hidden_search.v1"),
        approval_policy("policy.approval.hidden_search"),
        source("source.sdk.discovery"),
    );
    let catalog = CapabilityCatalogSnapshot {
        catalog_id: "catalog.discovery.1".to_string(),
        source_kind: CapabilitySourceKind::DiscoveryIndex,
        source_ref: source("source.sdk.discovery"),
        version: Some("v1".to_string()),
        content_hash: Some("sha256:catalog.discovery.1".to_string()),
        trust_state: TrustClass::SdkGenerated,
        activation_policy_ref: package_policy("policy.activation.discovery"),
        candidates: vec![CapabilityId::new("cap.hidden_search")],
    };
    let delta = PackageDelta {
        previous_fingerprint: package.fingerprint().expect("fingerprint"),
        requested_by: source("source.sdk.discovery"),
        reason: "activate hidden discovery candidate for next package".to_string(),
        activated_capabilities: vec![hidden_tool],
        deactivated_capability_ids: Vec::new(),
        catalogs: vec![catalog],
        sidecars: Vec::new(),
    };

    let next = package.apply_delta(delta).expect("delta applies");
    assert_eq!(next.capabilities.len(), 2);
    assert_eq!(next.catalogs.len(), 1);
    assert_ne!(
        package.fingerprint().expect("old fingerprint"),
        next.fingerprint().expect("next fingerprint")
    );
}

#[test]
fn sdk_consumer_conformance_report_is_fake_friendly() {
    let package = package_with_tool();
    let report = package
        .conformance_report()
        .expect("conformance report is deterministic");
    assert_eq!(
        report,
        RuntimePackageConformanceReport {
            fingerprint: package.fingerprint().unwrap(),
            provider_projection_count: 1,
            executable_route_count: 1,
            reserved_inactive_count: 0,
            catalog_count: 0,
            sidecar_count: 1,
        }
    );
}
