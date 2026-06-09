# Tool-Approval Quickstart

Tools are not callbacks hanging off a model response. A model-visible tool must
lower into a package capability, a route, policy refs, journal intent/result
records, and effect evidence.

The canonical path is:

`CapabilitySpec` -> `ToolRoute` -> `ToolRegistrySnapshot` -> `ToolRouter` ->
`ToolExecutionCoordinator` -> policy -> journal intent -> `ToolExecutor` ->
journal result -> provider tool-result continuation message.

```rust
use std::sync::Arc;

use agent_sdk_core::{
    AllowToolPolicy, CapabilityId, CapabilitySpec, DestinationKind, DestinationRef,
    ExecutorRef, IdempotencyKey, PolicyKind, PolicyRef, PrivacyClass, ProviderRouteSnapshot,
    RetentionClass, RuntimePackage, RuntimePackageId, SourceKind, SourceRef, ToolCallId,
    ToolExecutionContext, ToolExecutionCoordinator,
    ids::ContentRef as ContentRefId,
    testing::{FakeJournalStore, ScriptedToolExecutor},
    tool_ports::{
        ToolCallRequest, ToolExecutionOutput, ToolExecutorRegistry, ToolRegistrySnapshot,
        ToolRoute, ToolRouter,
    },
    tool_records::CanonicalToolName,
};

let package = RuntimePackage::builder(RuntimePackageId::new("package.quickstart.tools"))
    .provider_route(ProviderRouteSnapshot::new("provider.fake", "model.fake"))
    .capability(CapabilitySpec::fake_tool(
        "cap.tool.read",
        "workspace_read",
        agent_sdk_core::PackageSidecarRef::new("sidecar.schema.read", "tool_schema", "v1"),
        ExecutorRef::new("executor.workspace_read.v1"),
        PolicyRef::with_kind(PolicyKind::Approval, "policy.approval.read"),
        SourceRef::with_kind(SourceKind::Sdk, "source.sdk.toolpack"),
    ))
    .build()?;

let route = ToolRoute {
    capability_id: CapabilityId::new("cap.tool.read"),
    canonical_tool_name: CanonicalToolName::new("workspace_read"),
    namespace: agent_sdk_core::CapabilityNamespace::new("tool.workspace_read"),
    description: Some("Read a file from the workspace".to_string()),
    source: SourceRef::with_kind(SourceKind::Sdk, "source.sdk.toolpack"),
    destination: DestinationRef::with_kind(DestinationKind::Tool, "destination.tool.read"),
    executor_ref: Some(ExecutorRef::new("executor.workspace_read.v1")),
    policy_refs: vec![PolicyRef::with_kind(PolicyKind::Approval, "policy.approval.read")],
    requires_approval: true,
    sidecar_refs: vec![agent_sdk_core::PackageSidecarRef::new(
        "sidecar.schema.read",
        "tool_schema",
        "v1",
    )],
    effect_class: agent_sdk_core::EffectClass::Read,
    risk_class: agent_sdk_core::RiskClass::Low,
    privacy: PrivacyClass::ContentRefsOnly,
    retention: RetentionClass::RunScoped,
};

let snapshot = ToolRegistrySnapshot::from_runtime_package(&package, [route])?;

let executor = Arc::new(ScriptedToolExecutor::new(
    ExecutorRef::new("executor.workspace_read.v1"),
    ToolExecutionOutput::completed("workspace read returned content refs"),
));
let mut executors = ToolExecutorRegistry::new();
executors.register(executor)?;

let coordinator = ToolExecutionCoordinator::new(ToolRouter::new(snapshot), executors)
    .with_policy(Arc::new(AllowToolPolicy));

let journal = FakeJournalStore::default();
let outcome = coordinator.execute(
    &journal,
    ToolCallRequest {
        tool_call_id: ToolCallId::new("tool.call.read.1"),
        canonical_tool_name: CanonicalToolName::new("workspace_read"),
        source: SourceRef::with_kind(SourceKind::Sdk, "source.model.tool_call"),
        requested_args_refs: vec![ContentRefId::new("content.args.read.1")],
        redacted_args_summary: "read docs/start-here.md".to_string(),
        idempotency_key: Some(IdempotencyKey::new("idem.read.1")),
        dedupe_key: None,
    },
    ToolExecutionContext::new(
        agent_sdk_core::RunId::new("run.quickstart.tools"),
        agent_sdk_core::AgentId::new("agent.quickstart.tools"),
        SourceRef::with_kind(SourceKind::Sdk, "source.sdk.run_loop"),
        package.fingerprint()?.as_str(),
    ),
)?;
```

## What This Proves

- `ToolRoute` must match the executable capability frozen into the
  `RuntimePackage`.
- Approval dispatch is explicit: an approval policy ref documents policy
  lineage, while `requires_approval = true` gates executor release.
- Missing policy, missing executor, or failed journal intent append denies before
  executor start.
- The executor result is not durable until the terminal tool result record is
  appended.
- The app-facing `run_text` path can consume a provider `tool_use` response,
  execute the requested tool through this coordinator, append a tool-result
  message, and continue with the provider.
- Tool wrappers can be added later, but they must produce this same route,
  policy, journal, event, and effect shape.

## What This Does Not Claim Yet

- Streaming tool-call input deltas, concurrent tool execution, and
  provider-native function-result replay are still follow-up work.
- Live MCP, browser, shell, and workspace tool packs belong in optional adapter
  crates or the toolkit, with fail-closed resource and SSRF policy.
- Human approval transport is host-owned. Core owns the policy and journaled
  decision model, not UI copy or channels.
