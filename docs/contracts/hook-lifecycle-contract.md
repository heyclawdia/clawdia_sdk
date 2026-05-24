# Hook Lifecycle Contract

Hooks are first-class SDK lifecycle primitives. They let SDK users attach typed behavior at named agent-loop points without giving callbacks ambient access to process, filesystem, network, approval, memory, telemetry, or provider internals.

The same hook contract supports code-first registration and declarative config. Both lower into `HookSpec` package sidecars and executor refs inside `RuntimePackage`, then are fingerprinted before a run starts.

## Public Shape

```rust
// Non-compiling contract sketch.
pub struct HookSpec {
    pub hook_id: HookId,
    pub point: HookPoint,
    pub source: HookSource,
    pub ordering: HookOrdering,
    pub execution: HookExecutionMode,
    pub timeout: HookTimeoutPolicy,
    pub failure: HookFailurePolicy,
    pub mutation_rights: HookMutationRights,
    pub privacy: HookPrivacyPolicy,
    pub policy_ref: PolicyRef,
    pub executor_ref: HookExecutorRef,
}

pub enum HookPoint {
    RunStarting,
    BeforeContextAssembly,
    AfterContextAssembly,
    BeforeProviderProjection,
    BeforeModelCall,
    OnModelDelta,
    AfterModelCall,
    BeforeStructuredOutputValidation,
    AfterStructuredOutputValidation,
    BeforeToolCall,
    AfterToolCall,
    BeforeApprovalRequest,
    AfterApprovalDecision,
    BeforeSubagentStart,
    AfterSubagentTerminal,
    BeforeIsolationProcessStart,
    AfterIsolationProcessExit,
    OnRunCancelRequested,
    BeforeRunComplete,
    AfterRunTerminal,
    BeforeCompaction,
    AfterCompaction,
}

pub trait AgentHook: Send + Sync {
    async fn invoke(&self, input: HookInput) -> Result<HookResponse, AgentError>;
}

pub struct HookInput {
    pub hook_id: HookId,
    pub point: HookPoint,
    pub run_id: RunId,
    pub agent_id: AgentId,
    pub turn_id: Option<TurnId>,
    pub attempt_id: Option<AttemptId>,
    pub source: SourceRef,
    pub destination: Option<DestinationRef>,
    pub package_fingerprint: RuntimePackageFingerprint,
    pub view: HookView,
    pub policy_refs: Vec<PolicyDecisionRef>,
    pub cancellation: CancellationToken,
}

pub enum HookResponse {
    ObserveOnly,
    InjectContext(Vec<ContextInjectionRequest>),
    ModifyProjection(ProjectionPatch),
    RequestCompaction(CompactionRequest),
    ModifyValidationHints(ValidationHintPatch),
    ModifyToolRequest(ToolRequestPatch),
    ModifyApprovalRequest(ApprovalRequestPatch),
    Deny(DenyReason),
    RequestApproval(ApprovalRequestPatch),
    RequestRetry(RetryRequest),
    RewriteToolResult(ToolResultPatch),
    ModifySubagentRequest(SubagentRequestPatch),
    ModifyProcessRequest(ProcessRequestPatch),
    ValidateDetach(DetachValidationRequest),
    RequestUsageRollupRepair(UsageRollupRepairRequest),
    RequestCleanupRepair(CleanupRepairRequest),
    MarkProtectedContext(Vec<ContextItemId>),
    RequestProjectionAuditRepair(ProjectionAuditRepairRequest),
    StopCompletionWithRepairNeeded(RepairNeededReason),
    StopRun(StopReason),
}

pub enum HookExecutionMode {
    Blocking,
    NonBlocking {
        queue: HookQueueConfig,
        overflow: HookOverflowPolicy,
    },
}

pub struct HookQueueConfig {
    pub capacity: NonZeroUsize,
    pub terminal_reserve: NonZeroUsize,
}

pub enum HookOverflowPolicy {
    DropObserveOnly,
    SummarizeAndContinue,
    FailHookInvocation,
}
```

`HookView` is an SDK-produced redacted view. Raw prompt, model, tool, file, memory, media, or secret content is absent unless content-capture policy explicitly allows that hook source to see it.

## Defaults And Performance

The default hook posture is observation-first and safe under load:

- Hook inputs are envelope/index fields, typed IDs, policy refs, content refs, hashes, byte/token counts, statuses, and bounded redacted summaries.
- Raw content is never included by default.
- Hook delivery is nonblocking by default.
- Nonblocking hook queues are bounded and use declared overflow behavior.
- Slow observe hooks cannot block provider streaming, tool execution, cancellation, or terminal sealing.
- Blocking hooks must be explicitly declared and can guard only their named lifecycle point.
- Security-critical blocking hooks cannot fail open.
- Extension-backed hooks use the same SDK hook contract, but their JSON-RPC/process bridge is outside `agent-sdk-core`.

## Registration Surfaces

Declarative config:

```toml
# Non-authoritative TOML sketch.
[[hooks]]
id = "audit.before_tool"
point = "BeforeToolCall"
source = "host-config"
ordering = { phase = "normal", order = 100 }
execution = { mode = "blocking" }
timeout_ms = 250
failure = "interrupt"
mutation_rights = ["deny", "request_approval"]
policy_ref = "policy.hooks.audit"
executor_ref = "hook.audit_before_tool"
```

Code-first SDK:

```rust
// Non-compiling contract sketch.
let agent = Agent::builder()
    .id(AgentId::new("agent.default"))
    .on(HookPoint::BeforeToolCall, AuditHook::new())
    .on(HookPoint::OnRunCancelRequested, CleanupHook::bounded())
    .build()?;
```

Canonical lowering:

- Config hooks parse into `HookSpec` values.
- Reserved hook registration helpers such as `AgentBuilder::on(...)` create the same `HookSpec` sidecar shape using default timeout, failure, privacy, and policy refs.
- `RuntimePackageBuilder` stores hook specs as typed package sidecars with hook executor refs. A hook becomes a `CapabilitySpec` only when a feature workstream explicitly exposes it as a callable/discoverable capability.
- Runtime package fingerprint includes hook ID, point, source, ordering, execution mode, queue/overflow config, timeout, failure, mutation rights, privacy, policy ref, and executor ref.
- A hook cannot be added to an active run by mutating an ambient registry. Hook discovery or activation creates a next-turn or next-run package delta.

## Mutation Rights

Hooks mutate only through typed responses. No hook receives arbitrary mutable references to the loop, transcript, provider request, tool executor, journal, or host process.

| Hook point | Allowed response classes |
| --- | --- |
| `RunStarting` | observe, inject bounded context, stop run |
| `BeforeContextAssembly` | observe, inject bounded context |
| `AfterContextAssembly` | observe, `RequestCompaction`, stop run |
| `BeforeProviderProjection` | observe, modify projection metadata, stop run |
| `BeforeModelCall` | observe, modify projection through `ProjectionPatch`, request approval, stop run |
| `OnModelDelta` | observe only; stream-rule interventions own stop/mask/retry decisions |
| `AfterModelCall` | observe, request retry, stop run |
| `BeforeStructuredOutputValidation` | observe, `ModifyValidationHints` |
| `AfterStructuredOutputValidation` | observe, request repair retry within retry policy |
| `BeforeToolCall` | observe, deny, modify tool request within schema, request approval |
| `AfterToolCall` | observe, request retry, `RewriteToolResult` only through journaled result patch |
| `BeforeApprovalRequest` | observe, `ModifyApprovalRequest`, deny |
| `AfterApprovalDecision` | observe only |
| `BeforeSubagentStart` | observe, deny, `ModifySubagentRequest` within `SubagentLifecyclePolicy` and child package policy |
| `AfterSubagentTerminal` | observe, `RequestUsageRollupRepair` |
| `BeforeIsolationProcessStart` | observe, deny, `ModifyProcessRequest` within `ProcessOwnershipPolicy` and isolation policy |
| `AfterIsolationProcessExit` | observe, `RequestCleanupRepair` |
| `OnRunCancelRequested` | observe, propose cleanup through `RequestCleanupRepair` or an existing child/process lifecycle operation, cannot veto cancellation |
| `BeforeRunComplete` | observe, `ValidateDetach`, `StopCompletionWithRepairNeeded` |
| `AfterRunTerminal` | observe only; best effort and cannot change terminal result |
| `BeforeCompaction` | observe, `MarkProtectedContext` through policy |
| `AfterCompaction` | observe, `RequestProjectionAuditRepair` |

If a response class is not in the hook spec's `mutation_rights`, the SDK rejects it as `PolicyDenied` and records `HookResponseRejected`.

`HookResponse` is intentionally closed for the first Rust slice. Future response classes require updating this enum, this table, event payload fixtures, journal fixtures, and mutation-right tests. Hooks do not emit arbitrary events or enqueue generic SDK effects. A behavior-changing hook response is accepted only when it lowers into an existing domain operation such as context injection, tool denial, approval request mutation, process request mutation, child lifecycle action, or cleanup repair; that domain operation emits its normal events and journal records.

## Ordering, Timeouts, And Failure

```rust
// Non-compiling contract sketch.
pub enum HookFailurePolicy {
    FailOpenObserveOnly,
    Deny,
    InterruptRun,
    FailRun,
}

pub enum HookOrderingPhase {
    Early,
    Normal,
    Late,
}
```

Rules:

- Hook ordering is deterministic: `(point, phase, order, hook_id)`.
- Blocking hooks run before the lifecycle transition they guard. Nonblocking hooks observe through bounded queues and cannot delay the loop.
- Security-relevant hooks cannot use `FailOpenObserveOnly`.
- A timeout on a nonblocking observe-only hook records `HookTimedOut` and continues.
- A timeout on a blocking security hook applies its declared deny/interrupt/fail policy.
- Hook invocation is cancellable. Manual run cancellation interrupts in-flight hooks and records `HookCancelled` before child shutdown reconciliation continues.
- `OnRunCancelRequested` hooks have a separate small cleanup deadline and cannot extend the run's child-shutdown grace period unless the run policy explicitly allows it.

## Event And Journal Rules

Events:

- `HookRegistered`
- `HookInvoked`
- `HookCompleted`
- `HookFailed`
- `HookTimedOut`
- `HookCancelled`
- `HookResponseApplied`
- `HookResponseRejected`

Journal records:

- `HookRecord { registered spec hash }`
- `HookRecord { invocation started }`
- `HookRecord { response summary }`
- `HookRecord { timeout/cancel/failure }`
- `ContextRecord`, `ToolRecord`, `ApprovalRecord`, `SubagentRecord`, `IsolationRecord`, or `RecoveryRecord` for the typed mutation the hook requested.

Rules:

- A hook response that changes run behavior is journaled before it is applied.
- Accepted hook proposals lower into normal SDK domain operations. Any side effect created by that operation must satisfy intent-before-effect.
- `HookRegistered` is a run-effective event emitted when a hook spec becomes part of the immutable runtime package for a specific run, after `RunStarted` has a `run_id`. Pre-run package construction is represented by package/capability validation records, not by run-scoped `AgentEvent`s.
- Hook events use `SourceRef.kind = hook` for in-process hooks and `SourceRef.kind = extension` for extension-provided hooks.
- Hook payloads default to IDs, refs, hashes, sizes, statuses, policy refs, and redacted summaries. Raw content is opt-in.
- Replay never reinvokes hooks during audit replay. Resume replay restores the committed hook response state from journal records and invokes only hooks for new lifecycle points after resume.

## Extension Boundary

Core owns `HookSpec`, `HookPoint`, `HookInput`, `HookResponse`, lifecycle-specific proposal types, events, journal records, ordering, execution mode, queue/overflow semantics, timeout, policy, and canonical lowering.

Optional extension crates or hosts own JSON-RPC, subprocess lifecycle, marketplace UX, app-event fanout, packaged resource fallback, and concrete hook process management.

An extension-provided hook is just a hook whose `HookSource` and `executor_ref` route through an extension adapter. It cannot approve itself, grant its own permissions, or bypass runtime package policy.

## Acceptance Tests

- `agent_on_hook_lowers_to_hook_spec_sidecar`
- `config_hook_and_code_hook_share_runtime_package_shape`
- `hook_execution_mode_and_queue_are_fingerprinted`
- `hook_ordering_is_deterministic_by_point_phase_order_and_id`
- `nonblocking_observe_hook_timeout_fails_open_with_event`
- `security_hook_timeout_denies_or_interrupts_not_fail_open`
- `hook_response_class_outside_mutation_rights_is_rejected`
- `before_tool_hook_can_deny_before_executor_start`
- `before_isolation_process_hook_cannot_silently_downgrade_environment`
- `cancel_interrupts_inflight_hooks_and_continues_child_shutdown`
- `audit_replay_does_not_reinvoke_hooks`
- `resume_replay_restores_committed_hook_response_state`
- `extension_hook_routes_through_core_hook_spec_without_core_json_rpc_runtime`
- `slow_hook_does_not_block_loop`
- `hook_inputs_are_content_refs_by_default`
- `hook_events_have_no_raw_content_by_default`

## Complete Example

Typed shape:

```rust
// Non-compiling contract sketch.
let hook = HookSpec {
    hook_id: HookId::new("audit.before_tool"),
    point: HookPoint::BeforeToolCall,
    source: HookSource::InProcess,
    ordering: HookOrdering::normal(100),
    execution: HookExecutionMode::Blocking,
    timeout: HookTimeoutPolicy::bounded_ms(250),
    failure: HookFailurePolicy::InterruptRun,
    mutation_rights: HookMutationRights::deny_or_request_approval(),
    privacy: HookPrivacyPolicy::EnvelopeAndRedactedSummary,
    policy_ref: PolicyRef::new("policy.hooks.audit"),
    executor_ref: HookExecutorRef::in_process("hook.audit_before_tool"),
};

let package = RuntimePackageBuilder::new(RuntimePackageId::new("pkg.example"))
    .hook_sidecar(hook)
    .build_canonical_v1()?;
```

Replaceable ports:

- `AgentHook` can be in-process, extension-backed, remote, or fake.
- `HookExecutorRegistry` resolves `HookExecutorRef`.
- `RuntimePackageBuilder` validates hook sidecars and executor refs before runs start.

Wiring:

1. User registers hook via config or a reserved hook helper such as `AgentBuilder::on`.
2. Builder lowers it into a `HookSpec` package sidecar.
3. Runtime package fingerprints the hook sidecar, executor ref, and policy fields.
4. Agent loop reaches `BeforeToolCall`.
5. Hook bus invokes matching hooks in deterministic order.
6. SDK journals any behavior-changing response before applying it.

Events:

- `HookRegistered`
- `HookInvoked`
- `HookResponseApplied` or `HookResponseRejected`
- `HookCompleted`, `HookTimedOut`, `HookCancelled`, or `HookFailed`

Journal:

- `HookRecord { registered spec hash }`
- `HookRecord { invocation started }`
- `HookRecord { response summary }`
- `ToolRecord`, `ApprovalRecord`, or `RecoveryRecord` for the applied mutation.

Policies and failures:

- Security hooks cannot fail open.
- Observe-only hooks cannot mutate.
- Extension hooks cannot self-approve.
- Cancellation interrupts hooks and continues child shutdown.

SDK owns / Host owns:

- SDK owns hook points, typed responses, closed SDK effect requests, ordering, execution mode/queue/overflow semantics, timeout/failure semantics, package lowering, events, journals, replay behavior, and mutation rights.
- Host owns hook configuration files, installed hook executors, extension subprocesses, UI for hook errors, and product-specific hook libraries.

Tests:

- `agent_on_hook_lowers_to_hook_spec_sidecar`
- `config_hook_and_code_hook_share_runtime_package_shape`
- `security_hook_timeout_denies_or_interrupts_not_fail_open`
- `hook_execution_mode_and_queue_are_fingerprinted`
