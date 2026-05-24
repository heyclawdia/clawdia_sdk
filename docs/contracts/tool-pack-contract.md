# Tool Pack Contract

Built-in tool packs are optional SDK toolkit surfaces. They are not a coding-agent product.

## External Lessons

- oh-my-pi proves high-quality read/search/edit/write/shell tools make agents much easier to build.
- Pi keeps agent core separate from coding harness UI. The SDK should follow that boundary: contracts and optional toolkit crates, not a bundled product.
- Cursor/Claude expose MCP/tools as extension surfaces. The SDK should keep source, namespace, permission, and approval explicit.

## Placement

```mermaid
flowchart TD
  A["agent_sdk core"] --> B["Tool contracts"]
  A --> C["Tool result envelopes"]
  A --> D["Policy and journal"]
  E["agent_sdk_toolkit optional crates"] --> F["read/search/edit/write/shell implementations"]
  F --> A
  G["Host product"] --> H["RuntimePackage"]
  H --> F
```

Core owns contracts. Optional toolkit crates own broad tool implementations. Hosts decide which packs enter each runtime package.

Acceptance boundary:

- `agent_sdk_core` must build without broad toolkit implementations.
- Runtime packages can accept any external tool pack that satisfies the `ToolPack` contract.
- Test fakes in core can exercise the loop without file, shell, URL, SQLite, or AST dependencies.

## RuntimePackage Lowering And Snapshot

Tool packs are package inputs, not live registries. A host may discover installed SDK toolkit packs, MCP tools, extension actions, or host-provided tools, but a running agent can only see and execute the capabilities lowered into its active `RuntimePackage`.

Lowering a pack creates:

- callable or discoverable `CapabilitySpec` entries with `CapabilityId`, `CapabilityKind`, namespace, projection mode, executor ref, policy ref, sidecar refs, and optional isolation ref;
- a typed tool-pack sidecar snapshot containing `ToolPackId`, version, source `SourceRef`, trust state, tool specs, executor route refs, permission/approval/sandbox policy refs, redaction policy refs, and reconciliation requirements;
- provider-visible schemas and executable registry routes whose hashes match the runtime-package projection/execution invariant;
- package fingerprint inputs for every execution-affecting schema, executor route, policy ref, sidecar version, catalog source, isolation/detach policy, and redaction rule.

Rules:

- No preset display name, catalog scan, MCP discovery result, extension manifest, or host registry entry is executable by itself.
- Hidden/discoverable tools may return candidates, but activation creates a package delta for the next turn or run. It never mutates the active package ambiently.
- Missing `PolicyRef`, missing executor ref, namespace collision, stale catalog snapshot, or provider-visible schema without an executable route fails package validation.
- Toolkit helpers and manually declared tool specs must produce the same canonical package shape, fingerprint, `AgentEvent`s, `RunJournal` records, and policy failures.

## Common Tool Spec Fields

- typed tool spec ID and capability ID
- canonical tool name
- namespace and projection mode
- source kind and source ID
- input schema
- output schema
- effect class: read, write, network, process, memory, remote send, child run
- risk class
- executor ref
- policy refs for permission, approval, sandbox, autonomy/escalation, retention, and redaction where applicable
- required permissions
- sandbox/isolation requirement
- idempotency hint
- reconciliation requirement
- timeout
- cancellation support
- process ownership policy when the tool can start child processes
- privacy policy
- result redaction policy

## Pack Contracts

| Pack | Required semantics |
| --- | --- |
| `workspace_readonly` | path policy, symlink policy, hidden-file policy, max bytes, file-kind detection, parser pipeline, bounded-prefix behavior for oversized safe files, truncation guidance, anchors, hashes, MIME type, media/document/archive/SQLite/resource metadata, safe binary summaries |
| `workspace_search` | regex dialect, glob rules, gitignore behavior, max matches, context lines, pagination cursor |
| `workspace_edit` | read snapshot ref, hashline-style anchor validation, preview diff, apply phase, stale-anchor handling, inverse candidate |
| `workspace_write` | create/overwrite scope, parent dir policy, before/after hash, destructive approval |
| `shell` | structured argv by default, cwd/env policy, network policy, timeout, PTY mode, cancellation, isolation requirement, process ownership, detach policy |
| `resource_readers` | resource URI source, sensitivity, content ref, parser version, truncation, retention |
| `tool_discovery` | search hidden tools, activation policy, package delta, no ambient mutation |

Out of scope for the first SDK toolkit slice:

- LSP operations.
- Debugger/DAP operations.
- Browser automation.
- Product-specific code-review agents.
- Broad AST rewrite engines.

The core must leave room for hosts or later optional crates to add those, but implementation should start with read/search/hash-anchored edit/write/shell/resource primitives only.

## Per-Resource Permission Matrix

| Resource | Required permission | Extra policy |
| --- | --- | --- |
| workspace file | filesystem read in workspace scope | symlink/external mount policy |
| workspace directory | filesystem list in workspace scope | hidden/ignored file policy |
| URL | network egress permission | host allowlist, timeout, content type |
| archive | file read plus archive parser permission | decompression limits, path traversal guard |
| SQLite | file read plus structured DB reader permission | query limits, schema privacy |
| notebook/document | file read plus parser permission | embedded media handling |
| image/audio/video | media read permission | derived metadata only by default |
| memory URI | memory read permission | memory retention/sensitivity policy |
| artifact URI | artifact read permission | artifact owner/scope |
| MCP resource | MCP allowlist permission | server/tool namespace policy |
| skill/plugin URI | skill/plugin read permission | installed source trust |

Read and resource outputs stay behind refs until policy admits them. A read/search/resource tool result should return `ContentRef`s, source hashes, anchors, MIME/type metadata, detected file kind, parser pipeline, bounds, truncation state, media/document/archive metadata, warnings, and redacted summaries. If the result should influence a later provider call, it creates a `ContextContribution`; only `PostTool` policy and the context assembler can admit that contribution as a `ContextItem` in a `ContextProjection`.

## Format-Aware Read Requirements

The SDK toolkit read surface is one simple operation, but internally it must detect and route formats through typed readers:

- text, Markdown, JSON, and code: bounded UTF-8 with anchors only when the backing source is editable;
- PDF: extracted text plus page/document metadata, OCR-needed warnings for scanned or empty-text PDFs, and bounded OCR sidecar fallback when the host/workspace provides one;
- image: dimensions, color/type metadata, parser warnings, bounded OCR sidecar fallback when present, and no raw pixel bytes in model-visible text by default;
- RAW / Apple Photos-style media: RAW/DNG/HEIC/AVIF detection, dimensions where parsable, uncompressed strip and embedded preview metadata when available, bounded `.AAE` sidecar summaries, and explicit warnings when camera-specific demosaicing or Photos adjustment application is not available;
- Office OpenXML: DOCX/PPTX/XLSX text extraction from safe ZIP/XML reads;
- legacy Office: `.doc`/`.xls`/`.ppt` bounded sidecar/fallback extraction with limited-fidelity warnings unless a host converter is attached;
- archives: safe ZIP/TAR/TGZ/GZIP listing and later explicit entry reads, with path traversal and decompression limits;
- SQLite: read-only/query-only schema and bounded sample reads with no extension loading or writes;
- URI/resource: local data URLs may be decoded through the same output shape; live network/resource reads require host policy/resolver adapters and fail closed by default;
- fallback binary: bounded summary only.

Detection must prefer magic bytes over extension, use extension as a subtype hint, and fall back to UTF-8 validation before declaring opaque binary. Parser failures should be typed tool failures or structured warnings; they must not silently return lossy binary text. Oversized safe files return a bounded prefix with `truncated: true` and model-facing guidance to use search/grep or range reads; full-file parser adapters should not load files beyond the policy cap.

## Effect Class, Intent, And Reversibility Matrix

Every tool call, including reads, records auditable tool execution intent/result. Mutation and external visibility are separate from approval defaults and reversibility metadata.

| Effect class | External mutation? | Requires approval by default | Reversibility metadata |
| --- | --- | --- | --- |
| read | no | no, if in read scope | source/hash/truncation |
| anchored edit | yes | yes | before/after hash, diff, inverse candidate |
| write/overwrite | yes | yes | before/after hash, created/deleted path, non-reversible marker if needed |
| broad AST apply | yes | yes, second decision after preview | later optional crate only; preview ref, changed paths, inverse candidates |
| shell/process | yes | yes | command summary, stdout/stderr refs, exit/status, non-reversible marker |
| network send | yes | yes | dedupe key, ack ref, non-reversible marker |
| memory write | yes | policy-dependent | memory write receipt, idempotency key |
| remote message send | yes | yes/source-scoped | dedupe key, channel ack |

The shared `ToolRecord` must contain or map one-to-one to `EffectIntent { kind: ToolExecution }` before executor start and `EffectResult` after terminal status for all rows. Mutating file/process/network/memory behavior adds the relevant effect metadata and family-specific effect kind when applicable; read tools still record request, source, bounds, hashes, truncation, policy refs, and result refs for audit and replay.

## Idempotency And Reconciliation

Mutating packs must make replay and crash windows explicit. They cannot rely on "best effort" host behavior outside the journal.

Required mutating fields:

- `idempotency_key` for retryable operations, or an explicit non-idempotent reason and policy ref when retry is unsafe;
- `dedupe_key` for externally visible sends or operations where duplicate delivery is possible;
- before/after hashes or state refs for file, archive, structured-data, and memory mutations;
- external operation or receipt refs for process, network, MCP, memory, extension, or output-delivery effects;
- reconciliation adapter/ref, unsafe-pending reason, and terminal append status for unknown or crash-window outcomes;
- inverse candidate or non-reversible marker, with the inverse treated as advisory metadata rather than an automatic undo promise.

If `ToolRecord { intent }` cannot be appended, the mutating executor must not start. If an external side effect occurs but terminal result append fails, the run must enter recovery before another non-idempotent operation begins. Anti-entropy may repair derived indexes, telemetry cursors, package views, or known terminal records through the recorded reconciler; it must not rerun a non-idempotent tool or silently compensate external reality.

## Edit Flow

```mermaid
sequenceDiagram
  participant Model
  participant Read as "Read/Search"
  participant Planner as "WorkspaceEditPlanner"
  participant Approval
  participant Apply as "PatchApplier"
  participant Journal

  Model->>Read: "read/search"
  Read->>Journal: "ToolRecord: read intent/result with ContentRef"
  Read-->>Model: "anchors + hashes"
  Model->>Planner: "edit request with anchors"
  Planner->>Journal: "EditPlanCreated"
  Planner-->>Model: "preview diff"
  Model->>Approval: "apply request"
  Approval-->>Apply: "approved"
  Apply->>Journal: "ToolRecord: EffectIntent { kind: ToolExecution }"
  Apply->>Journal: "EffectIntent { kind: FileWrite }"
  Apply-->>Journal: "EffectResult { effect_id, before/after hash, reconciliation }"
```

## Reversibility Boundary

The SDK records effect lineage. It does not promise universal undo.

Required effect metadata:

- before hash
- after hash
- diff summary
- created/deleted paths
- idempotency key
- inverse patch candidate when safe
- formatter/diagnostic output ref
- external side-effect warning

Tool-pack journal records may add file, process, network, or memory-specific fields, but every tool call must embed or map one-to-one to tool execution intent/result records. Mutating packs additionally use `EffectKind::FileWrite`, `EffectKind::ProcessStart`, `EffectKind::ProcessSignal`, or the relevant memory/output kind as appropriate; they must not invent a separate file/process effect spine.

Evolution/product undo UX remains host-owned.

## Shell Process Ownership And Detach

Shell/process tools start agent-owned child artifacts by default.

```rust
// Non-compiling contract sketch.
pub struct ShellToolSpec {
    pub argv: Vec<String>,
    pub cwd: WorkspaceRef,
    pub env: RedactedEnv,
    pub timeout_ms: u64,
    pub isolation_requirement: Option<IsolationRequirementRef>,
    pub ownership: ProcessOwnershipPolicy,
    pub detach: Option<DetachRequest>,
}
```

Rules:

- A shell process that starts and exits during the tool call is recorded as normal `ToolRecord` and `IsolationRecord` state.
- A long-running process that continues after tool completion requires `detach: Some(...)`, allowed `DetachPolicy`, explicit user or host intent when configured, host acknowledgement, and `ChildLifecycleRecord` detach records.
- Without explicit detach, returning success while a process continues running is denied as an implicit orphan.
- Manual run cancellation terminates or interrupts agent-owned shell processes by default.
- `BeforeToolCall` hooks may deny or narrow shell requests through typed responses, but cannot silently change process ownership from agent-owned to detached or host-managed.
- Process output uses content refs/redacted summaries by default, including for detached processes.

## Acceptance Tests

- `read_output_includes_anchor_hash_truncation_and_mime`
- `edit_anchor_uses_hashline_snapshot_not_plain_line_number`
- `grep_respects_limits_and_reports_regex_compile_error`
- `symlink_policy_blocks_out_of_workspace_target`
- `edit_preview_does_not_write`
- `edit_apply_fails_on_stale_anchor_without_known_before_state`
- `write_requires_create_or_overwrite_scope`
- `shell_requires_timeout_and_sandbox_policy`
- `raw_shell_string_requires_high_risk_approval`
- `tool_discovery_activation_creates_package_delta`
- `tool_discovery_search_is_read_only_until_package_delta`
- `tool_pack_snapshot_includes_capability_sidecar_policy_and_executor_refs`
- `tool_pack_fingerprint_changes_when_executor_policy_or_sidecar_changes`
- `mutating_tool_records_effect_metadata`
- `read_tool_records_execution_intent_and_result`
- `mutating_tool_requires_idempotency_or_non_retryable_policy`
- `mutating_tool_reconciliation_required_for_unknown_terminal_status`
- `tool_intent_append_failure_prevents_tool_executor_start`
- `agent_sdk_core_builds_without_toolkit_features`
- `runtime_package_accepts_external_tool_pack_contract_without_core_tool_impl`
- `url_read_requires_network_permission`
- `memory_uri_read_requires_memory_permission`
- `mcp_resource_read_requires_allowlist`
- `non_idempotent_mutation_requires_intent_record_before_execute`
- `write_without_inverse_candidate_is_marked_non_reversible`
- `broad_ast_apply_requires_preview_and_second_policy_decision`
- `lsp_debugger_and_browser_tools_are_out_of_core_toolkit_slice`
- `tool_pack_preset_lowers_to_explicit_tool_specs`
- `tool_pack_helper_and_explicit_specs_emit_equivalent_tool_events`
- `shell_process_defaults_to_agent_owned_cleanup`
- `start_script_detach_requires_explicit_policy_and_journal_record`
- `manual_cancel_terminates_agent_owned_shell_process_by_default`
- `implicit_orphan_shell_process_is_denied`
- `before_tool_hook_cannot_silently_detach_process`

## Ergonomics

Simple API:

```rust
// Non-compiling contract sketch.
let package = RuntimePackage::for_agent(agent)
    .tool_pack(ToolPackPreset::WorkspaceReadOnly)
    .tool_pack(ToolPackPreset::WorkspaceEditWithApproval)
    .build()?;
```

Advanced API:

```rust
// Non-compiling contract sketch.
let pack = ToolPackBuilder::new(ToolPackId::new("workspace_safe"))
    .tool(edit_tool_spec)
    .policy(PolicyRef::new("approval.write_file"))
    .permission(Permission::WorkspaceWrite)
    .build()?;
```

Canonical lowering:

- `ToolPackPreset::WorkspaceReadOnly` expands into explicit read/list/search `CapabilitySpec`s, executor refs, permission refs, redaction policy refs, and a typed tool-pack sidecar.
- `ToolPackPreset::WorkspaceEditWithApproval` expands into preview/apply `CapabilitySpec`s with approval policy refs, effect-lineage requirements, idempotency hints, and reconciliation metadata.
- Runtime package fingerprint includes the lowered specs, sidecar versions, executor refs, schemas, policy refs, isolation/detach policy, and redaction refs, not the preset display name.

Equivalence:

- Preset tools and manually declared tool specs emit the same tool/approval events and journal records.
- Presets cannot bypass approval, sandbox, effect metadata, or package projection/execution checks.

SDK owns / Host owns:

- SDK owns preset definitions for optional toolkit crates and their lowering into tool specs.
- Host owns whether a preset is installed, which workspace roots are trusted, and whether product-specific undo UI exists.

Tests:

- `tool_pack_preset_lowers_to_explicit_tool_specs`
- `tool_pack_helper_and_explicit_specs_emit_equivalent_tool_events`
- `mutating_tool_records_effect_metadata`

## Complete Example

Typed shape:

```rust
// Non-compiling contract sketch.
let edit_tool = ToolSpec {
    tool_spec_id: ToolSpecId::new("tool_spec.workspace_edit.v1"),
    capability_id: CapabilityId::new("cap.sdk.workspace_edit.v1"),
    canonical_name: CanonicalToolName::new("workspace_edit"),
    namespace: CapabilityNamespace::new("sdk.workspace"),
    source: ToolSourceRef::sdk_toolkit("workspace_edit"),
    source_ref: SourceRef::sdk_toolkit("sdk.workspace_edit_pack.v1"),
    destination: DestinationRef::tool("workspace_edit"),
    executor_ref: ExecutorRef::tool("toolkit.workspace_edit.v1"),
    policy_refs: vec![PolicyRef::new("approval.write_file")],
    sidecar_ref: PackageSidecarRef::tool_pack("sdk.workspace_edit_pack.v1"),
    input_schema: SchemaRef::content("schemas/workspace_edit_input_v1"),
    output_schema: SchemaRef::content("schemas/workspace_edit_output_v1"),
    effect_class: EffectClass::AnchoredEdit,
    risk_class: RiskClass::High,
    required_permissions: vec![Permission::WorkspaceWrite],
    isolation_requirement: None,
    idempotency_hint: IdempotencyHint::RequiresAnchorHash,
    reconciliation: ReconciliationRequirement::BeforeAfterHash,
    timeout_ms: 10_000,
    cancellation: CancellationSupport::BestEffort,
    process_ownership: Some(ProcessOwnershipPolicy::agent_owned(parent_run_id)),
    privacy: ToolPrivacyPolicy::ContentRefsOnly,
    result_redaction: RedactionPolicyId::new("tool_result_default"),
};

let request = WorkspaceEditRequest {
    path: WorkspacePath::new("docs/notes.md"),
    anchor: HashLineAnchor { line: 42, before_hash: ContentHash::sha256("...") },
    replacement_ref: ContentRef::new("patch/replacement_1"),
    preview_only: false,
    preview_hash: Some(ContentHash::sha256("preview-before-apply")),
};
```

Replaceable ports:

- `ToolPack` registers specs and executors through the core tool contract.
- `WorkspaceReader`, `WorkspaceSearcher`, `PatchPlanner`, `PatchApplier`, and `ShellExecutor` are swappable toolkit ports.
- Host products can supply their own tool packs if they satisfy the same spec, effect, approval, and journal rules.

Wiring:

1. Runtime package includes the tool spec as a `CapabilitySpec` plus a typed tool-pack sidecar and executor ref.
2. Model sees projected schema only if the package validates.
3. Edit planner produces preview diff without mutation.
4. Approval policy gates apply.
5. Applier appends tool execution intent, verifies anchor hash, appends file-write intent, writes, records before/after hashes, and returns effect/reconciliation metadata.

`AgentEvent` variants:

- `ToolRequested`
- `ToolApprovalRequired`
- `ApprovalRequested`
- `ToolStarted` only after intent append
- `ToolCompleted` or `ToolFailed`

`RunJournal` records:

- `ToolRecord { requested }`
- `ToolRecord { edit plan preview }`
- `ApprovalRecord { apply decision }`
- `ToolRecord { EffectIntent { kind: ToolExecution } }`
- `ToolRecord { EffectIntent { kind: FileWrite } }` or one-to-one mapped file-write payload
- `ToolRecord { EffectResult, before_hash, after_hash, inverse_candidate, reconciliation_ref }`

Policies and failures:

- Stale anchors fail without writing.
- Shell commands require timeout and sandbox policy.
- Missing policy refs, executor refs, or intent append fail closed before execution.
- Shell processes are agent-owned by default and cannot outlive the run without explicit detach records.
- Non-reversible writes are marked explicitly.
- LSP/debugger/browser tools stay out of the first core toolkit slice.

SDK owns / Host owns:

- SDK owns tool spec schema, effect metadata contract, process ownership/detach contract, optional toolkit boundaries, and reversible lineage fields.
- Host owns workspace trust, mounted roots, concrete file/process access, detached process inspection/reclaim UI, product undo UX, and whether optional tool packs are installed.

Tests:

- `edit_anchor_uses_hashline_snapshot_not_plain_line_number`
- `edit_apply_fails_on_stale_anchor_without_known_before_state`
- `mutating_tool_records_effect_metadata`
- `read_tool_records_execution_intent_and_result`
- `tool_pack_snapshot_includes_capability_sidecar_policy_and_executor_refs`
- `tool_intent_append_failure_prevents_tool_executor_start`
- `start_script_detach_requires_explicit_policy_and_journal_record`
