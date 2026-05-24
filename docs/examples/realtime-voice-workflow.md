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
  Host->>Runtime: "RunRequest SourceRef::extension_voice"
  Runtime->>Journal: "RunStarted"
  Runtime->>RT: "connect send/receive halves"
  RT-->>Runtime: "RealtimeConnected"
  Voice->>Runtime: "audio ContentRef / bounded chunks"
  Runtime->>RT: "RealtimeInputSent"
  RT-->>Runtime: "transcript delta"
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
  Runtime->>Journal: "RunCompleted"
```

## Live Vs Durable

```mermaid
flowchart LR
  A["Realtime events"] --> B["RunJournal: durable connection/interruption/tool records"]
  A --> C["Bounded host display events: voice activity"]
  A --> D["Telemetry: latency/usage/cost"]
  C --> E["Voice surface / composer selectors"]
```

Transcript UI events can drop. Journaled response IDs and interruption records cannot.

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
