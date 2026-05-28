//! Data-only aliases for common isolation runtime refs.
//!
//! These helpers make profile construction easier while keeping concrete
//! adapter registration, process startup, network policy, and cleanup owned by
//! the host or a future optional runtime-adapter crate.

use std::fmt;

use agent_sdk_core::{IsolationClass, IsolationRuntimeRef};
use serde::{Deserialize, Serialize};

/// Well-known toolkit runtime refs for common environment profiles.
///
/// These values are data-only aliases for `IsolationRuntimeRef`. Hosts and
/// optional runtime crates still own adapter registration and enforcement.
///
/// ```
/// use agent_sdk_core::{IsolationClass, IsolationRuntimeRef};
/// use agent_sdk_toolkit::EnvironmentRuntime;
///
/// let runtime = EnvironmentRuntime::LocalContainer;
/// assert_eq!(runtime.as_str(), "runtime:local.container");
/// assert_eq!(runtime.isolation_class(), IsolationClass::Container);
///
/// let runtime_ref: IsolationRuntimeRef = runtime.into();
/// assert_eq!(runtime_ref.as_str(), "runtime:local.container");
/// ```
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum EnvironmentRuntime {
    /// Local container adapter ref (`runtime:local.container`), requiring container isolation.
    #[serde(rename = "runtime:local.container")]
    LocalContainer,
    /// Local sandbox adapter ref (`runtime:local.sandbox`), requiring sandbox isolation.
    #[serde(rename = "runtime:local.sandbox")]
    LocalSandbox,
    /// Local lightweight VM adapter ref (`runtime:local.lightweight_vm`), requiring VM isolation.
    #[serde(rename = "runtime:local.lightweight_vm")]
    LocalLightweightVm,
    /// Remote sandbox adapter ref (`runtime:remote.sandbox`), requiring remote sandbox isolation.
    #[serde(rename = "runtime:remote.sandbox")]
    RemoteSandbox,
    /// Explicit host-process adapter ref (`runtime:local.host_process`).
    ///
    /// Hosts should use this only when policy allows host process execution.
    #[serde(rename = "runtime:local.host_process")]
    LocalHostProcess,
}

impl EnvironmentRuntime {
    /// Returns the stable serialized runtime ref lowered into core isolation contracts.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LocalContainer => "runtime:local.container",
            Self::LocalSandbox => "runtime:local.sandbox",
            Self::LocalLightweightVm => "runtime:local.lightweight_vm",
            Self::RemoteSandbox => "runtime:remote.sandbox",
            Self::LocalHostProcess => "runtime:local.host_process",
        }
    }

    /// Returns the matching minimum isolation class for this runtime ref.
    ///
    /// The value is used by `AgentWorkspaceEnvironmentProfile::runtime` when it
    /// builds a core isolation requirement. It is not an adapter capability
    /// check by itself.
    pub fn isolation_class(self) -> IsolationClass {
        match self {
            Self::LocalContainer => IsolationClass::Container,
            Self::LocalSandbox => IsolationClass::Sandbox,
            Self::LocalLightweightVm => IsolationClass::LightweightVm,
            Self::RemoteSandbox => IsolationClass::RemoteSandbox,
            Self::LocalHostProcess => IsolationClass::HostProcess,
        }
    }

    /// Returns the core runtime ref for this well-known runtime alias.
    ///
    /// Constructing the ref is data-only. A host must still register a matching
    /// isolation adapter before execution can select it.
    pub fn runtime_ref(self) -> IsolationRuntimeRef {
        IsolationRuntimeRef::new(self.as_str())
    }
}

impl fmt::Display for EnvironmentRuntime {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl From<EnvironmentRuntime> for IsolationRuntimeRef {
    fn from(runtime: EnvironmentRuntime) -> Self {
        runtime.runtime_ref()
    }
}
