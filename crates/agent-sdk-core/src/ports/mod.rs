//! Host adapter boundaries for the SDK core. Use these traits and registries when
//! hosts provide providers, journals, sinks, tools, isolation, extensions, telemetry,
//! or subscriptions. Implementations may perform external side effects and must honor
//! policy, redaction, idempotency, and replay contracts.
//!
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

pub use crate::provider::ProviderAdapter;

use crate::{
    domain::{AgentError, AgentErrorKind, RetryClassification, RuntimePackageId},
    package::RuntimePackage,
    policy::PolicyOutcome,
    run::RunRequest,
};

/// Public realtime namespace. Use it for the documented realtime API
/// surface; prefer crate-root re-exports for common imports. Module
/// items must preserve the core ownership and side-effect boundaries
/// described in this file.
pub mod realtime;

#[derive(Clone, Default)]
/// Carries provider registry data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ProviderRegistry {
    providers: BTreeMap<String, Arc<dyn ProviderAdapter>>,
}

impl ProviderRegistry {
    /// Creates a new ports value with explicit caller-provided inputs.
    /// This constructor is data-only and performs no I/O or external
    /// side effects.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register.
    /// This mutates the in-memory registry so later runtime resolution can find the adapter or
    /// package.
    pub fn register(
        &mut self,
        route_id: impl Into<String>,
        provider: Arc<dyn ProviderAdapter>,
    ) -> Result<(), AgentError> {
        let route_id = route_id.into();
        if route_id.is_empty() {
            return Err(AgentError::missing_required_field(
                "provider_registry.route_id",
            ));
        }
        self.providers.insert(route_id, provider);
        Ok(())
    }

    /// Looks up an entry in this local store without registry or runtime work.
    /// This reads registry metadata and does not call the registered adapter or sink.
    pub fn get(&self, route_id: &str) -> Option<Arc<dyn ProviderAdapter>> {
        self.providers.get(route_id).cloned()
    }

    /// Reads the stored contains route without registry or runtime work.
    /// This reads registry metadata and does not call the registered adapter or sink.
    pub fn contains_route(&self, route_id: &str) -> bool {
        self.providers.contains_key(route_id)
    }

    /// Reads the stored len without registry or runtime work.
    /// This reads registry metadata and does not call the registered adapter or sink.
    pub fn len(&self) -> usize {
        self.providers.len()
    }

    /// Reports whether this value is empty. The check is pure and does
    /// not mutate SDK or host state.
    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }
}

/// Port or behavior contract for runtime package resolver. Implementors
/// should preserve policy, redaction, idempotency, and replay
/// expectations from the surrounding module. Implementations may
/// perform side effects only as described by the trait methods.
pub trait RuntimePackageResolver: Send + Sync {
    /// Resolves resolve through the configured ports boundary. Concrete
    /// implementations own any backing-store, filesystem, or network side
    /// effects.
    fn resolve(&self, package_id: &RuntimePackageId) -> Result<RuntimePackage, AgentError>;
}

#[derive(Clone, Default)]
/// Carries in memory runtime package resolver data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct InMemoryRuntimePackageResolver {
    packages: Arc<Mutex<BTreeMap<RuntimePackageId, RuntimePackage>>>,
}

impl InMemoryRuntimePackageResolver {
    /// Creates a new ports value with explicit caller-provided inputs.
    /// This constructor is data-only and performs no I/O or external
    /// side effects.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert.
    /// This mutates the in-memory registry so later runtime resolution can find the adapter or
    /// package.
    pub fn insert(&self, package: RuntimePackage) -> Result<(), AgentError> {
        package.validate()?;
        self.packages
            .lock()
            .map_err(|_| AgentError::contract_violation("package resolver lock poisoned"))?
            .insert(package.package_id.clone(), package);
        Ok(())
    }

    /// Constructs this value from packages. Use it when adapting
    /// canonical SDK records without introducing a second behavior
    /// path.
    pub fn from_packages(
        packages: impl IntoIterator<Item = RuntimePackage>,
    ) -> Result<Self, AgentError> {
        let resolver = Self::new();
        for package in packages {
            resolver.insert(package)?;
        }
        Ok(resolver)
    }
}

impl RuntimePackageResolver for InMemoryRuntimePackageResolver {
    fn resolve(&self, package_id: &RuntimePackageId) -> Result<RuntimePackage, AgentError> {
        self.packages
            .lock()
            .map_err(|_| AgentError::contract_violation("package resolver lock poisoned"))?
            .get(package_id)
            .cloned()
            .ok_or_else(|| {
                AgentError::new(
                    AgentErrorKind::InvalidPackage,
                    RetryClassification::HostConfigurationNeeded,
                    format!(
                        "runtime package {} is not registered with the package resolver",
                        package_id.as_str()
                    ),
                )
            })
    }
}

/// Port or behavior contract for runtime policy port. Implementors
/// should preserve policy, redaction, idempotency, and replay
/// expectations from the surrounding module. Implementations may
/// perform side effects only as described by the trait methods.
pub trait RuntimePolicyPort: Send + Sync {
    /// Evaluates whether a run may start with the resolved runtime package.
    /// Implementations may consult host policy state, but must not register the
    /// run, call the provider, append journal records, or publish events.
    fn evaluate_run_start(
        &self,
        request: &RunRequest,
        package: &RuntimePackage,
    ) -> Result<PolicyOutcome, AgentError>;
}

/// Port or behavior contract for output sink port. Implementors should
/// preserve policy, redaction, idempotency, and replay expectations
/// from the surrounding module. Implementations may perform side
/// effects only as described by the trait methods.
pub trait OutputSinkPort: Send + Sync {
    /// Returns the sink id identifier for this adapter.
    /// This reads registry metadata and does not call the registered adapter or sink.
    fn sink_id(&self) -> &str;
}

#[derive(Clone, Default)]
/// Carries output sink registry data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct OutputSinkRegistry {
    sinks: BTreeMap<String, Arc<dyn OutputSinkPort>>,
}

impl OutputSinkRegistry {
    /// Creates a new ports value with explicit caller-provided inputs.
    /// This constructor is data-only and performs no I/O or external
    /// side effects.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register.
    /// This mutates the in-memory registry so later runtime resolution can find the adapter or
    /// package.
    pub fn register(&mut self, sink: Arc<dyn OutputSinkPort>) -> Result<(), AgentError> {
        let sink_id = sink.sink_id().to_string();
        if sink_id.is_empty() {
            return Err(AgentError::missing_required_field("output_sink.sink_id"));
        }
        self.sinks.insert(sink_id, sink);
        Ok(())
    }

    /// Looks up an entry in this local store without registry or runtime work.
    /// This reads registry metadata and does not call the registered adapter or sink.
    pub fn get(&self, sink_id: &str) -> Option<Arc<dyn OutputSinkPort>> {
        self.sinks.get(sink_id).cloned()
    }

    /// Reads the stored len without registry or runtime work.
    /// This reads registry metadata and does not call the registered adapter or sink.
    pub fn len(&self) -> usize {
        self.sinks.len()
    }

    /// Reports whether this value is empty. The check is pure and does
    /// not mutate SDK or host state.
    pub fn is_empty(&self) -> bool {
        self.sinks.is_empty()
    }
}
