use std::fmt;

use agent_sdk_core::{IsolationClass, IsolationRuntimeRef};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
/// Well-known toolkit runtime refs for common environment profiles.
///
/// These values are data-only aliases for `IsolationRuntimeRef`. Hosts and
/// optional runtime crates still own adapter registration and enforcement.
pub enum EnvironmentRuntime {
    /// Local container adapter, for example a host-provided container runtime.
    #[serde(rename = "runtime:local.container")]
    LocalContainer,
    /// Local sandbox adapter.
    #[serde(rename = "runtime:local.sandbox")]
    LocalSandbox,
    /// Local lightweight VM adapter.
    #[serde(rename = "runtime:local.lightweight_vm")]
    LocalLightweightVm,
    /// Remote sandbox adapter.
    #[serde(rename = "runtime:remote.sandbox")]
    RemoteSandbox,
    /// Explicit host-process adapter. Use only when policy allows host process execution.
    #[serde(rename = "runtime:local.host_process")]
    LocalHostProcess,
}

impl EnvironmentRuntime {
    /// Returns the stable runtime ref lowered into core isolation contracts.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LocalContainer => "runtime:local.container",
            Self::LocalSandbox => "runtime:local.sandbox",
            Self::LocalLightweightVm => "runtime:local.lightweight_vm",
            Self::RemoteSandbox => "runtime:remote.sandbox",
            Self::LocalHostProcess => "runtime:local.host_process",
        }
    }

    /// Returns the matching minimum isolation class for this runtime kind.
    pub fn isolation_class(self) -> IsolationClass {
        match self {
            Self::LocalContainer => IsolationClass::Container,
            Self::LocalSandbox => IsolationClass::Sandbox,
            Self::LocalLightweightVm => IsolationClass::LightweightVm,
            Self::RemoteSandbox => IsolationClass::RemoteSandbox,
            Self::LocalHostProcess => IsolationClass::HostProcess,
        }
    }

    /// Returns the core runtime ref for this well-known runtime.
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
