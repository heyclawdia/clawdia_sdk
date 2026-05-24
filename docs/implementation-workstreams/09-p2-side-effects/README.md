# Phase 09: P2 Side Effects

Add tool, approval, output, hook, and telemetry side effects over the P1 loop. All launch targets in this folder may run in parallel after Phase 08 exits.

## Launch Targets

| Title | Run in parallel? | Purpose |
| --- | --- | --- |
| [Approval Broker](09a-approval-broker.md) | yes | Permission, approval, escalation, dispatcher absence, and fail-closed policy tests. |
| [Tool Execution](09b-tool-execution.md) | yes | Tool registry/router/executor, intent-before-execution, and read/write result records. |
| [Output Delivery](09c-output-delivery.md) | yes | Destination refs, sink dispatch, dedupe, delivery policy, and repair replay. |
| [Hook Lifecycle](09d-hook-lifecycle.md) | yes | Hook specs, config/code lowering, mutation rights, and journal-before-apply. |
| [Telemetry Core](09e-telemetry-core.md) | yes | Derived telemetry fanout, privacy/cost policy, bounded sinks, and failure isolation. |

## Exit Gate

- [x] Every mutating or externally visible action appends intent before execution and terminal result after execution.
- [x] Missing policy, dispatcher, adapter, sink, or journal append fails closed where required.
- [x] P2 tests prove tools/approval without creating a second run loop, event stream, journal, package registry, policy path, or side-effect path.
- [x] Phase exit report records reviewer PASS.
