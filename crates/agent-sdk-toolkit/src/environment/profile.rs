use std::collections::BTreeSet;

use agent_sdk_core::{
    AgentError, DestinationKind, DestinationRef, ExecutionEnvironment, ExecutionEnvironmentBuilder,
    ExecutionEnvironmentKind, FilesystemIsolationPolicy, IsolationCapability, IsolationClass,
    IsolationRequirement, IsolationRequirementSnapshot, IsolationRuntimeRef,
    NetworkIsolationPolicy, PolicyKind, PolicyRef, RuntimePackageSidecarId, SecretExposurePolicy,
    SourceKind, SourceRef, WorkspaceMountMode,
};

use crate::environment::{EgressAllowlist, EnvironmentRuntime};

#[derive(Clone, Debug)]
/// Builder for common isolated agent workspace environments.
///
/// The builder is data-only. It lowers into core isolation DTOs and does not
/// install runtimes, start containers, open sockets, resolve DNS, or configure
/// host firewall state.
pub struct AgentWorkspaceEnvironmentProfile {
    environment_id: String,
    isolation_class: IsolationClass,
    preferred_runtimes: Vec<IsolationRuntimeRef>,
    workspace_ref: Option<String>,
    workspace_mode: WorkspaceMountMode,
    network: NetworkIsolationPolicy,
    egress_allowlist: Option<EgressAllowlist>,
    required_capabilities: BTreeSet<IsolationCapability>,
    source: SourceRef,
    destination: DestinationRef,
}

impl AgentWorkspaceEnvironmentProfile {
    /// Creates a profile with safe defaults: no network, no ambient secrets,
    /// snapshot workspace mounts when a workspace is supplied, and cleanup-required lifecycle.
    pub fn new(environment_id: impl Into<String>) -> Self {
        let mut required_capabilities = BTreeSet::new();
        required_capabilities.insert(IsolationCapability::Cleanup);
        required_capabilities.insert(IsolationCapability::ReadOnlyRoot);
        required_capabilities.insert(IsolationCapability::ProcessTimeout);
        required_capabilities.insert(IsolationCapability::IoRedaction);
        required_capabilities.insert(IsolationCapability::NoNetworkGuarantee);
        Self {
            environment_id: environment_id.into(),
            isolation_class: IsolationClass::Container,
            preferred_runtimes: Vec::new(),
            workspace_ref: None,
            workspace_mode: WorkspaceMountMode::Snapshot,
            network: NetworkIsolationPolicy::Disabled,
            egress_allowlist: None,
            required_capabilities,
            source: SourceRef::with_kind(SourceKind::Sdk, "source.sdk.toolkit.environment"),
            destination: DestinationRef::with_kind(
                DestinationKind::ExternalRuntime,
                "destination.toolkit.environment",
            ),
        }
    }

    /// Sets the minimum isolation class required for this environment.
    pub fn isolation_class(mut self, isolation_class: IsolationClass) -> Self {
        self.isolation_class = isolation_class;
        self
    }

    /// Adds a preferred runtime adapter ref. Selection still requires a
    /// registered adapter capability report that satisfies the package policy.
    pub fn prefer_runtime(mut self, runtime_ref: impl Into<IsolationRuntimeRef>) -> Self {
        self.preferred_runtimes.push(runtime_ref.into());
        self
    }

    /// Sets the minimum isolation class and preferred runtime from a known toolkit runtime.
    pub fn runtime(mut self, runtime: EnvironmentRuntime) -> Self {
        self.isolation_class = runtime.isolation_class();
        self.preferred_runtimes.push(runtime.into());
        self
    }

    /// Mounts a workspace by alias using the default snapshot mode.
    pub fn workspace(mut self, workspace_ref: impl Into<String>) -> Self {
        self.workspace_ref = Some(workspace_ref.into());
        self.workspace_mode = WorkspaceMountMode::Snapshot;
        self
    }

    /// Mounts a workspace by alias with an explicit core workspace mode.
    pub fn workspace_with_mode(
        mut self,
        workspace_ref: impl Into<String>,
        mode: WorkspaceMountMode,
    ) -> Self {
        self.workspace_ref = Some(workspace_ref.into());
        self.workspace_mode = mode;
        self
    }

    /// Keeps the environment network disabled and requires a no-network guarantee.
    pub fn no_network(mut self) -> Self {
        self.network = NetworkIsolationPolicy::Disabled;
        self.egress_allowlist = None;
        self.required_capabilities
            .remove(&IsolationCapability::EgressAllowlist);
        self.required_capabilities
            .insert(IsolationCapability::NoNetworkGuarantee);
        self
    }

    /// Applies a deterministic egress allowlist and requires adapter support for it.
    pub fn egress_allowlist(mut self, allowlist: EgressAllowlist) -> Self {
        if allowlist.is_empty() {
            return self.no_network();
        }
        self.egress_allowlist = Some(allowlist);
        self.required_capabilities
            .remove(&IsolationCapability::NoNetworkGuarantee);
        self.required_capabilities
            .insert(IsolationCapability::EgressAllowlist);
        self
    }

    /// Adds a required isolation capability to the lowered requirement.
    pub fn require_capability(mut self, capability: IsolationCapability) -> Self {
        self.required_capabilities.insert(capability);
        self
    }

    /// Sets the source ref used for lineage.
    pub fn source(mut self, source: SourceRef) -> Self {
        self.source = source;
        self
    }

    /// Sets the destination ref used for lineage.
    pub fn destination(mut self, destination: DestinationRef) -> Self {
        self.destination = destination;
        self
    }

    /// Builds the canonical core environment plus toolkit metadata.
    pub fn build(self) -> Result<AgentWorkspaceEnvironment, AgentError> {
        let environment_id = self.environment_id;
        let network = if let Some(allowlist) = &self.egress_allowlist {
            allowlist.network_policy()?
        } else {
            self.network
        };
        let mut requirement = IsolationRequirement::at_least(self.isolation_class);
        for runtime_ref in self.preferred_runtimes {
            requirement = requirement.prefer(runtime_ref);
        }
        requirement = requirement.require_capabilities(self.required_capabilities);

        let mut builder = ExecutionEnvironment::require(requirement)
            .environment_id(environment_id.as_str())
            .network(network)
            .secrets(SecretExposurePolicy::no_ambient())
            .ephemeral()
            .source(self.source)
            .destination(self.destination);

        builder = apply_workspace(builder, self.workspace_ref, self.workspace_mode);

        let mut environment = builder.build()?;
        environment.spec.kind = environment_kind_for_class(self.isolation_class);

        Ok(AgentWorkspaceEnvironment {
            environment,
            egress_allowlist: self.egress_allowlist,
        })
    }
}

#[derive(Clone, Debug)]
/// Lowered toolkit environment profile.
pub struct AgentWorkspaceEnvironment {
    /// Canonical core execution environment.
    pub environment: ExecutionEnvironment,
    /// Optional toolkit egress allowlist that produced the network policy.
    pub egress_allowlist: Option<EgressAllowlist>,
}

impl AgentWorkspaceEnvironment {
    /// Returns the canonical core execution environment.
    pub fn environment(&self) -> &ExecutionEnvironment {
        &self.environment
    }

    /// Consumes this profile into the canonical core execution environment.
    pub fn into_environment(self) -> ExecutionEnvironment {
        self.environment
    }

    /// Returns the optional egress allowlist that produced the network policy.
    pub fn egress_allowlist(&self) -> Option<&EgressAllowlist> {
        self.egress_allowlist.as_ref()
    }

    /// Builds an isolation requirement snapshot for attachment to a `RuntimePackage`.
    pub fn isolation_snapshot(
        &self,
        sidecar_id: RuntimePackageSidecarId,
        redaction_policy_ref: PolicyRef,
        cleanup_policy_ref: PolicyRef,
        child_lifecycle_policy_ref: PolicyRef,
    ) -> IsolationRequirementSnapshot {
        IsolationRequirementSnapshot::from_environment(
            sidecar_id,
            &self.environment,
            redaction_policy_ref,
            cleanup_policy_ref,
            child_lifecycle_policy_ref,
        )
    }

    /// Builds an isolation snapshot using conservative toolkit policy refs.
    pub fn default_isolation_snapshot(&self) -> IsolationRequirementSnapshot {
        self.isolation_snapshot(
            RuntimePackageSidecarId::new("sidecar.toolkit.environment.isolation"),
            PolicyRef::with_kind(
                PolicyKind::RuntimePackage,
                "policy.toolkit.environment.redaction",
            ),
            PolicyRef::with_kind(
                PolicyKind::RuntimePackage,
                "policy.toolkit.environment.cleanup",
            ),
            PolicyRef::with_kind(
                PolicyKind::RuntimePackage,
                "policy.toolkit.environment.child_lifecycle",
            ),
        )
    }
}

fn apply_workspace(
    builder: ExecutionEnvironmentBuilder,
    workspace_ref: Option<String>,
    mode: WorkspaceMountMode,
) -> ExecutionEnvironmentBuilder {
    if let Some(workspace_ref) = workspace_ref {
        builder.workspace(workspace_ref, mode)
    } else {
        builder.filesystem(FilesystemIsolationPolicy::no_workspace())
    }
}

fn environment_kind_for_class(isolation_class: IsolationClass) -> ExecutionEnvironmentKind {
    match isolation_class {
        IsolationClass::HostProcess => ExecutionEnvironmentKind::HostProcess,
        IsolationClass::Sandbox => ExecutionEnvironmentKind::Sandbox,
        IsolationClass::Container => ExecutionEnvironmentKind::Container,
        IsolationClass::LightweightVm => ExecutionEnvironmentKind::LightweightVm,
        IsolationClass::RemoteSandbox => ExecutionEnvironmentKind::RemoteSandbox,
    }
}
