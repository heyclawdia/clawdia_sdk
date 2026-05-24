# Phase 06: P2 Side Effects

Add tool, approval, output, hook, and telemetry side effects over the P1 loop. All launch targets in this folder may run in parallel after Phase 05 exits.

## Launch Targets

| Title | Run in parallel? | Purpose |
| --- | --- | --- |
| [Approval Broker](06a-approval-broker.md) | yes | Permission, approval, escalation, dispatcher absence, and fail-closed policy tests. |
| [Tool Execution](06b-tool-execution.md) | yes | Tool registry/router/executor, intent-before-execution, and read/write result records. |
| [Output Delivery](06c-output-delivery.md) | yes | Destination refs, sink dispatch, dedupe, delivery policy, and repair replay. |
| [Hook Lifecycle](06d-hook-lifecycle.md) | yes | Hook specs, config/code lowering, mutation rights, and journal-before-apply. |
| [Telemetry Core](06e-telemetry-core.md) | yes | Derived telemetry fanout, privacy/cost policy, bounded sinks, and failure isolation. |

## Exit Gate

- [ ] Every mutating or externally visible action appends intent before execution and terminal result after execution.
- [ ] Missing policy, dispatcher, adapter, sink, or journal append fails closed where required.
- [ ] P2 tests prove tools/approval without creating a second run loop, event stream, journal, package registry, policy path, or side-effect path.
- [ ] Phase exit report records reviewer PASS.
