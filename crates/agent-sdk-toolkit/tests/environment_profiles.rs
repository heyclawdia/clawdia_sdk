use std::{env, process::Command, sync::Arc};

use agent_sdk_core::{
    AgentId, AgentSnapshot, EffectTerminalStatus, IsolationCapability, IsolationCapabilityReport,
    IsolationClass, IsolationLifecycleContext, IsolationLifecycleCoordinator, IsolationMatchStatus,
    IsolationRecord, IsolationRequirementSnapshot, IsolationRuntimeKind, IsolationRuntimeRegistry,
    JournalRecordKind, NetworkIsolationPolicy, PolicyKind, PolicyRef, ProviderRouteSnapshot,
    RuntimePackage, RuntimePackageId, RuntimePackageSidecarId, WorkspaceMountMode,
    testing::{FakeIsolationRuntime, FakeJournalStore},
};
use agent_sdk_toolkit::environment::{
    AgentWorkspaceEnvironment, AgentWorkspaceEnvironmentProfile, EgressAllowlist,
};

#[test]
fn egress_allowlist_validates_canonicalizes_dedups_and_lowers_to_core_policy() {
    let allowlist = EgressAllowlist::new()
        .allow("https://docs.rs")
        .allow("crates.io:443")
        .allow("http://example.test:80")
        .allow("https://docs.rs")
        .allow("http://example.test:80");

    assert_eq!(
        allowlist
            .canonical_entries()
            .expect("allowlist canonicalizes"),
        [
            "https://crates.io:443",
            "https://docs.rs:443",
            "http://example.test:80"
        ]
    );
    assert_eq!(
        allowlist.network_policy().expect("allowlist lowers"),
        NetworkIsolationPolicy::EgressScoped {
            rules: vec![
                "https://crates.io:443".to_string(),
                "https://docs.rs:443".to_string(),
                "http://example.test:80".to_string(),
            ],
        }
    );
}

#[test]
fn egress_allowlist_reports_invalid_scheme_host_and_port_errors() {
    assert_allowlist_error("ftp://docs.rs", "protocol");
    assert_allowlist_error("https://", "host");
    assert_allowlist_error("exa mple.test:443", "whitespace");
    assert_allowlist_error("https://docs.rs:0", "port");
    assert_allowlist_error("docs.rs:65536", "port");
}

#[test]
fn workspace_environment_profile_defaults_to_no_network() {
    let profile = AgentWorkspaceEnvironmentProfile::new("env.agent.workspace")
        .workspace("workspace.primary")
        .build()
        .expect("profile builds");

    assert_eq!(
        profile.environment().environment_id.as_str(),
        "env.agent.workspace"
    );
    assert_eq!(
        profile.environment().spec.filesystem.workspace.mode,
        WorkspaceMountMode::Snapshot
    );
    assert_eq!(
        profile.environment().spec.network,
        NetworkIsolationPolicy::Disabled
    );
}

#[test]
fn workspace_environment_profile_lowers_allowlist_to_environment_and_snapshot() {
    let allowlist = EgressAllowlist::new()
        .allow("https://docs.rs")
        .allow("crates.io:443")
        .allow("http://example.test:80");
    let profile = AgentWorkspaceEnvironmentProfile::new("env.agent.workspace")
        .workspace("workspace.primary")
        .isolation_class(IsolationClass::Container)
        .prefer_runtime("runtime.fake.container")
        .egress_allowlist(allowlist)
        .build()
        .expect("profile builds");
    let snapshot = profile.isolation_snapshot(
        RuntimePackageSidecarId::new("sidecar.environment.profile"),
        policy("policy.environment.redaction"),
        policy("policy.environment.cleanup"),
        policy("policy.environment.child"),
    );

    assert_eq!(
        profile.environment().spec.requirement.minimum_class,
        IsolationClass::Container
    );
    assert_eq!(
        profile.environment().spec.kind,
        agent_sdk_core::ExecutionEnvironmentKind::Container
    );
    assert_eq!(
        profile.environment().spec.network,
        NetworkIsolationPolicy::EgressScoped {
            rules: vec![
                "https://crates.io:443".to_string(),
                "https://docs.rs:443".to_string(),
                "http://example.test:80".to_string(),
            ],
        }
    );
    assert_eq!(
        snapshot,
        IsolationRequirementSnapshot::from_environment(
            RuntimePackageSidecarId::new("sidecar.environment.profile"),
            profile.environment(),
            policy("policy.environment.redaction"),
            policy("policy.environment.cleanup"),
            policy("policy.environment.child"),
        )
    );
}

#[test]
fn workspace_environment_profile_denies_allowlist_when_runtime_lacks_capability() {
    let profile = AgentWorkspaceEnvironmentProfile::new("env.agent.workspace")
        .workspace("workspace.primary")
        .isolation_class(IsolationClass::Container)
        .prefer_runtime("runtime.fake.container")
        .egress_allowlist(EgressAllowlist::new().allow("crates.io:443"))
        .build()
        .expect("profile builds");
    let package = package_for(&profile);
    let runtime = Arc::new(FakeIsolationRuntime::with_report(container_report(
        "runtime.fake.container",
    )));
    let mut registry = IsolationRuntimeRegistry::new();
    registry
        .register(runtime.clone())
        .expect("runtime registers");
    let journal = FakeJournalStore::default();
    let coordinator = IsolationLifecycleCoordinator::new(Arc::new(journal), registry);
    let process = profile
        .environment()
        .process(["/bin/echo", "must-not-start"])
        .build()
        .expect("process builds");

    let outcome = coordinator
        .start_process(
            &package,
            profile.environment().clone(),
            process,
            IsolationLifecycleContext::test(package.fingerprint().expect("fingerprint")),
            None,
        )
        .expect("capability gap returns denied outcome");

    assert_eq!(outcome.status, IsolationMatchStatus::DowngradeDenied);
    assert_eq!(runtime.start_process_call_count(), 0);
}

#[test]
fn package_fingerprint_changes_when_profile_network_policy_changes() {
    let offline_profile = AgentWorkspaceEnvironmentProfile::new("env.agent.workspace")
        .workspace("workspace.primary")
        .isolation_class(IsolationClass::Container)
        .prefer_runtime("runtime.fake.container")
        .build()
        .expect("offline profile builds");
    let networked_profile = AgentWorkspaceEnvironmentProfile::new("env.agent.workspace")
        .workspace("workspace.primary")
        .isolation_class(IsolationClass::Container)
        .prefer_runtime("runtime.fake.container")
        .egress_allowlist(EgressAllowlist::new().allow("crates.io:443"))
        .build()
        .expect("networked profile builds");

    assert_ne!(
        package_for(&offline_profile)
            .fingerprint()
            .expect("offline fingerprint"),
        package_for(&networked_profile)
            .fingerprint()
            .expect("networked fingerprint"),
        "network policy must stay part of the canonical isolation fingerprint"
    );
}

#[test]
fn workspace_environment_profile_starts_process_through_fake_runtime_coordinator() {
    let profile = AgentWorkspaceEnvironmentProfile::new("env.agent.workspace")
        .workspace("workspace.primary")
        .isolation_class(IsolationClass::Container)
        .prefer_runtime("runtime.fake.container")
        .build()
        .expect("profile builds");
    let package = package_for(&profile);
    let runtime = Arc::new(FakeIsolationRuntime::with_report(container_report(
        "runtime.fake.container",
    )));
    let mut registry = IsolationRuntimeRegistry::new();
    registry
        .register(runtime.clone())
        .expect("runtime registers");
    let journal = FakeJournalStore::default();
    let coordinator = IsolationLifecycleCoordinator::new(Arc::new(journal.clone()), registry);
    let process = profile
        .environment()
        .process(["/bin/echo", "hello-from-fake-container"])
        .build()
        .expect("process builds");

    let outcome = coordinator
        .start_process(
            &package,
            profile.environment().clone(),
            process,
            IsolationLifecycleContext::test(package.fingerprint().expect("fingerprint")),
            None,
        )
        .expect("fake runtime starts process");

    assert_eq!(outcome.status, IsolationMatchStatus::Matched);
    assert_eq!(runtime.start_process_call_count(), 1);
    let result_record = match outcome.result_record.as_ref().expect("process result") {
        IsolationRecord::ProcessStartResult(record) => record,
        other => panic!("expected process start result record, got {other:?}"),
    };
    assert_eq!(
        result_record.effect_result.terminal_status,
        EffectTerminalStatus::Completed
    );
    assert_eq!(
        journal.records()[0].record_kind,
        JournalRecordKind::EffectIntent
    );
    assert!(
        outcome
            .io_frames
            .iter()
            .all(|frame| !frame.raw_content_present),
        "fake runtime must preserve redacted process I/O defaults"
    );
}

#[test]
fn local_container_runtime_smoke_is_opt_in() {
    if env::var("AGENT_SDK_TOOLKIT_RUN_LOCAL_CONTAINER_SMOKE").as_deref() != Ok("1") {
        return;
    }
    let explicit_cli = env::var("AGENT_SDK_TOOLKIT_CONTAINER_CLI").ok();
    let cli = explicit_cli.as_deref().unwrap_or("container");
    let version = match Command::new(cli).arg("--version").output() {
        Ok(version) => version,
        Err(error) if explicit_cli.is_some() => {
            panic!("explicit container CLI path {cli:?} should run: {error}");
        }
        Err(_) => return,
    };
    assert!(
        version.status.success(),
        "container CLI should report a usable version when smoke is enabled"
    );

    let profile = AgentWorkspaceEnvironmentProfile::new("env.agent.local_container_smoke")
        .workspace("workspace.primary")
        .isolation_class(IsolationClass::Container)
        .prefer_runtime("runtime.local.container")
        .build()
        .expect("profile builds");
    assert_eq!(
        profile.environment().spec.requirement.minimum_class,
        IsolationClass::Container
    );
    assert_eq!(
        profile.environment().spec.network,
        NetworkIsolationPolicy::Disabled
    );
}

fn assert_allowlist_error(entry: &str, expected: &str) {
    let error = EgressAllowlist::new()
        .allow(entry)
        .network_policy()
        .expect_err("invalid allowlist entry should fail");
    assert!(
        error.context().message.contains(expected),
        "error for {entry:?} should mention {expected:?}, got: {}",
        error.context().message
    );
}

fn package_for(profile: &AgentWorkspaceEnvironment) -> RuntimePackage {
    RuntimePackage::builder(RuntimePackageId::new("package.environment.profile"))
        .agent(AgentSnapshot {
            agent_id: AgentId::new("agent.environment.profile"),
            name: "environment profile".to_string(),
            default_behavior_refs: Vec::new(),
        })
        .provider_route(ProviderRouteSnapshot::new(
            "provider.fake",
            "model.fake.environment",
        ))
        .isolation_requirement(profile.isolation_snapshot(
            RuntimePackageSidecarId::new("sidecar.environment.profile"),
            policy("policy.environment.redaction"),
            policy("policy.environment.cleanup"),
            policy("policy.environment.child"),
        ))
        .build()
        .expect("package builds")
}

fn container_report(runtime_ref: &str) -> IsolationCapabilityReport {
    let mut report = IsolationCapabilityReport::sandbox(runtime_ref);
    report.adapter_kind = IsolationRuntimeKind::Container;
    report.supported_classes = vec![IsolationClass::Container];
    report.capabilities = report.capabilities.with_all([
        IsolationCapability::Cleanup,
        IsolationCapability::ContentRefIo,
        IsolationCapability::IoRedaction,
        IsolationCapability::NoNetworkGuarantee,
        IsolationCapability::ProcessStats,
        IsolationCapability::ProcessTimeout,
        IsolationCapability::ReadOnlyRoot,
        IsolationCapability::SecretIsolation,
    ]);
    report
}

fn policy(id: &str) -> PolicyRef {
    PolicyRef::with_kind(PolicyKind::RuntimePackage, id)
}
