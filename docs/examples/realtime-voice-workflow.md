# Realtime Voice Workflow

Voice is not "chat with audio." It has send/receive halves, connection lifecycle, wake/listening UI, transcript finality, interruptions, and source-scoped approval.

## Wake To Final Output

```mermaid
sequenceDiagram
  participant Voice as "Voice extension / wake recognizer"
  participant Host as "Host app"
  participant UI as "Voice surface / composer UI"
  participant Runtime as "AgentRuntime"
  participant RT as "RealtimeProviderAdapter"
  participant Broker as "ApprovalBroker"
  participant Tool as "ToolExecutor"
  participant Journal as "RunJournal"

  Voice->>Host: "wake phrase detected"
  Host-->>UI: "host live event: voice activity"
  Host->>Runtime: "RunRequest SourceRef::extension"
  Runtime->>Journal: "RunStarted"
  Runtime->>RT: "connect send/receive halves"
  RT-->>Runtime: "RealtimeConnected"
  Runtime->>Journal: "RealtimeSessionRecord: connected"
  Voice->>Runtime: "audio ContentRef / bounded chunks"
  Runtime->>Journal: "RealtimeInputSent"
  Runtime->>RT: "send input frame"
  RT-->>Runtime: "transcript delta"
  Runtime->>Journal: "RealtimeOutputReceived"
  Runtime-->>UI: "bounded live transcript event"
  alt user interrupts
    Voice->>Runtime: "interruption(response_id)"
    Runtime->>Journal: "RealtimeInterrupted with response_id"
    Runtime->>RT: "cancel current response"
  end
  alt connection timeout
    RT-->>Runtime: "connection timeout"
    Runtime->>Journal: "RealtimeRestartRequested"
    Runtime->>Journal: "RealtimeRestartStarted"
    Runtime->>RT: "restart"
    Runtime-->>Voice: "gate outbound frames until connected"
    alt restart succeeds
      RT-->>Runtime: "connected"
      Runtime->>Journal: "RealtimeRestartCompleted"
    else restart fails
      RT-->>Runtime: "restart error"
      Runtime->>Journal: "RealtimeRestartFailed"
    Runtime-->>UI: "bounded failure state"
    end
  end
  alt tool requested
    RT-->>Runtime: "tool request"
    Runtime->>Broker: "approval request if policy asks"
    Broker-->>Host: "source-scoped approval"
    Host-->>Broker: "exact yes/no token or UI decision"
    Runtime->>Tool: "execute approved tool"
    Tool-->>Runtime: "ToolResultEnvelope"
    Runtime->>RT: "send tool result"
  end
  RT-->>Runtime: "final text/audio output"
  Runtime->>Journal: "RealtimeClosed + output delivery records"
  Runtime->>Journal: "RunCompleted after realtime/output/approval bookkeeping"
```

## Live Vs Durable

```mermaid
flowchart LR
  A["Realtime events"] --> B["RunJournal: durable session/interruption/tool/output records"]
  A --> C["Bounded host display events: voice activity"]
  A --> D["Telemetry: latency/usage/cost"]
  C --> E["Voice surface / composer selectors"]
```

Transcript UI events can drop. Journaled response IDs and interruption records cannot.

## Policy, Telemetry, And Recovery

- Policy decisions: media capture policy, realtime send/receive policy, stream intervention policy, source-scoped approval policy, restart/backpressure policy, and output delivery policy.
- Journal records: `RealtimeSessionRecord`, `StreamRuleRecord` when a stream rule intervenes, `ApprovalRecord`, `ToolRecord`, `OutputDispatchRecord`, `TelemetryRecord`, and `RecoveryRecord` when a connection or terminal append is unsafe.
- Telemetry/cost: latency, restart count, backpressure state, usage, tool calls, and output delivery status are derived from journal-backed events.
- Recovery: restart failure is observable before retry policy acts; raw audio is content-ref-only by default; terminal run completion waits for realtime close or explicit detach plus output/approval bookkeeping.

## Host-Owned Boundaries

- Wake phrase detection.
- Microphone permission.
- Voice extension settings.
- Voice activity rendering.
- Exact approval token transport.

## Acceptance Tests

- `realtime_restart_gates_outbound_audio_frames`
- `realtime_restart_records_requested_started_completed_in_order`
- `realtime_restart_failure_is_observable_before_retry_policy`
- `voice_tool_approval_cannot_use_source_extension_as_authority`
- `interruption_records_response_id_before_cancelling_output`
- `voice_app_event_loss_does_not_drop_realtime_journal_records`
