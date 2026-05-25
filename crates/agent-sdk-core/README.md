# agent-sdk-core

`agent-sdk-core` is the product-neutral primitive kernel for the Agent SDK. It owns typed domain primitives, runtime packages, records, public ports, run coordination, and reusable deterministic test support. Hosts own concrete providers, stores, UI, approval transport, containers, telemetry export, and product workflows.

## Public Surface

SDK consumers should import through the crate root and documented namespaces:

- `agent_sdk_core::prelude::*` for common app-building imports
- `agent_sdk_core::{Agent, AgentRuntime, RunRequest, RunHandle, RuntimePackage}`
- `agent_sdk_core::{AgentEvent, EventFrame, RunJournal, JournalRecord}`
- `agent_sdk_core::{AgentPool, AgentPoolStore, InMemoryAgentPoolStore, RunMessage, WakeCondition}`
- `agent_sdk_core::{OutputContract, ValidatedOutput, PolicyDecision}`
- `agent_sdk_core::ports` for host-implemented adapter contracts
- `agent_sdk_core::testing` for deterministic fake adapters and conformance helpers

`prelude` is facade-only: it re-exports stable crate-root items and does not add
helper behavior or bypass the canonical run, package, policy, event, journal,
telemetry, lineage, or redaction paths. Deep implementation modules are review
surfaces until a release explicitly blesses them.

## Package Shape

The crate follows the SDK package architecture gate:

| Folder | Responsibility |
| --- | --- |
| `src/domain/` | IDs, refs, policy, privacy, errors, and other ubiquitous-language primitives. |
| `src/package/` | Runtime package authority, capabilities, sidecars, catalogs, deltas, and fingerprints. |
| `src/records/` | Durable and observable records for events, journals, effects, context, content, output, and feature records. |
| `src/ports/` | Public trait boundaries for providers, journals, event buses, content, tools, sinks, isolation, extensions, realtime, and telemetry. |
| `src/application/` | Runtime coordination, state machines, canonical lowering, replay, recovery, and feature orchestration. |
| `src/testing/` | Deterministic fakes, scripted adapters, fixtures, and SDK-consumer test-kit helpers. |

Root integration tests are stable Cargo target shims. Full test bodies live under `tests/domain`, `tests/package`, `tests/records`, `tests/ports`, `tests/runtime`, `tests/feature_layers`, `tests/testing`, or `tests/scenarios`.

## Features

| Feature | Status |
| --- | --- |
| default | Empty. Core builds without optional toolkit, isolation, extension, OTel, workflow, host-adapter, network, or async-runtime dependencies. |
| test-support | Reserved for future gated test-kit expansion. Current deterministic fakes are part of the documented `agent_sdk_core::testing` namespace and exercise the same public ports as production code. |

## Unsupported In This Handoff

This crate does not ship live provider adapters, concrete container/runtime adapters, product UI adapters, remote-channel adapters, marketplace/extension host runtimes, network telemetry exporters, or workflow-engine ownership. Those remain host-owned or optional-crate responsibilities until a later release adds matching contracts and tests.
