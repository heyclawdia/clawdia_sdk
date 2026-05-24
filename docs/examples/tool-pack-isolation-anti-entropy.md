# Tool Pack, Isolation, And Anti-Entropy Workflow

This example shows a complex coding-agent-style workflow without turning the SDK into a coding product.

## Workflow

```mermaid
sequenceDiagram
  participant Host
  participant Package as "RuntimePackage"
  participant Search as "workspace_search"
  participant Edit as "workspace_edit"
  participant Approval
  participant Iso as "IsolationRuntime"
  participant Shell as "shell tool"
  participant Hook as "HookBus"
  participant Journal
  participant Telemetry
  participant AE as "AntiEntropyJob"

  Host->>Package: "add read/search/edit/shell packs"
  Package->>Journal: "package fingerprint"
  Host->>Search: "grep with limits"
  Search-->>Host: "anchors + hashes"
  Host->>Edit: "plan anchored edit"
  Edit->>Journal: "EditPlanCreated"
  Edit-->>Host: "preview diff"
  Host->>Approval: "apply approval"
  Approval-->>Edit: "approved"
  Edit->>Journal: "EffectIntent { kind: FileWrite }"
  Edit-->>Journal: "EffectResult { effect_id }"
  Host->>Approval: "shell approval"
  Approval-->>Shell: "approved"
  Shell->>Hook: "BeforeToolCall"
  Hook->>Journal: "HookInvoked"
  Hook-->>Shell: "allow/narrow/deny typed response"
  Hook->>Journal: "HookResponseApplied"
  Shell->>Iso: "prepare isolated env"
  Iso->>Journal: "IsolationEnvironmentPrepared"
  Shell->>Journal: "EffectIntent { kind: IsolatedProcessStart }"
  Shell->>Iso: "start process"
  Iso-->>Shell: "stdout/stderr refs + exit"
  Shell->>Journal: "EffectResult { effect_id } + IsolationProcessIoCaptured + IsolationProcessStatsRecorded + IsolationProcessExited + ToolCompleted"
  Journal->>Telemetry: "usage/tool/isolation cost export"
  AE->>Journal: "scan pending effects/export cursors"
```

## Long-Running Process Detach

```mermaid
sequenceDiagram
  participant User as "Host/user request"
  participant Shell as "shell tool"
  participant Policy as "RunChildLifecyclePolicy"
  participant Journal
  participant Host as "Host process owner"

  User->>Shell: "start script and leave it running"
  Shell->>Journal: "ToolRecord: process start intent"
  Shell->>Policy: "detach allowed?"
  alt detach denied
    Policy-->>Shell: "deny implicit orphan"
    Shell->>Journal: "ToolFailed: detach denied"
  else detach allowed
    Shell->>Journal: "ChildLifecycleDetachRequested"
    Shell->>Host: "request ownership ack"
    Host-->>Shell: "host_ack_ref + reclaim policy"
    Shell->>Journal: "ChildLifecycleDetachAcknowledged + ChildLifecycleDetached"
    Shell-->>User: "started, tracked by host owner"
  end
```

Manual cancellation of the parent run uses the same policy but defaults to shutdown: non-detached shell processes receive signal/terminate intent before the adapter is called, then terminal cleanup or recovery records are appended.

## Package Delta For Tool Discovery

```mermaid
flowchart TD
  A["Model sees small tool set"] --> B["search_tool_discovery"]
  B --> C["candidate hidden tool"]
  C --> D{"Activation policy"}
  D -->|denied| E["no package change"]
  D -->|approved| F["PackageDeltaRequested"]
  F --> G["Next-turn RuntimePackage fingerprint"]
```

Tool discovery never mutates the active package ambiently.

## Stream Rule Safety

```mermaid
flowchart TD
  A["Tool stdout/stderr StreamDelta"] --> B["StreamRuleEngine"]
  B --> C{"secret or unsafe marker?"}
  C -->|no| D["bounded redacted progress event"]
  C -->|yes mask| E["StreamRuleMatched"]
  E --> F["StreamInterventionApplied"]
  F --> G["masked output before sink/telemetry"]
  C -->|yes stop| H["StreamInterventionRequested"]
  H --> I["policy-approved stop/cancel path"]
```

Stream rules observe bounded stream deltas and content refs. They cannot read hidden reasoning, bypass tool policy, or deliver output directly.

## Anti-Entropy Repair

```mermaid
flowchart TD
  A["Crash after shell process exits"] --> B["Intent exists, terminal append missing"]
  B --> C["Resume replay detects pending side effect"]
  C --> D["Process reconciliation adapter"]
  D --> E{"Process status known?"}
  E -->|yes| F["Append terminal ToolRecord/IsolationRecord"]
  E -->|no| G["RepairNeeded: host action"]
  F --> H["Re-export telemetry"]
```

## Host-Owned Boundaries

- Which optional tool packs are installed or enabled.
- Workspace roots, symlink policy, and mount policy.
- Approval UI and autonomy settings.
- Concrete isolation runtime adapter.
- Host analytics exporter and repair scheduling.
- Product UX for review, undo, or recommendation flows.

## Events, Journals, Telemetry, And Recovery

- Events: tool, approval, hook, stream-rule, isolation, child-lifecycle, output-delivery, telemetry, and recovery families.
- Journal records: `ToolRecord`, `ApprovalRecord`, `HookRecord`, `StreamRuleRecord`, `IsolationRecord`, `ChildLifecycleRecord`, `OutputDispatchRecord`, `TelemetryRecord`, and `RecoveryRecord`.
- Policy decisions: tool permission, approval/autonomy/escalation, hook mutation rights, stream intervention, isolation class/capability/trust downgrade, child lifecycle detach/reclaim, redaction/content-capture, and telemetry sink policy.
- Telemetry/cost: tool attempts, hook latency, isolated process stats, stream-rule interventions, output delivery, and repair cursor status are projections from journal-backed events.
- Recovery: anti-entropy can append terminal records or require host action, but it never reruns file writes, shell processes, output sends, provider calls, memory writes, extension actions, or detached process ownership transfers without idempotency or explicit repair policy.

## Acceptance Tests

- `anchored_edit_rejects_stale_anchor_before_write`
- `tool_discovery_activation_creates_next_snapshot_only`
- `non_idempotent_mutation_requires_intent_record_before_execute`
- `container_required_denies_hostprocess_fallback`
- `journal_terminal_append_failure_after_side_effect_enters_recovery_and_blocks_more_side_effects`
- `anti_entropy_repairs_telemetry_summary_cursor_without_rerunning_agent`
- `start_script_detach_requires_explicit_policy_and_journal_record`
- `manual_cancel_terminates_agent_owned_shell_process_by_default`
- `before_tool_hook_cannot_silently_detach_process`
- `stream_rule_masks_secret_before_tool_output_delivery`
