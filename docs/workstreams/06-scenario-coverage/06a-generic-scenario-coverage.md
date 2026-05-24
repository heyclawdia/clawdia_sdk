# Goal 06a: Generic Scenario Coverage

## Phase

[Phase 06: Scenario Coverage](README.md)

## Owner Role

[Generic Scenario Coverage](../_roles/10-generic-scenario-coverage.md)

## Parallelism

Only goal in Phase 06. Run after every Phase 05 goal exits. Do not start Phase 07 until this goal passes.

## Required Reading

- `README.md`
- `docs/start-here.md`
- `coding_standards.md`
- `docs/workstreams/validation-gates.md`
- `docs/reference/sdk-review-checklist.md`
- `docs/architecture/primitive-map.md`
- phase README
- owner role doc
- all prior phase exit reports
- read-only inputs below

## Writable Files

- `docs/examples/*`

## Read-Only Inputs

- all SDK contracts
- all architecture docs
- `docs/workstreams/validation-gates.md`
- `docs/reference/feature-to-primitive-matrix.md`
- `docs/reference/open-questions-and-ambiguities.md`
- all prior phase exit reports

## Primitive Focus

- Prove desktop, CLI/headless, realtime, remote channel, external runtime, memory, subagent, isolation, telemetry, and output-delivery scenarios compose from the same primitives.
- Mark product UI, routing, credentials, stores, marketplaces, dashboards, and workflow policies as host-owned.

## Must Not Own

Core contract changes, shared primitive names, runtime-package fingerprint fields, or product-specific examples in active SDK contracts.

## Validation And Review

- Scenario mapping table: scenario -> SDK primitives -> host-owned boundaries -> events/journals -> validation.
- Proposal blocks for any missing primitive; no direct edits to shared architecture/reference docs.
- Product-neutrality audit over examples.

## Validation Evidence

- Worker: serialized Phase 06 scenario coverage owner.
- Changed example files: `docs/examples/README.md`, `docs/examples/desktop-chat-tool-approval.md`, `docs/examples/realtime-voice-workflow.md`, `docs/examples/remote-headless-approval.md`, `docs/examples/external-runtime-session-lifecycle.md`, `docs/examples/live-vs-durable-event-flow.md`, `docs/examples/memory-context-compaction.md`, `docs/examples/subagent-supervision-workflow.md`, `docs/examples/tool-pack-isolation-anti-entropy.md`, `docs/examples/structured-output-stream-rules.md`, and `docs/examples/extension-action-boundary.md`.
- Added a scenario coverage matrix mapping every role-listed scenario to SDK primitives, host-owned boundaries, events/journals/telemetry/recovery, and validation.
- Added product-neutral examples for typed structured output plus stream-rule intervention, and extension capability/action boundaries.
- Updated existing examples to use Phase 05 event names and to name policy decisions, telemetry/cost projection, and recovery behavior.
- No Rust source, package manifests, executable tests, or fixtures were created.

## Review Packet

- Primitive decision: scenarios are coverage constraints over the existing kernel and feature layers; no new core primitive or capability variant is proposed.
- SDK-owned boundaries preserved: examples route behavior through `RuntimePackage`, `RunRequest`, `AgentEvent`, `RunJournal`, `PolicyRef`, `EffectIntent` / `EffectResult`, typed refs, feature sidecars, and replaceable ports.
- Host-owned boundaries preserved: UI, channel transport, credentials, app-event stores, trace stores, extension manifests/runtimes, memory backends, marketplaces, dashboards, workflow routing, process ownership, retry scheduling, and product copy stay outside core.
- Proposal status: no missing primitive proposals were found in Phase 06 scenario coverage.
