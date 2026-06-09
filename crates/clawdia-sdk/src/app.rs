//! Sync-first `AgentApp` facade over the canonical core runtime.

use agent_sdk_core::{
    Agent, AgentError, AgentEventStream, AgentId, AgentPoolStore, AgentRuntime,
    AgentRuntimeBuilder, ApprovalDispatcher, ArchiveCursor, CheckpointStore, CompiledEventFilter,
    ContentResolutionError, ContentResolutionPolicy, ContentResolveRequest, ContentResolver,
    EventArchiveReader, EventCursor, EventFrame, InMemoryAgentEventBus, JournalRecord,
    ProviderAdapter, ProviderArgumentStore, ProviderCapabilities, ProviderRequest,
    ProviderResponse, ProviderRouteSnapshot, ProviderStreamChunk, ProviderUsage, ResolvedContent,
    RunCheckpoint, RunId, RunJournal, RunJournalReader, RunRequest, RunResult, RuntimePackage,
    RuntimePackageId, RuntimePolicyPort, SourceKind, SourceRef, ToolExecutionStore, ToolExecutor,
    ToolPolicyPort, ToolRoute, TypedOutputModel,
};
use std::sync::Arc;

#[cfg(feature = "workspace-tools")]
use agent_sdk_toolkit::{
    InMemoryToolkitContentStore, JsonToolArgumentStore, JsonToolContentStore, ToolArgs, ToolOutput,
    TypedTool,
};

#[cfg(feature = "evals")]
use agent_sdk_core::RunTrace;
#[cfg(feature = "evals")]
use agent_sdk_eval::{CostPolicy, RunReport};

/// Sync-first application facade assembled from canonical Agent SDK ports.
///
/// `AgentApp` owns no alternate execution path. It builds an `AgentRuntime`
/// with caller-supplied ports and lowers ergonomic `run_text`/`run_typed`
/// calls into `RunRequest` values for that runtime.
#[derive(Clone)]
pub struct AgentApp {
    agent: Agent,
    runtime: AgentRuntime,
    default_source: SourceRef,
    stores: Option<AgentAppStores>,
}

#[derive(Clone)]
/// Store bundle accepted by the facade while preserving per-port ownership.
pub struct AgentAppStores {
    pub journal: Arc<dyn RunJournal>,
    pub journal_reader: Arc<dyn RunJournalReader>,
    pub content: Arc<dyn ContentResolver + Send + Sync>,
    pub provider_arguments: Arc<dyn ProviderArgumentStore>,
    pub checkpoint: Option<Arc<dyn CheckpointStore>>,
    pub event_archive: Option<Arc<dyn EventArchiveReader>>,
    pub agent_pool: Option<Arc<dyn AgentPoolStore>>,
    pub tool_execution: Option<Arc<dyn ToolExecutionStore>>,
}

impl AgentAppStores {
    #[cfg(feature = "file-store")]
    /// Creates facade stores backed by the file-store adapter crate.
    pub fn file(root: impl Into<std::path::PathBuf>) -> Self {
        let bundle = agent_sdk_store_file::FileStoreBundle::new(root);
        Self {
            journal: Arc::new(bundle.journal()),
            journal_reader: Arc::new(bundle.journal()),
            content: Arc::new(bundle.content()),
            provider_arguments: Arc::new(bundle.provider_arguments()),
            checkpoint: Some(Arc::new(bundle.checkpoints())),
            event_archive: Some(Arc::new(bundle.event_archive())),
            agent_pool: Some(Arc::new(bundle.agent_pool())),
            tool_execution: Some(Arc::new(bundle.tool_execution())),
        }
    }

    #[cfg(feature = "sqlite-store")]
    /// Creates facade stores backed by the SQLite adapter crate.
    pub fn sqlite(bundle: agent_sdk_store_sqlite::SqliteStoreBundle) -> Result<Self, AgentError> {
        Ok(Self {
            journal: Arc::new(bundle.journal()?),
            journal_reader: Arc::new(bundle.journal()?),
            content: Arc::new(bundle.content()?),
            provider_arguments: Arc::new(bundle.provider_arguments()?),
            checkpoint: Some(Arc::new(bundle.checkpoints()?)),
            event_archive: Some(Arc::new(bundle.event_archive()?)),
            agent_pool: Some(Arc::new(bundle.agent_pool()?)),
            tool_execution: Some(Arc::new(bundle.tool_execution()?)),
        })
    }

    #[cfg(feature = "postgres-store")]
    /// Creates facade stores backed by the Postgres-style adapter crate.
    pub fn postgres(bundle: agent_sdk_store_postgres::PostgresStoreBundle) -> Self {
        Self {
            journal: Arc::new(bundle.journal()),
            journal_reader: Arc::new(bundle.journal()),
            content: Arc::new(bundle.content()),
            provider_arguments: Arc::new(bundle.provider_arguments()),
            checkpoint: Some(Arc::new(bundle.checkpoints())),
            event_archive: Some(Arc::new(bundle.event_archive())),
            agent_pool: Some(Arc::new(bundle.agent_pool())),
            tool_execution: Some(Arc::new(bundle.tool_execution())),
        }
    }

    #[cfg(feature = "supabase-store")]
    /// Creates facade stores backed by the Supabase adapter crate.
    pub fn supabase(bundle: agent_sdk_store_supabase::SupabaseStoreBundle) -> Self {
        Self {
            journal: Arc::new(bundle.journal()),
            journal_reader: Arc::new(bundle.journal()),
            content: Arc::new(bundle.content()),
            provider_arguments: Arc::new(bundle.provider_arguments()),
            checkpoint: Some(Arc::new(bundle.checkpoints())),
            event_archive: Some(Arc::new(bundle.event_archive())),
            agent_pool: Some(Arc::new(bundle.agent_pool())),
            tool_execution: None,
        }
    }
}

#[derive(Clone, Debug)]
#[non_exhaustive]
/// Read-only evidence snapshot for one run assembled through existing
/// `AgentApp` ports.
///
/// Fields stay separated by source on purpose: live event frames are buffered
/// observation, archived frames are an optional event projection, journal
/// records are durable run truth, and checkpoints are resume accelerators.
/// Constructing this value does not publish events, append journals, or write
/// checkpoints.
pub struct AgentAppRunEvidence {
    /// Run this evidence snapshot was collected for.
    pub run_id: RunId,
    /// Buffered live frames collected from the canonical runtime event bus.
    pub live_event_frames: Vec<EventFrame>,
    /// Archived frames for this run from the optional event archive reader.
    pub archived_event_frames: Vec<EventFrame>,
    /// Durable records read through `RunJournalReader`.
    pub journal_records: Vec<JournalRecord>,
    /// Latest checkpoint accelerator from the optional checkpoint store.
    pub latest_checkpoint: Option<RunCheckpoint>,
}

impl AgentApp {
    /// Starts an app builder for one agent.
    pub fn builder(agent: Agent) -> AgentAppBuilder {
        AgentAppBuilder {
            agent,
            runtime: AgentRuntime::builder().event_bus(InMemoryAgentEventBus::default()),
            package: None,
            #[cfg(feature = "workspace-tools")]
            package_bundles: Vec::new(),
            provider_route_id: "provider.fake".to_string(),
            provider_model_id: "model.fake".to_string(),
            default_source: None,
            stores: None,
            provider_arguments: None,
            #[cfg(feature = "workspace-tools")]
            tool_output_store: Arc::new(InMemoryToolkitContentStore::default()),
        }
    }

    /// Returns the agent used by this app.
    pub fn agent(&self) -> &Agent {
        &self.agent
    }

    /// Returns the canonical runtime assembled by this app.
    pub fn runtime(&self) -> &AgentRuntime {
        &self.runtime
    }

    /// Returns the store bundle supplied to the facade, if one was configured.
    ///
    /// The bundle is exposed as typed ports only. It is not facade-owned
    /// session state, and helpers that need durable evidence still read through
    /// the specific reader port for that evidence.
    pub fn stores(&self) -> Option<&AgentAppStores> {
        self.stores.as_ref()
    }

    /// Runs a text request through the canonical runtime.
    pub fn run_text(
        &self,
        run_id: RunId,
        input: impl Into<String>,
    ) -> Result<RunResult, AgentError> {
        self.run_request(RunRequest::text(
            run_id,
            self.agent.id().clone(),
            self.default_source.clone(),
            input,
        ))
    }

    /// Runs a typed request through the canonical runtime and output contract path.
    pub fn run_typed<T: TypedOutputModel>(
        &self,
        run_id: RunId,
        input: impl Into<String>,
    ) -> Result<RunResult, AgentError> {
        self.runtime.run_typed::<T>(RunRequest::text(
            run_id,
            self.agent.id().clone(),
            self.default_source.clone(),
            input,
        ))
    }

    /// Runs an explicit request through the canonical runtime.
    pub fn run_request(&self, request: RunRequest) -> Result<RunResult, AgentError> {
        self.runtime.run_text(request)
    }

    /// Subscribes to events for one run through the canonical runtime event bus.
    pub fn subscribe_run(
        &self,
        run_id: RunId,
        cursor: Option<EventCursor>,
    ) -> Result<AgentEventStream, AgentError> {
        self.runtime.subscribe_run(run_id, cursor)
    }

    /// Subscribes to filtered events through the canonical runtime event bus.
    pub fn subscribe_filtered(
        &self,
        filter: CompiledEventFilter,
        cursor: Option<EventCursor>,
    ) -> Result<AgentEventStream, AgentError> {
        self.runtime.subscribe_events(filter, cursor)
    }

    /// Collects currently buffered live frames for one run through the
    /// canonical runtime event bus.
    ///
    /// This is live observation, not durable truth. Use
    /// `journal_records_for_run` for durable replay/report evidence and
    /// `archived_event_frames` for configured archive reads.
    pub fn event_frames_for_run(
        &self,
        run_id: RunId,
        cursor: Option<EventCursor>,
    ) -> Result<Vec<EventFrame>, AgentError> {
        Ok(self.subscribe_run(run_id, cursor)?.collect())
    }

    /// Reads durable journal records for one run through `RunJournalReader`.
    pub fn journal_records_for_run(
        &self,
        run_id: &RunId,
    ) -> Result<Vec<JournalRecord>, AgentError> {
        self.required_stores("journal_records_for_run")?
            .journal_reader
            .records_for_run(run_id)
    }

    /// Reads archived event frames through `EventArchiveReader` when the
    /// configured store bundle supplies one.
    ///
    /// Archived frames are still an event projection. They do not replace the
    /// run journal as durable truth.
    pub fn archived_event_frames(
        &self,
        cursor: Option<ArchiveCursor>,
    ) -> Result<Vec<EventFrame>, AgentError> {
        let stores = self.required_stores("archived_event_frames")?;
        let archive = stores.event_archive.as_ref().ok_or_else(|| {
            AgentError::host_configuration_needed(
                "AgentApp archived_event_frames requires AgentAppStores event archive",
            )
        })?;
        archive.frames_after(cursor)
    }

    /// Collects the common read-only evidence needed for e2e tests and
    /// diagnostics for one run.
    ///
    /// This helper keeps each evidence source in a separate field. Missing
    /// optional archive or checkpoint ports produce empty archive evidence or
    /// no checkpoint respectively, while missing `AgentAppStores` remains a
    /// host-configuration error because durable journal truth is required.
    pub fn run_evidence(&self, run_id: &RunId) -> Result<AgentAppRunEvidence, AgentError> {
        let stores = self.required_stores("run_evidence")?;
        let live_event_frames = self.event_frames_for_run(run_id.clone(), None)?;
        let journal_records = stores.journal_reader.records_for_run(run_id)?;
        let archived_event_frames = stores
            .event_archive
            .as_ref()
            .map(|archive| {
                archive.frames_after(None).map(|frames| {
                    frames
                        .into_iter()
                        .filter(|frame| &frame.event.envelope.run_id == run_id)
                        .collect()
                })
            })
            .transpose()?
            .unwrap_or_default();
        let latest_checkpoint = stores
            .checkpoint
            .as_ref()
            .map(|checkpoint| checkpoint.load_latest(run_id))
            .transpose()?
            .flatten();

        Ok(AgentAppRunEvidence {
            run_id: run_id.clone(),
            live_event_frames,
            archived_event_frames,
            journal_records,
            latest_checkpoint,
        })
    }

    /// Loads the latest checkpoint accelerator for a run when one is
    /// configured.
    ///
    /// Checkpoints are resume accelerators. They do not create or replace
    /// journal truth.
    pub fn latest_checkpoint(&self, run_id: &RunId) -> Result<Option<RunCheckpoint>, AgentError> {
        let stores = self.required_stores("latest_checkpoint")?;
        let checkpoint = stores.checkpoint.as_ref().ok_or_else(|| {
            AgentError::host_configuration_needed(
                "AgentApp latest_checkpoint requires AgentAppStores checkpoint store",
            )
        })?;
        checkpoint.load_latest(run_id)
    }

    /// Builds a run report from durable journal records.
    #[cfg(feature = "evals")]
    pub fn run_report<'a>(
        &self,
        run_id: &RunId,
        records: impl IntoIterator<Item = &'a JournalRecord>,
        cost_policy: Option<&dyn CostPolicy>,
    ) -> Result<RunReport, AgentError> {
        let trace = RunTrace::from_records(run_id, records);
        RunReport::from_run_trace(&trace, cost_policy)
    }

    /// Builds a run report from the configured durable journal reader.
    #[cfg(feature = "evals")]
    pub fn run_report_from_stores(
        &self,
        run_id: &RunId,
        cost_policy: Option<&dyn CostPolicy>,
    ) -> Result<RunReport, AgentError> {
        let records = self.journal_records_for_run(run_id)?;
        self.run_report(run_id, records.iter(), cost_policy)
    }

    /// Builds a run report from an already collected evidence snapshot.
    ///
    /// Reports are derived from durable journal records only. Live and
    /// archived event frames stay observable projections, not report truth.
    #[cfg(feature = "evals")]
    pub fn run_report_from_evidence(
        &self,
        evidence: &AgentAppRunEvidence,
        cost_policy: Option<&dyn CostPolicy>,
    ) -> Result<RunReport, AgentError> {
        self.run_report(
            &evidence.run_id,
            evidence.journal_records.iter(),
            cost_policy,
        )
    }

    fn required_stores(&self, helper: &str) -> Result<&AgentAppStores, AgentError> {
        self.stores.as_ref().ok_or_else(|| {
            AgentError::host_configuration_needed(format!(
                "AgentApp {helper} requires AgentAppStores"
            ))
        })
    }
}

/// Builder for `AgentApp`.
pub struct AgentAppBuilder {
    agent: Agent,
    runtime: AgentRuntimeBuilder,
    package: Option<RuntimePackage>,
    #[cfg(feature = "workspace-tools")]
    package_bundles: Vec<agent_sdk_toolkit::ToolkitPackBundle>,
    provider_route_id: String,
    provider_model_id: String,
    default_source: Option<SourceRef>,
    stores: Option<AgentAppStores>,
    provider_arguments: Option<Arc<dyn ProviderArgumentStore>>,
    #[cfg(feature = "workspace-tools")]
    tool_output_store: Arc<dyn JsonToolContentStore>,
}

impl AgentAppBuilder {
    /// Uses an explicit runtime package.
    pub fn package(mut self, package: RuntimePackage) -> Self {
        self.package = Some(package);
        self
    }

    /// Sets the inferred runtime package provider route.
    pub fn provider_route(
        mut self,
        route_id: impl Into<String>,
        model_id: impl Into<String>,
    ) -> Self {
        self.provider_route_id = route_id.into();
        self.provider_model_id = model_id.into();
        self
    }

    /// Registers one provider adapter with the canonical runtime builder.
    pub fn provider<P>(
        mut self,
        route_id: impl Into<String>,
        provider: P,
    ) -> Result<Self, AgentError>
    where
        P: ProviderAdapter + 'static,
    {
        self.runtime = self.runtime.provider(route_id, provider)?;
        Ok(self)
    }

    /// Registers one shared provider adapter with the canonical runtime builder.
    pub fn shared_provider(
        mut self,
        route_id: impl Into<String>,
        provider: Arc<dyn ProviderAdapter>,
    ) -> Result<Self, AgentError> {
        self.runtime = self
            .runtime
            .provider(route_id, SharedProviderAdapter(provider))?;
        Ok(self)
    }

    /// Configures the runtime journal port.
    pub fn journal<J>(mut self, journal: J) -> Self
    where
        J: agent_sdk_core::RunJournal + 'static,
    {
        self.runtime = self.runtime.journal(journal);
        self
    }

    /// Configures the runtime event bus.
    pub fn event_bus<E>(mut self, event_bus: E) -> Self
    where
        E: agent_sdk_core::AgentEventBus + 'static,
    {
        self.runtime = self.runtime.event_bus(event_bus);
        self
    }

    /// Configures the runtime content resolver.
    pub fn content<C>(mut self, content: C) -> Self
    where
        C: ContentResolver + Send + Sync + 'static,
    {
        self.runtime = self.runtime.content(content);
        self
    }

    /// Configures the runtime start policy.
    pub fn policy<P>(mut self, policy: P) -> Self
    where
        P: RuntimePolicyPort + 'static,
    {
        self.runtime = self.runtime.policy(policy);
        self
    }

    /// Adds one runtime tool route.
    pub fn tool_route(mut self, route: ToolRoute) -> Self {
        self.runtime = self.runtime.tool_route(route);
        self
    }

    /// Adds one runtime tool executor.
    pub fn tool_executor(mut self, executor: Arc<dyn ToolExecutor>) -> Result<Self, AgentError> {
        self.runtime = self.runtime.tool_executor(executor)?;
        Ok(self)
    }

    /// Configures the runtime tool policy.
    pub fn tool_policy<P>(mut self, policy: P) -> Self
    where
        P: ToolPolicyPort + 'static,
    {
        self.runtime = self.runtime.tool_policy(policy);
        self
    }

    /// Configures the host-owned approval dispatcher used by approval-gated
    /// tool execution.
    pub fn approval_dispatcher<D>(mut self, dispatcher: D) -> Self
    where
        D: ApprovalDispatcher + 'static,
    {
        self.runtime = self.runtime.approval_dispatcher(dispatcher);
        self
    }

    /// Configures shared store adapters and wires runtime-owned ports.
    pub fn stores(mut self, stores: AgentAppStores) -> Self {
        self.runtime = self
            .runtime
            .journal(SharedRunJournal(stores.journal.clone()))
            .content(SharedContentResolver(stores.content.clone()));
        self.provider_arguments = Some(stores.provider_arguments.clone());
        self.stores = Some(stores);
        self
    }

    /// Adds a typed tool through the toolkit helper path.
    #[cfg(feature = "workspace-tools")]
    pub fn typed_tool<A, R>(mut self, tool: TypedTool<A, R>) -> Result<Self, AgentError>
    where
        A: ToolArgs,
        R: ToolOutput,
    {
        let argument_store = self.provider_arguments.clone().ok_or_else(|| {
            AgentError::host_configuration_needed(
                "AgentApp typed tools require AgentAppStores provider arguments",
            )
        })?;
        let source = self.default_source.clone().unwrap_or_else(|| {
            SourceRef::with_kind(SourceKind::Sdk, "source.sdk.agent_app.typed_tools")
        });
        let bundle = tool.pack_bundle(source)?;
        for route in &bundle.routes {
            self.runtime = self.runtime.tool_route(route.clone());
        }
        self.runtime = self.runtime.tool_executor(tool.executor(
            Arc::new(ProviderArgumentJsonStore(argument_store)),
            self.tool_output_store.clone(),
        ))?;
        self.package_bundles.push(bundle);
        Ok(self)
    }

    /// Sets the default source used by `run_text` and `run_typed`.
    pub fn default_source(mut self, source: SourceRef) -> Self {
        self.default_source = Some(source);
        self
    }

    /// Builds the app and underlying canonical runtime.
    pub fn build(self) -> Result<AgentApp, AgentError> {
        let agent_id = self.agent.id().clone();
        let package = self.package.unwrap_or_else(|| {
            let builder = RuntimePackage::builder(RuntimePackageId::new(format!(
                "package.{}",
                agent_id.as_str()
            )))
            .agent(self.agent.snapshot())
            .provider_route(ProviderRouteSnapshot::new(
                self.provider_route_id,
                self.provider_model_id,
            ));
            #[cfg(feature = "workspace-tools")]
            let builder = {
                let mut builder = builder;
                for bundle in &self.package_bundles {
                    builder = bundle.install_into(builder);
                }
                builder
            };
            builder
                .build()
                .expect("inferred AgentApp runtime package is valid")
        });
        if package.agent.agent_id != agent_id {
            return Err(AgentError::new(
                agent_sdk_core::AgentErrorKind::InvalidPackage,
                agent_sdk_core::RetryClassification::HostConfigurationNeeded,
                "AgentApp package agent must match the app agent",
            ));
        }
        let stores = self.stores.clone();
        let runtime = self.runtime.default_package(package).build()?;
        let default_source = self.default_source.unwrap_or_else(|| {
            SourceRef::with_kind(
                SourceKind::Host,
                format!("source.agent_app.{}", agent_id.as_str()),
            )
        });
        Ok(AgentApp {
            agent: self.agent,
            runtime,
            default_source,
            stores,
        })
    }
}

#[derive(Clone)]
struct SharedProviderAdapter(Arc<dyn ProviderAdapter>);

impl ProviderAdapter for SharedProviderAdapter {
    fn capabilities(&self) -> ProviderCapabilities {
        self.0.capabilities()
    }

    fn complete(&self, request: &ProviderRequest) -> Result<ProviderResponse, AgentError> {
        self.0.complete(request)
    }

    fn stream(&self, request: &ProviderRequest) -> Result<Vec<ProviderStreamChunk>, AgentError> {
        self.0.stream(request)
    }

    fn extract_usage(&self, response: &ProviderResponse) -> ProviderUsage {
        self.0.extract_usage(response)
    }
}

impl Default for AgentAppBuilder {
    fn default() -> Self {
        Self {
            agent: Agent::builder()
                .id(AgentId::new("agent.app.default"))
                .name("agent app")
                .build()
                .expect("default AgentApp agent is valid"),
            runtime: AgentRuntime::builder().event_bus(InMemoryAgentEventBus::default()),
            package: None,
            #[cfg(feature = "workspace-tools")]
            package_bundles: Vec::new(),
            provider_route_id: "provider.fake".to_string(),
            provider_model_id: "model.fake".to_string(),
            default_source: None,
            stores: None,
            provider_arguments: None,
            #[cfg(feature = "workspace-tools")]
            tool_output_store: Arc::new(InMemoryToolkitContentStore::default()),
        }
    }
}

#[derive(Clone)]
struct SharedRunJournal(Arc<dyn RunJournal>);

impl RunJournal for SharedRunJournal {
    fn append(
        &self,
        record: agent_sdk_core::JournalRecord,
    ) -> Result<agent_sdk_core::JournalCursor, AgentError> {
        self.0.append(record)
    }
}

#[derive(Clone)]
struct SharedContentResolver(Arc<dyn ContentResolver + Send + Sync>);

impl ContentResolver for SharedContentResolver {
    fn resolve(
        &self,
        request: ContentResolveRequest,
        policy: ContentResolutionPolicy,
    ) -> Result<ResolvedContent, ContentResolutionError> {
        self.0.resolve(request, policy)
    }

    fn store_resolved_content(
        &self,
        content_ref: &agent_sdk_core::content::ContentRef,
        bytes: Vec<u8>,
    ) -> Result<(), ContentResolutionError> {
        self.0.store_resolved_content(content_ref, bytes)
    }
}

#[cfg(feature = "workspace-tools")]
struct ProviderArgumentJsonStore(Arc<dyn ProviderArgumentStore>);

#[cfg(feature = "workspace-tools")]
impl JsonToolArgumentStore for ProviderArgumentJsonStore {
    fn load_json(
        &self,
        content_ref: &agent_sdk_core::domain::ContentRef,
    ) -> Result<agent_sdk_toolkit::serde_json::Value, AgentError> {
        self.0.load_provider_arguments_json(content_ref)
    }
}
