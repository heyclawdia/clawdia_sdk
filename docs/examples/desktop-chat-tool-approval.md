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
  Host->>Runtime: "RunRequest(SourceRef::desktop, DestinationRef::ui)"
  Runtime->>Journal: "RunRecord: RunStarted"
  Runtime->>Context: "assemble messages/memory/files/tools"
  Context->>Journal: "ContextRecord + ContextProjectionAudited"
  Runtime->>Provider: "stream projected request"
  Provider-->>Runtime: "ModelStreamDelta"
  Runtime-->>UI: "ModelStreamDelta live event"
  Provider-->>Runtime: "tool_use"
  Runtime->>Journal: "ModelAttemptRecord + ToolRecord intent"
  Runtime->>Policy: "classify tool"
  Policy-->>Runtime: "Ask"
  Runtime->>Broker: "ApprovalRequest"
  Broker-->>UI: "host approval prompt"
  UI-->>Broker: "approve / approve_for_session / deny"
  Broker->>Journal: "ApprovalRecord"
  Runtime->>Tool: "execute approved tool"
  Tool->>Journal: "ToolStarted / ToolCompleted"
  Tool-->>Runtime: "ToolResultEnvelope"
  Runtime->>Provider: "continue with tool result"
  Provider-->>Runtime: "final assistant message"
  Runtime->>Journal: "MessageRecord + RunCompleted"
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
- `TelemetryRecord`

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
