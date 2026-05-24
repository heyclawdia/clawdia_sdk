use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};

use crate::{
    content_ports::ContentResolver,
    domain::{
        AgentError, AgentErrorKind, AgentId, RetryClassification, RunId, RuntimePackageId,
        SourceRef,
    },
    error::CausalIds,
    event::{CompiledEventFilter, EventCursor, EventKind},
    event_bus::{AgentEventBus, AgentEventStream},
    journal::{JournalCursor, JournalRecord},
    journal_ports::RunJournal,
    package::{RuntimePackage, RuntimePackageFingerprint},
    policy::PolicyDecision,
    ports::{
        InMemoryRuntimePackageResolver, OutputSinkPort, OutputSinkRegistry, ProviderAdapter,
        ProviderRegistry, RuntimePackageResolver, RuntimePolicyPort,
    },
    run::{RunRequest, RunResult, RunStatus},
    run_handle::{InMemoryRunControlStore, RunControlStore, RunHandle},
    subscription::RunSubscriptionSource,
};

#[derive(Clone)]
pub struct AgentRuntime {
    inner: Arc<RuntimeInner>,
}

impl AgentRuntime {
    pub fn builder() -> AgentRuntimeBuilder {
        AgentRuntimeBuilder::default()
    }

    pub fn start_run(&self, request: RunRequest) -> Result<RunHandle, AgentError> {
        let _journal = self.journal_port(&request.run_id)?;
        let _events = self.event_bus_port(&request.run_id)?;
        let _content = self.content_port(&request.run_id)?;
        let policy = self.policy_port(&request.run_id)?;

        let effective = self.resolve_effective_package(&request)?;
        self.provider_for(&effective.package, &request.run_id)?;
        self.evaluate_run_start_policy(policy.as_ref(), &request, &effective.package)?;

        let cancellation = CancellationHandle::new();
        let entry = RegisteredRun {
            run_id: request.run_id.clone(),
            agent_id: request.agent_id.clone(),
            source: request.source.clone(),
            status: RunRegistryStatus::Registered,
            runtime_package_id: effective.package.package_id.clone(),
            runtime_package_fingerprint: effective.fingerprint,
            provider_route_id: effective.package.provider_route.route_id,
            provider_model_id: effective.package.provider_route.model_id,
            cancellation: cancellation.clone(),
        };

        self.insert_run(entry, &request.run_id)?;
        self.inner
            .run_control
            .register_run(request.run_id.clone(), request.agent_id.clone())?;
        Ok(RunHandle::new(
            request.run_id,
            Arc::new(RuntimeRunControlStore {
                runtime: self.clone(),
            }),
            Arc::new(RuntimeRunSubscriptionSource {
                runtime: self.clone(),
            }),
        ))
    }

    pub fn run_text(&self, request: RunRequest) -> Result<RunResult, AgentError> {
        let handle = self.start_run(request.clone())?;
        crate::loop_driver::run_p0_text(self, request, handle)
    }

    pub fn run_typed<T: crate::typed_output_ports::TypedOutputModel>(
        &self,
        request: RunRequest,
    ) -> Result<RunResult, AgentError> {
        self.run_text(request.with_output_contract(crate::output::OutputContract::for_type::<T>()))
    }

    pub fn resolve_effective_package(
        &self,
        request: &RunRequest,
    ) -> Result<EffectiveRuntimePackage, AgentError> {
        let package_id = self
            .inner
            .default_package_id
            .as_ref()
            .ok_or_else(|| missing_port_error("default runtime package", &request.run_id))?;
        let resolver = self.package_resolver_port(&request.run_id)?;
        let mut package = resolver.resolve(package_id).map_err(|error| {
            error.with_causal_ids(CausalIds {
                run_id: Some(request.run_id.clone()),
                ..CausalIds::default()
            })
        })?;
        if let Some(output_contract) = &request.output_contract {
            package = package
                .with_output_contract(output_contract)
                .map_err(|error| {
                    error.with_causal_ids(CausalIds {
                        run_id: Some(request.run_id.clone()),
                        ..CausalIds::default()
                    })
                })?;
        }

        package.validate().map_err(|error| {
            error.with_causal_ids(CausalIds {
                run_id: Some(request.run_id.clone()),
                ..CausalIds::default()
            })
        })?;
        if package.agent.agent_id != request.agent_id {
            return Err(AgentError::new(
                AgentErrorKind::InvalidPackage,
                RetryClassification::HostConfigurationNeeded,
                "runtime package agent snapshot must match the run request agent_id",
            )
            .with_causal_ids(CausalIds {
                run_id: Some(request.run_id.clone()),
                ..CausalIds::default()
            }));
        }

        let fingerprint = package.fingerprint().map_err(|error| {
            error.with_causal_ids(CausalIds {
                run_id: Some(request.run_id.clone()),
                ..CausalIds::default()
            })
        })?;
        Ok(EffectiveRuntimePackage {
            package,
            fingerprint,
        })
    }

    pub fn cancel_run(&self, run_id: &RunId) -> Result<(), AgentError> {
        let mut runs = self
            .inner
            .runs
            .lock()
            .map_err(|_| AgentError::contract_violation("run registry lock poisoned"))?;
        let entry = runs.get_mut(run_id).ok_or_else(|| {
            AgentError::new(
                AgentErrorKind::InvalidStateTransition,
                RetryClassification::RepairNeeded,
                "run is not registered with this runtime",
            )
            .with_causal_ids(CausalIds {
                run_id: Some(run_id.clone()),
                ..CausalIds::default()
            })
        })?;
        entry.cancellation.cancel();
        entry.status = RunRegistryStatus::CancellationRequested;
        self.inner.run_control.request_cancel(run_id)?;
        Ok(())
    }

    pub fn run_snapshot(&self, run_id: &RunId) -> Result<RunSnapshot, AgentError> {
        let runs = self
            .inner
            .runs
            .lock()
            .map_err(|_| AgentError::contract_violation("run registry lock poisoned"))?;
        runs.get(run_id)
            .map(RegisteredRun::snapshot)
            .ok_or_else(|| {
                AgentError::new(
                    AgentErrorKind::InvalidStateTransition,
                    RetryClassification::RepairNeeded,
                    "run is not registered with this runtime",
                )
                .with_causal_ids(CausalIds {
                    run_id: Some(run_id.clone()),
                    ..CausalIds::default()
                })
            })
    }

    pub fn registered_run_count(&self) -> Result<usize, AgentError> {
        Ok(self
            .inner
            .runs
            .lock()
            .map_err(|_| AgentError::contract_violation("run registry lock poisoned"))?
            .len())
    }

    pub fn events(&self) -> Result<Arc<dyn AgentEventBus>, AgentError> {
        self.event_bus_subscription_port()
    }

    pub fn subscribe_all(
        &self,
        cursor: Option<EventCursor>,
    ) -> Result<AgentEventStream, AgentError> {
        self.event_bus_subscription_port()?.subscribe_all(cursor)
    }

    pub fn subscribe_run(
        &self,
        run_id: RunId,
        cursor: Option<EventCursor>,
    ) -> Result<AgentEventStream, AgentError> {
        self.event_bus_subscription_port()?
            .subscribe_run(run_id, cursor)
    }

    pub fn subscribe_agent(
        &self,
        agent_id: AgentId,
        cursor: Option<EventCursor>,
    ) -> Result<AgentEventStream, AgentError> {
        self.event_bus_subscription_port()?
            .subscribe_agent(agent_id, cursor)
    }

    pub fn subscribe_events(
        &self,
        filter: CompiledEventFilter,
        cursor: Option<EventCursor>,
    ) -> Result<AgentEventStream, AgentError> {
        self.event_bus_subscription_port()?
            .subscribe_filtered(filter, cursor)
    }

    pub fn provider_registry(&self) -> &ProviderRegistry {
        &self.inner.providers
    }

    pub fn output_sinks(&self) -> &OutputSinkRegistry {
        &self.inner.output_sinks
    }

    fn insert_run(&self, entry: RegisteredRun, run_id: &RunId) -> Result<(), AgentError> {
        let mut runs = self
            .inner
            .runs
            .lock()
            .map_err(|_| AgentError::contract_violation("run registry lock poisoned"))?;
        if runs.contains_key(run_id) {
            return Err(AgentError::new(
                AgentErrorKind::InvalidStateTransition,
                RetryClassification::NotRetryable,
                "run_id is already registered with this runtime",
            )
            .with_causal_ids(CausalIds {
                run_id: Some(run_id.clone()),
                ..CausalIds::default()
            }));
        }
        runs.insert(run_id.clone(), entry);
        Ok(())
    }

    pub(crate) fn provider_for(
        &self,
        package: &RuntimePackage,
        run_id: &RunId,
    ) -> Result<Arc<dyn ProviderAdapter>, AgentError> {
        self.inner
            .providers
            .get(&package.provider_route.route_id)
            .ok_or_else(|| {
                AgentError::new(
                    AgentErrorKind::ProviderFailure,
                    RetryClassification::HostConfigurationNeeded,
                    format!(
                        "missing provider adapter for package route {}",
                        package.provider_route.route_id
                    ),
                )
                .with_causal_ids(CausalIds {
                    run_id: Some(run_id.clone()),
                    ..CausalIds::default()
                })
            })
    }

    pub(crate) fn provider_for_route(
        &self,
        route_id: &str,
        run_id: &RunId,
    ) -> Result<Arc<dyn ProviderAdapter>, AgentError> {
        self.inner.providers.get(route_id).ok_or_else(|| {
            AgentError::new(
                AgentErrorKind::ProviderFailure,
                RetryClassification::HostConfigurationNeeded,
                format!("missing provider adapter for package route {route_id}"),
            )
            .with_causal_ids(CausalIds {
                run_id: Some(run_id.clone()),
                ..CausalIds::default()
            })
        })
    }

    fn evaluate_run_start_policy(
        &self,
        policy: &dyn RuntimePolicyPort,
        request: &RunRequest,
        package: &RuntimePackage,
    ) -> Result<(), AgentError> {
        let outcome = policy.evaluate_run_start(request, package)?;
        if outcome.is_allowed() {
            return Ok(());
        }

        let mut error = AgentError::new(
            AgentErrorKind::PolicyDenial,
            RetryClassification::UserActionNeeded,
            policy_denial_message(&outcome.decision),
        )
        .with_causal_ids(CausalIds {
            run_id: Some(request.run_id.clone()),
            ..CausalIds::default()
        })
        .with_source(request.source.clone());
        for policy_ref in outcome.policy_refs {
            error = error.with_policy_ref(policy_ref);
        }
        Err(error)
    }

    fn package_resolver_port(
        &self,
        run_id: &RunId,
    ) -> Result<Arc<dyn RuntimePackageResolver>, AgentError> {
        self.inner
            .package_resolver
            .clone()
            .ok_or_else(|| missing_port_error("runtime package resolver", run_id))
    }

    pub(crate) fn journal_port(&self, run_id: &RunId) -> Result<Arc<dyn RunJournal>, AgentError> {
        self.inner
            .journal
            .clone()
            .ok_or_else(|| missing_port_error("run journal", run_id))
    }

    pub(crate) fn event_bus_port(
        &self,
        run_id: &RunId,
    ) -> Result<Arc<dyn AgentEventBus>, AgentError> {
        self.inner
            .events
            .clone()
            .ok_or_else(|| missing_port_error("agent event bus", run_id))
    }

    fn event_bus_subscription_port(&self) -> Result<Arc<dyn AgentEventBus>, AgentError> {
        self.inner
            .events
            .clone()
            .ok_or_else(|| missing_runtime_port_error("agent event bus"))
    }

    pub(crate) fn content_port(
        &self,
        run_id: &RunId,
    ) -> Result<Arc<dyn ContentResolver + Send + Sync>, AgentError> {
        self.inner
            .content
            .clone()
            .ok_or_else(|| missing_port_error("content resolver", run_id))
    }

    fn policy_port(&self, run_id: &RunId) -> Result<Arc<dyn RuntimePolicyPort>, AgentError> {
        self.inner
            .policy
            .clone()
            .ok_or_else(|| missing_port_error("runtime policy port", run_id))
    }

    pub(crate) fn seal_terminal_result_from_journal(
        &self,
        record: &JournalRecord,
        output: impl Into<String>,
    ) -> Result<RunResult, AgentError> {
        self.inner
            .run_control
            .seal_terminal_result_from_journal(record, output)
    }

    pub(crate) fn next_journal_seq(&self) -> u64 {
        self.inner.next_journal_seq.fetch_add(1, Ordering::SeqCst) + 1
    }
}

impl Default for AgentRuntime {
    fn default() -> Self {
        AgentRuntimeBuilder::default()
            .build()
            .expect("empty runtime builder is infallible")
    }
}

#[derive(Default)]
pub struct AgentRuntimeBuilder {
    providers: ProviderRegistry,
    package_resolver: Option<Arc<dyn RuntimePackageResolver>>,
    local_packages: Vec<RuntimePackage>,
    default_package_id: Option<RuntimePackageId>,
    journal: Option<Arc<dyn RunJournal>>,
    events: Option<Arc<dyn AgentEventBus>>,
    content: Option<Arc<dyn ContentResolver + Send + Sync>>,
    policy: Option<Arc<dyn RuntimePolicyPort>>,
    output_sinks: OutputSinkRegistry,
}

impl AgentRuntimeBuilder {
    pub fn providers(mut self, providers: ProviderRegistry) -> Self {
        self.providers = providers;
        self
    }

    pub fn provider<P>(
        mut self,
        route_id: impl Into<String>,
        provider: P,
    ) -> Result<Self, AgentError>
    where
        P: ProviderAdapter + 'static,
    {
        self.providers.register(route_id, Arc::new(provider))?;
        Ok(self)
    }

    pub fn package_resolver<R>(mut self, resolver: R) -> Self
    where
        R: RuntimePackageResolver + 'static,
    {
        self.package_resolver = Some(Arc::new(resolver));
        self
    }

    pub fn default_package_id(mut self, package_id: RuntimePackageId) -> Self {
        self.default_package_id = Some(package_id);
        self
    }

    pub fn package(mut self, package: RuntimePackage) -> Self {
        self.local_packages.push(package);
        self
    }

    pub fn default_package(mut self, package: RuntimePackage) -> Self {
        self.default_package_id = Some(package.package_id.clone());
        self.local_packages.push(package);
        self
    }

    pub fn journal<J>(mut self, journal: J) -> Self
    where
        J: RunJournal + 'static,
    {
        self.journal = Some(Arc::new(journal));
        self
    }

    pub fn event_bus<E>(mut self, event_bus: E) -> Self
    where
        E: AgentEventBus + 'static,
    {
        self.events = Some(Arc::new(event_bus));
        self
    }

    pub fn content<C>(mut self, content: C) -> Self
    where
        C: ContentResolver + Send + Sync + 'static,
    {
        self.content = Some(Arc::new(content));
        self
    }

    pub fn policy<P>(mut self, policy: P) -> Self
    where
        P: RuntimePolicyPort + 'static,
    {
        self.policy = Some(Arc::new(policy));
        self
    }

    pub fn output_sink<S>(mut self, sink: S) -> Result<Self, AgentError>
    where
        S: OutputSinkPort + 'static,
    {
        self.output_sinks.register(Arc::new(sink))?;
        Ok(self)
    }

    pub fn build(self) -> Result<AgentRuntime, AgentError> {
        let package_resolver = match (self.package_resolver, self.local_packages.is_empty()) {
            (Some(resolver), _) => Some(resolver),
            (None, true) => None,
            (None, false) => Some(Arc::new(InMemoryRuntimePackageResolver::from_packages(
                self.local_packages,
            )?) as Arc<dyn RuntimePackageResolver>),
        };

        Ok(AgentRuntime {
            inner: Arc::new(RuntimeInner {
                providers: self.providers,
                package_resolver,
                default_package_id: self.default_package_id,
                journal: self.journal,
                events: self.events,
                content: self.content,
                policy: self.policy,
                output_sinks: self.output_sinks,
                run_control: InMemoryRunControlStore::default(),
                next_journal_seq: AtomicU64::new(0),
                runs: Mutex::new(BTreeMap::new()),
            }),
        })
    }
}

struct RuntimeInner {
    providers: ProviderRegistry,
    package_resolver: Option<Arc<dyn RuntimePackageResolver>>,
    default_package_id: Option<RuntimePackageId>,
    journal: Option<Arc<dyn RunJournal>>,
    events: Option<Arc<dyn AgentEventBus>>,
    content: Option<Arc<dyn ContentResolver + Send + Sync>>,
    policy: Option<Arc<dyn RuntimePolicyPort>>,
    output_sinks: OutputSinkRegistry,
    run_control: InMemoryRunControlStore,
    next_journal_seq: AtomicU64,
    runs: Mutex<BTreeMap<RunId, RegisteredRun>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectiveRuntimePackage {
    pub package: RuntimePackage,
    pub fingerprint: RuntimePackageFingerprint,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunSnapshot {
    pub run_id: RunId,
    pub agent_id: AgentId,
    pub source: SourceRef,
    pub status: RunRegistryStatus,
    pub runtime_package_id: RuntimePackageId,
    pub runtime_package_fingerprint: RuntimePackageFingerprint,
    pub provider_route_id: String,
    pub provider_model_id: String,
    pub cancellation_requested: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RunRegistryStatus {
    Registered,
    CancellationRequested,
}

#[derive(Clone, Default)]
pub struct CancellationHandle {
    cancelled: Arc<AtomicBool>,
}

impl CancellationHandle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

impl std::fmt::Debug for CancellationHandle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CancellationHandle")
            .field("cancelled", &self.is_cancelled())
            .finish()
    }
}

#[derive(Clone, Debug)]
struct RegisteredRun {
    run_id: RunId,
    agent_id: AgentId,
    source: SourceRef,
    status: RunRegistryStatus,
    runtime_package_id: RuntimePackageId,
    runtime_package_fingerprint: RuntimePackageFingerprint,
    provider_route_id: String,
    provider_model_id: String,
    cancellation: CancellationHandle,
}

impl RegisteredRun {
    fn snapshot(&self) -> RunSnapshot {
        RunSnapshot {
            run_id: self.run_id.clone(),
            agent_id: self.agent_id.clone(),
            source: self.source.clone(),
            status: self.status.clone(),
            runtime_package_id: self.runtime_package_id.clone(),
            runtime_package_fingerprint: self.runtime_package_fingerprint.clone(),
            provider_route_id: self.provider_route_id.clone(),
            provider_model_id: self.provider_model_id.clone(),
            cancellation_requested: self.cancellation.is_cancelled(),
        }
    }
}

fn missing_port_error(port_name: &str, run_id: &RunId) -> AgentError {
    AgentError::new(
        AgentErrorKind::HostConfigurationNeeded,
        RetryClassification::HostConfigurationNeeded,
        format!("missing required runtime port: {port_name}"),
    )
    .with_causal_ids(CausalIds {
        run_id: Some(run_id.clone()),
        ..CausalIds::default()
    })
}

fn missing_runtime_port_error(port_name: &str) -> AgentError {
    AgentError::new(
        AgentErrorKind::HostConfigurationNeeded,
        RetryClassification::HostConfigurationNeeded,
        format!("missing required runtime port: {port_name}"),
    )
}

fn policy_denial_message(decision: &PolicyDecision) -> String {
    match decision {
        PolicyDecision::Deny { reason }
        | PolicyDecision::Interrupt { reason }
        | PolicyDecision::Defer {
            resume_policy: crate::policy::ResumePolicy { reason, .. },
        } => reason.code.clone(),
        PolicyDecision::Ask { .. } => "policy requested host approval before run start".to_string(),
        PolicyDecision::Modify { .. } => {
            "policy modification is not valid for runtime start".to_string()
        }
        PolicyDecision::Allow { .. } => "policy allowed run start".to_string(),
    }
}

#[derive(Clone)]
struct RuntimeRunControlStore {
    runtime: AgentRuntime,
}

impl RunControlStore for RuntimeRunControlStore {
    fn status(&self, run_id: &RunId) -> Result<RunStatus, AgentError> {
        self.runtime.inner.run_control.status(run_id)
    }

    fn terminal_result(&self, run_id: &RunId) -> Result<Option<RunResult>, AgentError> {
        self.runtime.inner.run_control.terminal_result(run_id)
    }

    fn request_cancel(&self, run_id: &RunId) -> Result<(), AgentError> {
        self.runtime.cancel_run(run_id)
    }
}

#[derive(Clone)]
struct RuntimeRunSubscriptionSource {
    runtime: AgentRuntime,
}

impl RunSubscriptionSource for RuntimeRunSubscriptionSource {
    fn subscribe_all(&self, cursor: Option<EventCursor>) -> Result<AgentEventStream, AgentError> {
        self.runtime.subscribe_all(cursor)
    }

    fn subscribe_run(
        &self,
        run_id: RunId,
        cursor: Option<EventCursor>,
    ) -> Result<AgentEventStream, AgentError> {
        self.runtime.subscribe_run(run_id, cursor)
    }

    fn subscribe_agent(
        &self,
        agent_id: AgentId,
        cursor: Option<EventCursor>,
    ) -> Result<AgentEventStream, AgentError> {
        self.runtime.subscribe_agent(agent_id, cursor)
    }

    fn subscribe_events(
        &self,
        filter: CompiledEventFilter,
        cursor: Option<EventCursor>,
    ) -> Result<AgentEventStream, AgentError> {
        self.runtime.subscribe_events(filter, cursor)
    }

    fn replay_run_from_cursor(
        &self,
        _run_id: RunId,
        _cursor: JournalCursor,
    ) -> Result<AgentEventStream, AgentError> {
        Err(AgentError::host_configuration_needed(
            "run journal replay subscription requires an archive-backed subscription source",
        ))
    }

    fn latest_terminal_event(
        &self,
        run_id: &RunId,
    ) -> Result<Option<crate::event::EventFrame>, AgentError> {
        Ok(self
            .runtime
            .subscribe_run(run_id.clone(), None)?
            .filter(|frame| {
                matches!(
                    frame.event.envelope.event_kind,
                    EventKind::RunCompleted | EventKind::RunFailed | EventKind::RunCancelled
                )
            })
            .last())
    }
}
