# Desktop Chat With Tool Approval

This example shows a normal desktop or web chat path as SDK contracts.

## Sequence

```mermaid
sequenceDiagram
  participant UI as "Desktop chat UI"
  participant Host as "Host app"
  participant Runtime as "AgentRuntime"
  participant Context as "ContextAssembler"
  participant Provider as "ProviderAdapter"
  participant Policy as "Policy stack"
  participant Broker as "ApprovalBroker"
  participant Tool as "ToolExecutor"
  participant Journal as "RunJournal"
  participant Telemetry as "TelemetryFanout"

  UI->>Host: "send message + attachments"
  Host->>Runtime: "RunRequest(SourceRef::host_surface, DestinationRef::output_sink)"
  Runtime->>Journal: "RunRecord: RunStarted"
  Runtime->>Context: "assemble messages/memory/files/tools"
  Context->>Journal: "ContextRecord + ContextProjectionAudited"
  Runtime->>Provider: "stream projected request"
  Provider-->>Runtime: "ModelStreamDelta"
  Runtime-->>UI: "ModelStreamDelta live event through host bridge"
  Provider-->>Runtime: "tool_use"
  Runtime->>Journal: "ModelAttemptRecord + ToolRecord intent"
  Runtime->>Policy: "classify tool"
  Policy-->>Runtime: "Ask"
  Runtime->>Broker: "ApprovalRequest"
  Broker-->>UI: "host approval prompt"
  UI-->>Broker: "approve / approve_for_session / deny"
  Broker->>Journal: "ApprovalRecord dispatch intent/result"
  Runtime->>Tool: "execute approved tool"
  Tool->>Journal: "ToolStarted / ToolCompleted"
  Tool-->>Runtime: "ToolResultEnvelope"
  Runtime->>Provider: "continue with tool result"
  Provider-->>Runtime: "final assistant message"
  Runtime->>Journal: "MessageRecord + output delivery + RunCompleted"
  Runtime->>Telemetry: "usage/cost/tool/approval rollup"
```

## Event Families

- `run_lifecycle`
- `turn_lifecycle`
- `message`
- `model`
- `memory_context`
- `tool`
- `approval`
- `telemetry_cost`
- `output_delivery`

## Journal Records

- `RunRecord`
- `TurnRecord`
- `ContextRecord`
- `MessageRecord`
- `ModelAttemptRecord`
- `ApprovalRecord`
- `ToolRecord`
- `OutputDispatchRecord`
- `TelemetryRecord`
- `RecoveryRecord` when tool/result/output terminal append is unsafe

## Policy, Telemetry, And Recovery

- Policy decisions: context projection policy, tool permission policy, approval/escalation policy, redaction/content-capture policy, and output delivery policy.
- Telemetry/cost: model usage, tool attempts, approval latency, output delivery status, and final run status are derived from journal-backed events.
- Recovery: if a tool or output send may have happened but terminal append fails, the run enters recovery before another non-idempotent side effect starts. UI event loss never becomes run truth.

## Host-Owned Boundaries

- UI prompt copy and rendering.
- Desktop or web event transport.
- Conversation persistence.
- Any temporary compatibility fail-open policy, if enabled, lives in the host adapter and emits compatibility events.

## Acceptance Tests

- `desktop_chat_tool_approval_sequence_matches_contract`
- `desktop_transport_failure_uses_explicit_compat_policy_not_sdk_default`
- `tool_started_never_precedes_approval_when_policy_asks`
- `projection_audit_precedes_provider_call`
