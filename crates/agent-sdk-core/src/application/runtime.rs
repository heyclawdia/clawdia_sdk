//! Runtime coordination for starting, observing, and completing runs. Use this module
//! when a host wires providers, package resolution, policy, journals, and event
//! streams into the core loop. Runtime methods may call configured adapters, mutate
//! run state, append journals, and publish events.
//!
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
        SessionId, SourceRef, TurnId,
    },
    error::CausalIds,
    event::{CompiledEventFilter, EventCursor, EventKind},
    event_bus::{AgentEventBus, AgentEventStream},
    hook_ports::{HookExecutorRegistry, InMemoryHookExecutorRegistry},
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
    tool_execution::ToolExecutionCoordinator,
    tool_ports::{
        ToolExecutor, ToolExecutorRegistry, ToolPolicyPort, ToolRegistrySnapshot, ToolRoute,
        ToolRouter,
    },
};

#[derive(Clone)]
/// Holds agent runtime application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct AgentRuntime {
    inner: Arc<RuntimeInner>,
}

impl AgentRuntime {
    /// Starts a builder for this application::runtime value. Building
    /// is data-only; runtime side effects occur only when a later
    /// coordinator or host port executes the built configuration.
    pub fn builder() -> AgentRuntimeBuilder {
        AgentRuntimeBuilder::default()
    }

    /// Registers a run with the runtime and returns a handle for control and
    /// subscription.
    /// This resolves package, provider, journal, event, content, and policy
    /// ports, evaluates start policy, mutates the run registry, and registers
    /// run control; it does not call the provider model.
    pub fn start_run(&self, request: RunRequest) -> Result<RunHandle, AgentError> {
        let _journal = self.journal_port(&request.run_id)?;
        let _events = self.event_bus_port(&request.run_id)?;
        let _content = self.content_port(&request.run_id)?;
        let policy = self.policy_port(&request.run_id)?;

        let effective = self.resolve_effective_package(&request)?;
        self.provider_for(&effective.package, &request.run_id)?;
        crate::hooks::validate_package_hooks(
            &effective.package.hooks,
            self.inner.hook_registry.as_ref(),
        )
        .map_err(|error| {
            error.with_causal_ids(CausalIds {
                run_id: Some(request.run_id.clone()),
                ..CausalIds::default()
            })
        })?;
        self.evaluate_run_start_policy(policy.as_ref(), &request, &effective.package)?;

        let cancellation = CancellationHandle::new();
        let entry = RegisteredRun {
            run_id: request.run_id.clone(),
            session_id: request.session_id.clone(),
            turn_id: request.turn_id.clone(),
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

    /// Runs a P0 text request to completion through the configured runtime.
    /// This registers the run, calls the P0 loop driver, and may use provider,
    /// journal, content, event, validation, and policy ports selected by the
    /// resolved package.
    pub fn run_text(&self, request: RunRequest) -> Result<RunResult, AgentError> {
        let handle = self.start_run(request.clone())?;
        crate::loop_driver::run_p0_text(self, request, handle)
    }

    /// Runs a typed request by attaching the model's output contract and using
    /// the same runtime path as `run_text`.
    /// Validation and repair side effects remain on the canonical P1 output
    /// pipeline; this helper does not create a parallel typed-output path.
    pub fn run_typed<T: crate::typed_output_ports::TypedOutputModel>(
        &self,
        request: RunRequest,
    ) -> Result<RunResult, AgentError> {
        self.run_text(request.with_output_contract(crate::output::OutputContract::for_type::<T>()))
    }

    /// Resolve effective package.
    /// This reads configured runtime package state, applies request-level tightening, and
    /// computes the package fingerprint; it does not call provider or tool executors.
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

    /// Cancel run.
    /// This marks the registered run as cancellation requested and forwards the request to run
    /// control; actual adapter cleanup happens in the owning control path.
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

    /// Returns run snapshot for callers that need to inspect the contract state.
    /// This reads the in-memory run registry and returns a snapshot without mutating run state.
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

    /// Returns registered run count for callers that need to inspect the contract state.
    /// This reads the in-memory run registry length without starting, cancelling, or replaying
    /// runs.
    pub fn registered_run_count(&self) -> Result<usize, AgentError> {
        Ok(self
            .inner
            .runs
            .lock()
            .map_err(|_| AgentError::contract_violation("run registry lock poisoned"))?
            .len())
    }

    /// Returns the configured event bus as a subscription source.
    /// This retrieves the port so callers can subscribe; it does not publish events or drive a
    /// run.
    pub fn events(&self) -> Result<Arc<dyn AgentEventBus>, AgentError> {
        self.event_bus_subscription_port()
    }

    /// Subscribe all.
    /// This delegates to the configured event bus to create a read-only stream for all visible
    /// events.
    pub fn subscribe_all(
        &self,
        cursor: Option<EventCursor>,
    ) -> Result<AgentEventStream, AgentError> {
        self.event_bus_subscription_port()?.subscribe_all(cursor)
    }

    /// Subscribe run.
    /// This delegates to the configured event bus to create a read-only stream scoped to one
    /// run.
    pub fn subscribe_run(
        &self,
        run_id: RunId,
        cursor: Option<EventCursor>,
    ) -> Result<AgentEventStream, AgentError> {
        self.event_bus_subscription_port()?
            .subscribe_run(run_id, cursor)
    }

    /// Subscribe agent.
    /// This delegates to the event-bus subscription port to create a read-only stream for
    /// matching agent events.
    pub fn subscribe_agent(
        &self,
        agent_id: AgentId,
        cursor: Option<EventCursor>,
    ) -> Result<AgentEventStream, AgentError> {
        self.event_bus_subscription_port()?
            .subscribe_agent(agent_id, cursor)
    }

    /// Subscribe events.
    /// This delegates to the event-bus subscription port to create a read-only filtered stream.
    pub fn subscribe_events(
        &self,
        filter: CompiledEventFilter,
        cursor: Option<EventCursor>,
    ) -> Result<AgentEventStream, AgentError> {
        self.event_bus_subscription_port()?
            .subscribe_filtered(filter, cursor)
    }

    /// Returns provider registry for callers that need to inspect the contract state.
    /// This returns the configured provider registry reference; callers must still use
    /// policy-checked runtime paths to execute providers.
    pub fn provider_registry(&self) -> &ProviderRegistry {
        &self.inner.providers
    }

    /// Returns the output sinks currently held by this value.
    /// This returns the configured sink registry; sending remains owned by output-delivery
    /// paths.
    pub fn output_sinks(&self) -> &OutputSinkRegistry {
        &self.inner.output_sinks
    }

    /// Builds the tool execution coordinator for the effective package.
    /// This validates configured tool routes against the runtime package and
    /// returns a coordinator over the shared tool router, policy, journal,
    /// and effect spine. It does not execute a tool.
    pub(crate) fn tool_execution_coordinator(
        &self,
        package: &RuntimePackage,
        run_id: &RunId,
    ) -> Result<ToolExecutionCoordinator, AgentError> {
        if self.inner.tool_routes.is_empty() {
            return Err(missing_port_error("tool routes", run_id));
        }
        let snapshot =
            ToolRegistrySnapshot::from_runtime_package(package, self.inner.tool_routes.clone())
                .map_err(|error| {
                    error.with_causal_ids(CausalIds {
                        run_id: Some(run_id.clone()),
                        ..CausalIds::default()
                    })
                })?;
        let coordinator = ToolExecutionCoordinator::new(
            ToolRouter::new(snapshot),
            self.inner.tool_executors.clone(),
        );
        Ok(match self.inner.tool_policy.clone() {
            Some(policy) => coordinator.with_policy(policy),
            None => coordinator,
        })
    }

    /// Returns hook executor registry for application coordinators.
    /// This retrieves the configured port without invoking hook executors.
    pub(crate) fn hook_registry_port(&self) -> Arc<dyn HookExecutorRegistry> {
        self.inner.hook_registry.clone()
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

    /// Returns the provider adapter configured for the runtime package route.
    /// This is a registry lookup only; the provider call happens later in the loop driver.
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

    /// Returns the provider adapter configured for a route id.
    /// This is a registry lookup only; it does not send a model request.
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

    /// Returns the journal port currently held by this value.
    /// This returns the journal port selected for the run and does not append a record.
    pub(crate) fn journal_port(&self, run_id: &RunId) -> Result<Arc<dyn RunJournal>, AgentError> {
        self.inner
            .journal
            .clone()
            .ok_or_else(|| missing_port_error("run journal", run_id))
    }

    /// Returns the event bus port selected for the run.
    /// This retrieves the configured port without publishing an event.
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

    /// Returns the content port currently held by this value.
    /// This returns the content resolver selected for the run and does not resolve raw content.
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

    /// Seals run-control terminal state from a journal terminal record.
    /// This delegates to the run-control store and may mutate handle state; it does not append a
    /// new journal record or publish an event.
    pub(crate) fn seal_terminal_result_from_journal(
        &self,
        record: &JournalRecord,
        output: impl Into<String>,
    ) -> Result<RunResult, AgentError> {
        self.inner
            .run_control
            .seal_terminal_result_from_journal(record, output)
    }

    /// Allocates the next in-memory journal sequence number for this runtime.
    /// This advances an atomic counter; the caller is responsible for appending the record.
    pub(crate) fn next_journal_seq(&self) -> u64 {
        let _guard = self
            .inner
            .journal_sequence_lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        self.inner.next_journal_seq.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Reserves a contiguous journal sequence block for records appended by a coordinator.
    /// This does not append records or call host-controlled code.
    pub(crate) fn reserve_journal_seq_block(&self, width: u64) -> u64 {
        debug_assert!(width > 0);
        let _guard = self
            .inner
            .journal_sequence_lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        self.inner
            .next_journal_seq
            .fetch_add(width, Ordering::SeqCst)
            + 1
    }

    /// Returns the next journal sequence number without reserving it.
    /// This is used by coordinators that may or may not append records.
    pub(crate) fn next_journal_seq_hint(&self) -> u64 {
        self.inner.next_journal_seq.load(Ordering::SeqCst) + 1
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
/// Holds agent runtime builder application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
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
    hook_registry: Option<Arc<dyn HookExecutorRegistry>>,
    tool_routes: Vec<ToolRoute>,
    tool_executors: ToolExecutorRegistry,
    tool_policy: Option<Arc<dyn ToolPolicyPort>>,
}

impl AgentRuntimeBuilder {
    /// Returns an updated value with providers configured.
    /// This stores a provider registry in the builder and performs no provider calls.
    pub fn providers(mut self, providers: ProviderRegistry) -> Self {
        self.providers = providers;
        self
    }

    /// Returns an updated value with provider configured.
    /// This adds one provider adapter to the builder registry and performs no provider calls.
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

    /// Returns an updated value with package resolver configured.
    /// This is builder configuration only; it stores the resolver for future run starts and
    /// performs no I/O.
    pub fn package_resolver<R>(mut self, resolver: R) -> Self
    where
        R: RuntimePackageResolver + 'static,
    {
        self.package_resolver = Some(Arc::new(resolver));
        self
    }

    /// Returns an updated value with default package id configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn default_package_id(mut self, package_id: RuntimePackageId) -> Self {
        self.default_package_id = Some(package_id);
        self
    }

    /// Returns an updated value with package configured.
    /// This reads or configures runtime state without executing a provider or tool.
    pub fn package(mut self, package: RuntimePackage) -> Self {
        self.local_packages.push(package);
        self
    }

    /// Returns an updated value with default package configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn default_package(mut self, package: RuntimePackage) -> Self {
        self.default_package_id = Some(package.package_id.clone());
        self.local_packages.push(package);
        self
    }

    /// Returns an updated value with journal configured.
    /// This reads or configures runtime state without executing a provider or tool.
    pub fn journal<J>(mut self, journal: J) -> Self
    where
        J: RunJournal + 'static,
    {
        self.journal = Some(Arc::new(journal));
        self
    }

    /// Returns an updated value with event bus configured.
    /// This stores the event-bus port in the builder and does not publish events.
    pub fn event_bus<E>(mut self, event_bus: E) -> Self
    where
        E: AgentEventBus + 'static,
    {
        self.events = Some(Arc::new(event_bus));
        self
    }

    /// Returns an updated value with content configured.
    /// This reads or configures runtime state without executing a provider or tool.
    pub fn content<C>(mut self, content: C) -> Self
    where
        C: ContentResolver + Send + Sync + 'static,
    {
        self.content = Some(Arc::new(content));
        self
    }

    /// Returns policy for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn policy<P>(mut self, policy: P) -> Self
    where
        P: RuntimePolicyPort + 'static,
    {
        self.policy = Some(Arc::new(policy));
        self
    }

    /// Returns output sink for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn output_sink<S>(mut self, sink: S) -> Result<Self, AgentError>
    where
        S: OutputSinkPort + 'static,
    {
        self.output_sinks.register(Arc::new(sink))?;
        Ok(self)
    }

    /// Adds one tool route to the runtime's app-facing tool execution
    /// configuration. The route is validated against the effective runtime
    /// package only when a model-requested tool call is lowered.
    pub fn tool_route(mut self, route: ToolRoute) -> Self {
        self.tool_routes.push(route);
        self
    }

    /// Replaces the runtime's configured tool routes. This is builder
    /// configuration only and does not execute or resolve tools.
    pub fn tool_routes(mut self, routes: impl IntoIterator<Item = ToolRoute>) -> Self {
        self.tool_routes = routes.into_iter().collect();
        self
    }

    /// Replaces the runtime's configured tool executor registry. This is
    /// builder configuration only and does not execute tools.
    pub fn tool_executors(mut self, executors: ToolExecutorRegistry) -> Self {
        self.tool_executors = executors;
        self
    }

    /// Adds one tool executor to the runtime's configured executor
    /// registry. This stores the executor behind the public port and does
    /// not execute it.
    pub fn tool_executor(mut self, executor: Arc<dyn ToolExecutor>) -> Result<Self, AgentError> {
        self.tool_executors.register(executor)?;
        Ok(self)
    }

    /// Configures the runtime's tool policy port. This is builder
    /// configuration only and does not evaluate policy.
    pub fn tool_policy<P>(mut self, policy: P) -> Self
    where
        P: ToolPolicyPort + 'static,
    {
        self.tool_policy = Some(Arc::new(policy));
        self
    }

    /// Returns hook executor registry for the current value.
    /// This stores the registry for future run starts and hook invocation; it does not invoke
    /// hook executors.
    pub fn hook_executor_registry<R>(mut self, registry: R) -> Self
    where
        R: HookExecutorRegistry + 'static,
    {
        self.hook_registry = Some(Arc::new(registry));
        self
    }

    /// Finishes builder validation and returns the configured value.
    /// This is data-only unless the surrounding builder explicitly
    /// documents adapter or store access.
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
                hook_registry: self
                    .hook_registry
                    .unwrap_or_else(|| Arc::new(InMemoryHookExecutorRegistry::default())),
                tool_routes: self.tool_routes,
                tool_executors: self.tool_executors,
                tool_policy: self.tool_policy,
                run_control: InMemoryRunControlStore::default(),
                next_journal_seq: AtomicU64::new(0),
                journal_sequence_lock: Mutex::new(()),
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
    hook_registry: Arc<dyn HookExecutorRegistry>,
    tool_routes: Vec<ToolRoute>,
    tool_executors: ToolExecutorRegistry,
    tool_policy: Option<Arc<dyn ToolPolicyPort>>,
    run_control: InMemoryRunControlStore,
    next_journal_seq: AtomicU64,
    journal_sequence_lock: Mutex<()>,
    runs: Mutex<BTreeMap<RunId, RegisteredRun>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Holds effective runtime package application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct EffectiveRuntimePackage {
    /// Package used by this record or request.
    pub package: RuntimePackage,
    /// Deterministic fingerprint for package, event, telemetry, or validation
    /// evidence.
    pub fingerprint: RuntimePackageFingerprint,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Holds run snapshot application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct RunSnapshot {
    /// Run identifier used for lineage, filtering, replay, and dedupe.
    pub run_id: RunId,
    /// Optional host-provided session identifier for grouping related turns.
    pub session_id: Option<SessionId>,
    /// Optional host-provided turn identifier for this run.
    pub turn_id: Option<TurnId>,
    /// Agent identifier used for lineage, filtering, and ownership checks.
    pub agent_id: AgentId,
    /// Source label or ref for this item; it is metadata and does not fetch
    /// content by itself.
    pub source: SourceRef,
    /// Finite status for this record or lifecycle stage.
    pub status: RunRegistryStatus,
    /// Stable runtime package id used for typed lineage, lookup, or dedupe.
    pub runtime_package_id: RuntimePackageId,
    /// Fingerprint of the runtime package snapshot in force when this value was produced.
    /// Use it for replay, dedupe, and package-lineage checks; the field is evidence and does
    /// not execute package behavior.
    pub runtime_package_fingerprint: RuntimePackageFingerprint,
    /// Stable provider route id used for typed lineage, lookup, or dedupe.
    pub provider_route_id: String,
    /// Stable provider model id used for typed lineage, lookup, or dedupe.
    pub provider_model_id: String,
    /// Whether cancellation requested is enabled.
    /// Policy, validation, or routing code uses this flag to choose the explicit behavior.
    pub cancellation_requested: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Enumerates the finite run registry status cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum RunRegistryStatus {
    /// Use this variant when the contract needs to represent registered; selecting it has no side effect by itself.
    Registered,
    /// Use this variant when the contract needs to represent cancellation requested; selecting it has no side effect by itself.
    CancellationRequested,
}

#[derive(Clone, Default)]
/// Holds cancellation handle application-layer state or configuration.
/// Use it with the documented coordinator methods; run, journal, event, provider, or port effects are called out on those methods rather than on construction.
pub struct CancellationHandle {
    cancelled: Arc<AtomicBool>,
}

impl CancellationHandle {
    /// Creates a new application::runtime value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new() -> Self {
        Self::default()
    }

    /// Cancel.
    /// This flips the cancellation token in memory; callers still need the owning run path to
    /// observe and apply cancellation.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Reports whether this value is cancelled. The check is pure and
    /// does not mutate SDK or host state.
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
    session_id: Option<SessionId>,
    turn_id: Option<TurnId>,
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
            session_id: self.session_id.clone(),
            turn_id: self.turn_id.clone(),
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
