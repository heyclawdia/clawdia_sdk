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

pub mod realtime;

#[derive(Clone, Default)]
pub struct ProviderRegistry {
    providers: BTreeMap<String, Arc<dyn ProviderAdapter>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self::default()
    }

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

    pub fn get(&self, route_id: &str) -> Option<Arc<dyn ProviderAdapter>> {
        self.providers.get(route_id).cloned()
    }

    pub fn contains_route(&self, route_id: &str) -> bool {
        self.providers.contains_key(route_id)
    }

    pub fn len(&self) -> usize {
        self.providers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }
}

pub trait RuntimePackageResolver: Send + Sync {
    fn resolve(&self, package_id: &RuntimePackageId) -> Result<RuntimePackage, AgentError>;
}

#[derive(Clone, Default)]
pub struct InMemoryRuntimePackageResolver {
    packages: Arc<Mutex<BTreeMap<RuntimePackageId, RuntimePackage>>>,
}

impl InMemoryRuntimePackageResolver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, package: RuntimePackage) -> Result<(), AgentError> {
        package.validate()?;
        self.packages
            .lock()
            .map_err(|_| AgentError::contract_violation("package resolver lock poisoned"))?
            .insert(package.package_id.clone(), package);
        Ok(())
    }

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

pub trait RuntimePolicyPort: Send + Sync {
    fn evaluate_run_start(
        &self,
        request: &RunRequest,
        package: &RuntimePackage,
    ) -> Result<PolicyOutcome, AgentError>;
}

pub trait OutputSinkPort: Send + Sync {
    fn sink_id(&self) -> &str;
}

#[derive(Clone, Default)]
pub struct OutputSinkRegistry {
    sinks: BTreeMap<String, Arc<dyn OutputSinkPort>>,
}

impl OutputSinkRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, sink: Arc<dyn OutputSinkPort>) -> Result<(), AgentError> {
        let sink_id = sink.sink_id().to_string();
        if sink_id.is_empty() {
            return Err(AgentError::missing_required_field("output_sink.sink_id"));
        }
        self.sinks.insert(sink_id, sink);
        Ok(())
    }

    pub fn get(&self, sink_id: &str) -> Option<Arc<dyn OutputSinkPort>> {
        self.sinks.get(sink_id).cloned()
    }

    pub fn len(&self) -> usize {
        self.sinks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sinks.is_empty()
    }
}
