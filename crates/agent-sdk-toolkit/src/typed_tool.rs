//! Typed tool authoring helpers layered over core tool execution ports.
//! Helpers in this module build tool declarations and executors; execution,
//! policy, approval, journals, events, and recovery remain owned by
//! `agent-sdk-core`.

use std::{future::Future, marker::PhantomData, pin::Pin, sync::Arc};

use agent_sdk_core::{
    AgentError, CapabilityId, CapabilityNamespace, CapabilityPermission, ExecutorRef,
    PackageSidecarRef, PolicyKind, PolicyRef, ProviderArgumentStore, SourceRef,
    ToolExecutionOutput, ToolExecutionRequest, ToolExecutor,
    domain::ContentRef as ContentRefId,
    output::SchemaVersion,
    policy::{EffectClass, RiskClass},
    tool_records::CanonicalToolName,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{
    AsyncTool, Tool, ToolkitPackBundle,
    packs::{ToolBuilder, ToolPackBuilder},
    testing::{InMemoryJsonArgumentStore, InMemoryToolkitContentStore},
};

/// Result type returned by typed tool handlers.
pub type ToolResult<T> = Result<T, ToolError>;

/// Typed arguments for a toolkit-authored tool.
pub trait ToolArgs: Serialize + DeserializeOwned + Send + Sync + 'static {
    /// Stable schema id for the argument shape.
    const SCHEMA_ID: &'static str;
    /// Semantic schema version for the argument shape.
    const SCHEMA_VERSION: SchemaVersion;

    /// Returns a provider-safe JSON schema for these arguments.
    fn schema() -> Value;
}

/// Typed output returned by a toolkit-authored tool.
pub trait ToolOutput: Serialize + Send + Sync + 'static {
    /// Returns a bounded summary safe for journals, events, logs, and prompts.
    fn redacted_summary(&self) -> String {
        "typed tool output".to_string()
    }
}

/// Stable identity for a typed tool declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolIdentity {
    /// Provider-visible canonical tool name.
    pub name: CanonicalToolName,
    /// Tool version.
    pub version: String,
    /// Runtime package capability id.
    pub capability_id: CapabilityId,
    /// Capability namespace.
    pub namespace: CapabilityNamespace,
    /// Executor ref registered in the runtime.
    pub executor_ref: ExecutorRef,
    /// Schema sidecar ref used by provider projection and fingerprints.
    pub schema_ref: PackageSidecarRef,
}

impl ToolIdentity {
    /// Creates a deterministic typed tool identity from name and version.
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Result<Self, AgentError> {
        let name = name.into();
        let version = version.into();
        CanonicalToolName::try_new(name.clone()).map_err(|error| {
            AgentError::contract_violation(format!("invalid tool name: {error}"))
        })?;
        if version.trim().is_empty() {
            return Err(AgentError::missing_required_field("typed_tool.version"));
        }
        Ok(Self {
            name: CanonicalToolName::new(name.clone()),
            version: version.clone(),
            capability_id: CapabilityId::new(format!("cap.tool.{name}")),
            namespace: CapabilityNamespace::new(format!("tool.{name}")),
            executor_ref: ExecutorRef::new(format!("executor.{name}.{version}")),
            schema_ref: PackageSidecarRef::new(
                format!("schema.{name}.{version}"),
                "json_schema",
                version,
            ),
        })
    }

    /// Sets an explicit capability id.
    pub fn capability_id(mut self, id: CapabilityId) -> Self {
        self.capability_id = id;
        self
    }

    /// Sets an explicit executor ref.
    pub fn executor_ref(mut self, executor_ref: ExecutorRef) -> Self {
        self.executor_ref = executor_ref;
        self
    }

    /// Sets an explicit schema ref.
    pub fn schema_ref(mut self, schema_ref: PackageSidecarRef) -> Self {
        self.schema_ref = schema_ref;
        self
    }
}

/// Provider-safe schema snapshot for a typed tool.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolSchemaSnapshot {
    /// Schema sidecar ref with content hash populated.
    pub schema_ref: PackageSidecarRef,
    /// Redacted schema body.
    pub redacted_schema: Value,
    /// Hash of the normalized schema body.
    pub content_hash: String,
}

impl ToolSchemaSnapshot {
    fn new(mut schema_ref: PackageSidecarRef, schema: Value) -> Result<Self, AgentError> {
        let normalized = normalize_json_value(schema);
        let bytes = serde_json::to_vec(&normalized).map_err(|error| {
            AgentError::contract_violation(format!("tool schema serialization failed: {error}"))
        })?;
        let content_hash = format!("sha256:{}", hex_lower(&Sha256::digest(bytes)));
        schema_ref.content_hash = Some(content_hash.clone());
        Ok(Self {
            schema_ref,
            redacted_schema: normalized,
            content_hash,
        })
    }
}

/// Execution context passed to typed tool handlers.
#[derive(Clone)]
pub struct TypedToolContext {
    /// Canonical core execution request.
    pub request: ToolExecutionRequest,
}

/// JSON argument store for typed tool execution.
pub trait JsonToolArgumentStore: Send + Sync {
    /// Loads the JSON arguments behind a content ref.
    fn load_json(&self, content_ref: &ContentRefId) -> Result<Value, AgentError>;
}

/// JSON content store for typed tool outputs.
pub trait JsonToolContentStore: Send + Sync {
    /// Stores one JSON result behind a content ref.
    fn put_json(&self, content_ref: ContentRefId, value: Value) -> Result<(), AgentError>;
}

impl JsonToolArgumentStore for InMemoryJsonArgumentStore {
    fn load_json(&self, content_ref: &ContentRefId) -> Result<Value, AgentError> {
        self.get(content_ref)
    }
}

impl JsonToolArgumentStore for Arc<dyn ProviderArgumentStore> {
    fn load_json(&self, content_ref: &ContentRefId) -> Result<Value, AgentError> {
        self.load_provider_arguments_json(content_ref)
    }
}

impl JsonToolContentStore for InMemoryToolkitContentStore {
    fn put_json(&self, content_ref: ContentRefId, value: Value) -> Result<(), AgentError> {
        self.put(content_ref, &value)
    }
}

/// Typed tool error kind.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ToolErrorKind {
    /// Tool arguments could not be decoded.
    InvalidArguments,
    /// Handler returned a failure.
    HandlerFailed,
    /// Output could not be serialized.
    OutputSerialization,
    /// Content store failed.
    ContentStore,
    /// Tool was cancelled.
    Cancelled,
    /// Tool timed out.
    TimedOut,
}

/// Structured typed tool error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolError {
    /// Finite error kind.
    pub kind: ToolErrorKind,
    /// Stable error code.
    pub code: String,
    /// Redacted summary safe for durable records.
    pub redacted_summary: String,
}

impl ToolError {
    /// Creates a structured typed tool error.
    pub fn new(
        kind: ToolErrorKind,
        code: impl Into<String>,
        redacted_summary: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            code: code.into(),
            redacted_summary: redacted_summary.into(),
        }
    }

    /// Creates a handler-failed error.
    pub fn handler_failed(code: impl Into<String>, summary: impl Into<String>) -> Self {
        Self::new(ToolErrorKind::HandlerFailed, code, summary)
    }
}

/// Host-provided runner for async typed handlers while core execution remains sync.
pub trait AsyncToolRunner: Send + Sync {
    /// Drives an async typed tool future to completion.
    fn block_on_tool<R: ToolOutput>(
        &self,
        future: Pin<Box<dyn Future<Output = ToolResult<R>> + Send>>,
    ) -> ToolResult<R>;
}

type SyncHandler<A, R> = dyn Fn(A, TypedToolContext) -> ToolResult<R> + Send + Sync;

/// Typed tool declaration plus handler adapter.
pub struct TypedTool<A: ToolArgs, R: ToolOutput> {
    identity: ToolIdentity,
    schema: ToolSchemaSnapshot,
    policy_ref: PolicyRef,
    required_permissions: Vec<CapabilityPermission>,
    effect_class: EffectClass,
    risk_class: RiskClass,
    timeout_ms: u64,
    require_approval: bool,
    handler: Arc<SyncHandler<A, R>>,
}

impl<A: ToolArgs, R: ToolOutput> TypedTool<A, R> {
    /// Starts a typed tool builder.
    pub fn builder(identity: ToolIdentity) -> TypedToolBuilder<A, R> {
        TypedToolBuilder::new(identity)
    }

    /// Returns the schema snapshot.
    pub fn schema_snapshot(&self) -> &ToolSchemaSnapshot {
        &self.schema
    }

    /// Marks this tool as requiring host approval before execution.
    pub fn require_approval(mut self) -> Self {
        self.require_approval = true;
        self.risk_class = RiskClass::High;
        self
    }

    /// Returns whether approval is required for this tool.
    pub fn approval_required(&self) -> bool {
        self.require_approval
    }

    /// Lowers to an ergonomic toolkit tool declaration.
    pub fn tool(&self) -> Result<Tool, AgentError> {
        self.tool_builder().build()
    }

    /// Lowers to an ergonomic async toolkit declaration.
    pub fn async_tool(&self) -> Result<AsyncTool, AgentError> {
        self.tool_builder().build_async()
    }

    /// Creates a core tool executor adapter for this typed tool.
    pub fn executor(
        &self,
        args: Arc<dyn JsonToolArgumentStore>,
        out: Arc<dyn JsonToolContentStore>,
    ) -> Arc<dyn ToolExecutor> {
        Arc::new(TypedToolExecutor {
            executor_ref: self.identity.executor_ref.clone(),
            args,
            out,
            handler: self.handler.clone(),
            _args: PhantomData::<A>,
            _output: PhantomData::<R>,
        })
    }

    /// Builds a toolkit pack bundle containing this tool declaration.
    pub fn pack_bundle(&self, source: SourceRef) -> Result<ToolkitPackBundle, AgentError> {
        ToolPackBuilder::new(
            agent_sdk_core::ToolPackId::new(format!("toolpack.{}", self.identity.name.as_str())),
            agent_sdk_core::ToolPackKind::External,
            self.identity.version.clone(),
            source,
        )
        .listen(self.tool()?)
        .build()
    }

    fn tool_builder(&self) -> ToolBuilder {
        let mut builder = Tool::builder(
            self.identity.name.as_str(),
            self.identity.executor_ref.as_str(),
            self.schema.schema_ref.sidecar_id.clone(),
            self.policy_ref.clone(),
        )
        .capability_id(self.identity.capability_id.clone())
        .namespace(self.identity.namespace.clone())
        .redacted_schema(self.schema.redacted_schema.clone())
        .effect(self.effect_class.clone(), self.risk_class.clone())
        .timeout_ms(self.timeout_ms);
        for permission in &self.required_permissions {
            builder = builder.required_permission(permission.clone());
        }
        if self.require_approval {
            builder = builder.require_approval();
        }
        builder
    }
}

/// Builder for a typed tool.
pub struct TypedToolBuilder<A: ToolArgs, R: ToolOutput> {
    identity: ToolIdentity,
    policy_ref: PolicyRef,
    required_permissions: Vec<CapabilityPermission>,
    effect_class: EffectClass,
    risk_class: RiskClass,
    timeout_ms: u64,
    handler: Option<Arc<SyncHandler<A, R>>>,
}

impl<A: ToolArgs, R: ToolOutput> TypedToolBuilder<A, R> {
    fn new(identity: ToolIdentity) -> Self {
        Self {
            identity,
            policy_ref: PolicyRef::with_kind(PolicyKind::RuntimePackage, "policy.tool.typed"),
            required_permissions: Vec::new(),
            effect_class: EffectClass::Read,
            risk_class: RiskClass::Low,
            timeout_ms: 10_000,
            handler: None,
        }
    }

    /// Sets an explicit policy ref.
    pub fn policy_ref(mut self, policy_ref: PolicyRef) -> Self {
        self.policy_ref = policy_ref;
        self
    }

    /// Marks the tool read-only.
    pub fn read_only(mut self) -> Self {
        self.effect_class = EffectClass::Read;
        self.risk_class = RiskClass::Low;
        self
    }

    /// Marks the tool as write-like.
    pub fn write_effect(mut self) -> Self {
        self.effect_class = EffectClass::Write;
        self.risk_class = RiskClass::High;
        self
    }

    /// Sets explicit effect and risk classes.
    pub fn effect(mut self, effect_class: EffectClass, risk_class: RiskClass) -> Self {
        self.effect_class = effect_class;
        self.risk_class = risk_class;
        self
    }

    /// Adds a required permission.
    pub fn required_permission(mut self, permission: CapabilityPermission) -> Self {
        self.required_permissions.push(permission);
        self
    }

    /// Sets execution timeout metadata.
    pub fn timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Sets the sync handler.
    pub fn sync_handler<F>(mut self, handler: F) -> Self
    where
        F: Fn(A, TypedToolContext) -> ToolResult<R> + Send + Sync + 'static,
    {
        self.handler = Some(Arc::new(handler));
        self
    }

    /// Sets an async handler through a host-owned runner.
    pub fn async_handler<F, Fut, Runner>(mut self, runner: Arc<Runner>, handler: F) -> Self
    where
        F: Fn(A, TypedToolContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ToolResult<R>> + Send + 'static,
        Runner: AsyncToolRunner + 'static,
    {
        self.handler = Some(Arc::new(move |args, context| {
            runner.block_on_tool(Box::pin(handler(args, context)))
        }));
        self
    }

    /// Builds the typed tool declaration and executor adapter.
    pub fn build(self) -> Result<TypedTool<A, R>, AgentError> {
        let schema = ToolSchemaSnapshot::new(self.identity.schema_ref.clone(), A::schema())?;
        let handler = self
            .handler
            .ok_or_else(|| AgentError::missing_required_field("typed_tool.handler"))?;
        Ok(TypedTool {
            identity: self.identity,
            schema,
            policy_ref: self.policy_ref,
            required_permissions: self.required_permissions,
            effect_class: self.effect_class,
            risk_class: self.risk_class,
            timeout_ms: self.timeout_ms,
            require_approval: false,
            handler,
        })
    }
}

struct TypedToolExecutor<A: ToolArgs, R: ToolOutput> {
    executor_ref: ExecutorRef,
    args: Arc<dyn JsonToolArgumentStore>,
    out: Arc<dyn JsonToolContentStore>,
    handler: Arc<SyncHandler<A, R>>,
    _args: PhantomData<A>,
    _output: PhantomData<R>,
}

impl<A: ToolArgs, R: ToolOutput> ToolExecutor for TypedToolExecutor<A, R> {
    fn executor_ref(&self) -> &ExecutorRef {
        &self.executor_ref
    }

    fn execute(&self, request: &ToolExecutionRequest) -> Result<ToolExecutionOutput, AgentError> {
        let Some(args_ref) = request
            .resolved_call
            .request
            .requested_args_refs
            .first()
            .cloned()
        else {
            return Ok(ToolExecutionOutput::failed(
                "typed tool arguments were missing",
                "typed_tool.invalid_arguments",
            ));
        };
        let args_json = match self.args.load_json(&args_ref) {
            Ok(value) => value,
            Err(error) => {
                return Ok(ToolExecutionOutput::failed(
                    "typed tool arguments could not be loaded",
                    format!("typed_tool.argument_store.{:?}", error.kind()),
                ));
            }
        };
        let args = match serde_json::from_value::<A>(args_json) {
            Ok(args) => args,
            Err(error) => {
                return Ok(ToolExecutionOutput::failed(
                    "typed tool arguments failed schema decoding",
                    format!("typed_tool.invalid_arguments.{error}"),
                ));
            }
        };
        let output = match (self.handler)(
            args,
            TypedToolContext {
                request: request.clone(),
            },
        ) {
            Ok(output) => output,
            Err(error) => {
                return Ok(ToolExecutionOutput::failed(
                    error.redacted_summary,
                    error.code,
                ));
            }
        };
        let output_summary = output.redacted_summary();
        let output_json = match serde_json::to_value(&output) {
            Ok(value) => value,
            Err(error) => {
                return Ok(ToolExecutionOutput::failed(
                    "typed tool output could not be serialized",
                    format!("typed_tool.output_serialization.{error}"),
                ));
            }
        };
        let result_ref = ContentRefId::new(format!(
            "content.tool.{}.result",
            request.effect_intent.effect_id.as_str()
        ));
        if let Err(error) = self.out.put_json(result_ref.clone(), output_json) {
            return Ok(ToolExecutionOutput::failed(
                "typed tool output could not be stored",
                format!("typed_tool.content_store.{:?}", error.kind()),
            ));
        }
        let mut output = ToolExecutionOutput::completed(output_summary);
        output.content_refs.push(result_ref);
        Ok(output)
    }
}

#[cfg(feature = "schema-generation")]
/// Generates a normalized schema value for a `schemars` type.
pub fn schemars_schema<T: schemars::JsonSchema>() -> Value {
    serde_json::to_value(schemars::schema_for!(T)).expect("schemars schema serializes")
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn normalize_json_value(value: Value) -> Value {
    match value {
        Value::Array(values) => {
            Value::Array(values.into_iter().map(normalize_json_value).collect())
        }
        Value::Object(map) => {
            let mut entries = map
                .into_iter()
                .map(|(key, value)| (key, normalize_json_value(value)))
                .collect::<Vec<_>>();
            entries.sort_by(|left, right| left.0.cmp(&right.0));
            Value::Object(entries.into_iter().collect())
        }
        other => other,
    }
}
