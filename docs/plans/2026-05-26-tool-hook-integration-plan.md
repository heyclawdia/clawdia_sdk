# Tool Hook Integration Plan

Date: 2026-05-26

## Objective

Wire package-declared `BeforeToolCall` and `AfterToolCall` hooks into the
generic `ToolExecutionCoordinator` so tool lifecycle hooks are active on the
side-effect path instead of remaining coordinator-only contract tests.

## Current Problem Shape

The hook contract already includes `BeforeToolCall` and `AfterToolCall`, and the
hook coordinator can validate, invoke, and journal behavior-changing hook
responses. The tool execution coordinator, however, currently routes directly
from resolved tool call to pre-tool policy, tool intent, executor, result, and
post-tool policy. That means tool hook specs can exist in a runtime package but
do not affect actual tool execution unless a caller invokes the hook
coordinator manually outside the canonical tool path.

The structural fix is to make tool execution itself the owner of tool-hook
lowering while reusing the existing hook coordinator, tool records, effect
spine, policy outcomes, and deterministic fakes.

## Authoritative Source Of Truth

- `docs/contracts/hook-lifecycle-contract.md`: hook points, mutation rights,
  journal-before-apply, and response lowering matrix.
- `docs/contracts/tool-approval-contract.md`: tool lifecycle, pre/post policy,
  fail-closed dependency behavior, and intent-before-executor rule.
- `docs/contracts/tool-pack-contract.md`: tool-pack side-effect boundaries and
  specific watchpoint that `BeforeToolCall` hooks may deny or narrow shell/tool
  requests without silently changing ownership.
- `docs/implementation-workstreams/09-p2-side-effects/09b-tool-execution.md`:
  tool coordinator owned files and validation target.
- `docs/implementation-workstreams/09-p2-side-effects/09d-hook-lifecycle.md`:
  hook coordinator owned files and journal-before-apply requirement.
- `docs/implementation-workstreams/09-p2-side-effects/_phase/phase-exit-report.md`:
  carry-forward watchpoint that generic denied-before-execution tool paths did
  not journal denied tool records in Phase 09 unless a later contract explicitly
  adds that behavior.

## Relevant Existing Context

- `AGENTS.md`: do not create branches; keep the SDK product-neutral; use coding
  orchestrator for complex tasks.
- `README.md` and `docs/start-here.md`: core owns typed primitives and ports;
  concrete tools, UI, network, filesystem, and product workflows stay outside
  core or optional crates.
- `coding_standards.md` and `docs/architecture/coding-standards.md`: mutating
  side effects must preserve journal durability, policy, lineage, privacy,
  replay, and mockability.
- `docs/workstreams/validation-gates.md`: implementation needs tests, primitive
  fit, effect gate, no-mini-SDK proof, mockability, and source-layout audits.
- `docs/reference/sdk-review-checklist.md`: reject ambient callbacks, generic
  side-effect hatches, hidden policy paths, and helper behavior that bypasses
  canonical records.
- `docs/architecture/primitive-map.md`: hooks are package sidecars plus ordered
  executors; tools are package capabilities plus executor ports and the shared
  effect spine.
- `docs/plans/2026-05-26-run-text-hook-integration-plan.md`: recent run-text
  slice established the active-loop pattern for guarded hook invocation, bounded
  redacted summaries, conservative one-mutating-hook validation, and durable
  rejected hook records.

## Behavior Contract

New behavior:

- `ToolExecutionCoordinator` can be configured with package hook specs and a
  hook executor registry port.
- It invokes only tool lifecycle hooks in this slice:
  `BeforeToolCall` and `AfterToolCall`.
- Tool hook specs are resolved through the same host-owned
  `HookExecutorRegistry`; missing hook executors fail before tool executor
  start.
- `BeforeToolCall` accepted `Deny` responses append hook
  journal-before-apply evidence, append a terminal tool
  denied-before-execution record with an `EffectResult` whose terminal status is
  `DeniedBeforeExecution`, and prevent both tool intent append and tool executor
  start. This hook-owned denial is durable even though generic missing-policy or
  missing-executor denied paths remain unjournaled from the Phase 09 contract.
- `BeforeToolCall` accepted `ModifyToolRequest` responses append hook evidence,
  append a tool request-modified record that preserves both the original and
  patched redacted argument summaries plus hook lineage, then narrow the tool
  request's redacted argument summary before pre-tool policy and executor
  intent. The current DTO only carries a redacted summary, so this slice does
  not mutate raw arguments, content refs, route, executor ref, or package
  authority.
- `AfterToolCall` observe hooks run after the original tool result is durably
  recorded and before post-tool policy observes the final output.
- `AfterToolCall` accepted `RewriteToolResult` responses append hook evidence,
  append a tool result-rewritten record, then replace only the returned
  redacted result summary. The original tool result journal record remains
  intact.
- Guard-level payload bound failures append rejected hook response evidence
  before failing closed. For `BeforeToolCall` payload rejection, the executor is
  not called. For `AfterToolCall` rewrite payload rejection, the already
  journaled original tool result remains durable, the rejected hook response is
  recorded, no rewrite record is appended, post-tool policy does not run, and
  the coordinator returns the hook payload failure rather than silently exposing
  the original as success.
- Unsupported active tool hook mutation rights, currently `RequestApproval` at
  `BeforeToolCall` and `RequestRetry` at `AfterToolCall`, fail closed before
  tool executor start instead of being accepted and ignored. They remain in the
  global contract matrix but need a later approval/retry integration slice with
  attempt IDs, dispatcher sequencing, and idempotency semantics.

Preserved behavior:

- Tool execution still resolves against the runtime-package-derived
  `ToolRegistrySnapshot`; hooks cannot change route, capability ID,
  executor ref, policy refs, content refs, package state, or host process
  ownership.
- Tool executor start still requires pre-tool policy allow and a successfully
  appended tool execution intent.
- Tool terminal result append failure still enters recovery for unsafe pending
  side effects.
- Existing tool execution tests without hooks keep their current behavior and
  fixture shape.
- Hook executors remain host-owned ports; core does not store closures, spawn
  hook processes, or add product-specific adapters.
- Concrete filesystem, shell, MCP, workspace, or browser behavior remains out of
  `agent-sdk-core`.

Removed behavior:

- None.

Tests proving behavior:

- Before-tool deny prevents executor start and journals hook decision plus a
  terminal denied-before-execution tool record/effect result.
- Before-tool request modification changes the executor-visible redacted
  argument summary only after hook journal evidence and a tool modification
  record that preserves original and patched summaries.
- After-tool observe hook runs after the original result is recorded.
- After-tool rewrite preserves the original result record, appends hook evidence
  and a rewrite record, and returns the rewritten redacted summary to post-tool
  policy.
- Oversized deny, modify-request, and rewrite-result summaries fail closed with
  rejected hook response records; the before-tool cases prevent executor start
  and the after-tool rewrite case preserves the original result without running
  post-tool policy.
- Missing hook executor fails before the tool executor is called.
- Unsupported request-approval and request-retry rights fail before tool
  executor start.
- Existing no-hook tool execution fixtures remain unchanged.

## Scope

Writable files for this slice:

- `docs/plans/2026-05-26-tool-hook-integration-plan.md`
- `docs/contracts/hook-lifecycle-contract.md`
- `docs/contracts/tool-approval-contract.md`
- `crates/agent-sdk-core/src/application/tool.rs`
- `crates/agent-sdk-core/src/records/tool.rs`
- `crates/agent-sdk-core/tests/feature_layers/tool_execution_contract.rs`
- new or updated fixtures under `crates/agent-sdk-core/tests/fixtures/tools/`

Out of scope:

- Tool hook invocation from `run_text`, because the current P0/P1 provider loop
  does not execute tools.
- Approval broker wiring for hook-requested approval.
- Tool retry orchestration for hook-requested retry.
- Raw argument patching, schema-aware argument transforms, content-ref rewrites,
  executor route changes, tool-pack behavior, MCP, shell, filesystem, or
  product UI.
- Branch creation or push.

## Workstreams

1. Tool hook configuration and validation:
   - Add hook specs and registry configuration to `ToolExecutionCoordinator`.
   - Filter/invoke only `BeforeToolCall` and `AfterToolCall`.
   - Validate that supported active mutation rights are lowerable in this slice
     and that at most one behavior-changing hook is present at each tool point.

2. Before-tool lowering:
   - Build bounded redacted `HookView` values from resolved tool-call metadata.
   - Lower accepted `Deny` into a hook-backed tool denial record and no executor
     call.
   - Lower accepted `ModifyToolRequest` into a request-modified tool record and
     modified redacted summary before policy/executor intent. The tool record
     must retain both original and patched summaries for replay/debugging.

3. After-tool lowering:
   - Invoke observe/rewrite hooks after original terminal result record append.
   - Lower `RewriteToolResult` into a tool result-rewritten journal record before
     mutating the returned output summary.
   - Keep post-tool policy evaluation over the final output.
   - If rewrite validation fails after the original terminal result was already
     recorded, append a rejected hook response record, do not append a rewrite
     tool record, do not run post-tool policy, and return the hook failure with
     the original result preserved in the journal.

4. Tests and docs:
   - Add focused `tool_execution_contract` tests for the new active hook paths.
   - Update hook/tool docs with the active-slice limits and future
     approval/retry watchpoints.
   - Run focused tests, full core tests, clippy, and source-layout audits.

## Risk / Gotcha Carry-Forward

- Do not make hooks a tool executor or approval dispatcher. Hook responses are
  typed proposals lowered by the tool domain.
- Do not let `ModifyToolRequest` mutate raw args through the current summary-only
  DTO. Schema-aware argument patching is a separate contract expansion.
- Do not implement `RequestApproval` without threading the real approval broker
  and dispatcher sequencing through tool execution.
- Do not implement `RequestRetry` without attempt-specific IDs, idempotency
  checks, and bounded retry journal evidence.
- Do not hide accepted-but-unapplied mutations behind later same-point hook
  failures; keep this slice conservative with at most one behavior-changing hook
  per active tool point.
- Do not change existing no-hook tool fixture shapes.
- Do not add concrete tool-pack behavior to core.

## Public API / SemVer Note

This slice adds builder-style configuration methods on `ToolExecutionCoordinator`
and may add durable `ToolCallRecordStatus` variants for hook-owned tool request
modification, hook denial, and result rewrite evidence. The workspace is alpha,
but these are public surface changes and must be covered by the API review note
in the final handoff.

## Review Packet

Primitive decision:

- Reused kernel primitives: `RuntimePackage` hook specs, hook executor registry,
  `ToolExecutionCoordinator`, `RunJournal`, `EffectIntent`, `EffectResult`,
  `ToolCallRecord`, `PolicyOutcome`, source/destination refs, privacy, and typed
  IDs.
- New feature-layer primitives: none.
- New capability variants: none.
- Host-owned behavior kept out: concrete hooks, approval UI/transport, tool
  implementations, filesystem/shell/MCP behavior, and product-specific routing.

Validation evidence to collect:

- `cargo fmt --check`
- `cargo test -p agent-sdk-core --test tool_execution_contract --test hook_lifecycle_contract`
- `cargo test -p agent-sdk-core`
- `cargo clippy --workspace --all-targets -- -D warnings`
- Source-layout audit commands from `docs/workstreams/validation-gates.md`

Reviewer checklist:

- Tool hooks are invoked only at tool lifecycle points.
- Accepted hook mutations are journaled before apply and paired with tool-domain
  evidence when they change request/result behavior.
- Tool executor still cannot start before package route validation, pre-tool
  allow policy, and tool intent append.
- Unsupported approval/retry hook responses fail closed instead of being silently
  ignored.
- No raw content, concrete tool behavior, product UI, or ambient callbacks enter
  core.
