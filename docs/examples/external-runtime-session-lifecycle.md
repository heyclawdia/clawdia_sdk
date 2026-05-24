# External Runtime Session Lifecycle

External runtimes can preserve state across prompts, but their process/session lifecycle is host-owned. The SDK should expose portable launch, restore, send, receive, retire, event mapping, package fingerprint, and journal contracts without owning any specific runtime cache or UI surface.

## State Diagram

```mermaid
stateDiagram-v2
  [*] --> NoRuntime
  NoRuntime --> PrewarmRequested: "surface prewarm"
  PrewarmRequested --> RuntimeReady: "bootstrap ok"
  PrewarmRequested --> NoRuntime: "prewarm failed nonfatal"
  NoRuntime --> PromptRequested: "first prompt"
  RuntimeReady --> PromptRequested: "first prompt uses prewarm"
  PromptRequested --> RestoreAttempt: "restore key available"
  PromptRequested --> NewSession: "no restore"
  RestoreAttempt --> ActiveSession: "restore ok"
  RestoreAttempt --> NewSession: "restore failed"
  NewSession --> HistoryReplay: "sanitize/replay history"
  HistoryReplay --> ActiveSession
  ActiveSession --> LaterPrompt: "same surface"
  LaterPrompt --> ActiveSession: "fingerprint stable"
  LaterPrompt --> RetireRuntime: "fingerprint outdated"
  ActiveSession --> RetireRuntime: "over compaction threshold"
  ActiveSession --> RetireRuntime: "surface close/reset"
  ActiveSession --> Shutdown: "host shutdown"
  RetireRuntime --> NoRuntime
  Shutdown --> [*]
```

## ID Boundaries

| Concept | Owner | Must stay distinct |
| --- | --- | --- |
| `SurfaceSessionId` | host UI or channel surface | conversation ID |
| `RestoreKey` | host external-runtime adapter | runtime session ID |
| `ExternalRuntimeSessionId` | external runtime | SDK run ID |
| `RunId` | SDK run | surface session |
| `RuntimePackageFingerprint` | SDK/host package builder | external process ID |

## Adapter Event Flow

```mermaid
flowchart TD
  A["prewarm surface session"] --> B["Host runtime session cache"]
  B --> C["ExternalAgentAdapter.launch/restore"]
  C --> D["External runtime process/session"]
  D --> E["External events"]
  E --> F["AgentEvent mapping"]
  F --> G["RunJournal"]
  F --> H["Telemetry / trace sink"]
  F --> I["Host live stream"]
```

## Host-Owned Boundaries

- External command resolution and install bootstrap.
- Live runtime cache.
- Restore key policy.
- Prewarm timing.
- Retirement scheduling.
- Process cleanup.
- Surface UI and product retry messaging.

## Acceptance Tests

- `surface_session_restore_key_runtime_session_ids_do_not_collapse`
- `fingerprint_mismatch_retires_live_runtime`
- `over_threshold_external_turn_replays_compacted_history_on_next_bootstrap`
- `prewarm_failure_is_nonfatal_and_actionable`
- `surface_close_retires_runtime_without_sealing_conversation`
