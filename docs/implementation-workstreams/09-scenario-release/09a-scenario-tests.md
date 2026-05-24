# Scenario Tests

## Phase

[Phase 09: Scenario Release](README.md)

## Parallelism

Parallel-safe with [API Review](09b-api-review.md) and [Release Readiness](09c-release-readiness.md).

## Contract Inputs

- [examples/README.md](../../examples/README.md)
- [desktop-chat-tool-approval.md](../../examples/desktop-chat-tool-approval.md)
- [remote-headless-approval.md](../../examples/remote-headless-approval.md)
- [structured-output-stream-rules.md](../../examples/structured-output-stream-rules.md)
- [extension-action-boundary.md](../../examples/extension-action-boundary.md)

## Implementation Objective

Prove representative generic host workflows compose from SDK primitives without importing product behavior into core.

## Owned Implementation Surface

- `crates/agent-sdk-core/tests/scenarios/`
- optional scenario tests under feature crates when relevant
- scenario fixture outputs under `crates/agent-sdk-core/tests/fixtures/scenarios/`

## Must Deliver

- Fake workflows for desktop/web chat approval, CLI/headless approval, remote output dedupe, realtime/stream safeguard, structured output, memory/context compaction, tool-pack/isolation repair, subagent supervision, extension action boundary, and live-vs-durable event flow.
- Scenario matrix mapping each workflow to SDK primitives and host-owned boundaries.
- Tests that fail if product-specific host behavior enters core.

## Validation

- `cargo test -p agent-sdk-core --test scenario_matrix`
- optional crate scenario tests where relevant
- product-neutrality scenario audit

## Must Not

- Use live providers, product UI, real remote channels, real containers, or product trace stores as first proof.
