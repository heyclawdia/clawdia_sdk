# Goal 02a: Core Run API

## Phase

[Phase 02: Primitive Kernel](README.md)

## Owner Role

[Core Api Runtime](../_roles/01-core-api-runtime.md)

## Parallelism

Parallel-safe with every other goal in Phase 02 after Phase 01 exits. Do not start Phase 03 until all Phase 02 goals finish.

## Required Reading

- `README.md`
- `docs/start-here.md`
- `coding_standards.md`
- `docs/workstreams/validation-gates.md`
- `docs/reference/sdk-review-checklist.md`
- `docs/architecture/primitive-map.md`
- phase README
- owner role doc
- read-only inputs below

## Writable Files

- `docs/contracts/api-contracts.md`
- `docs/contracts/run-handle-reconnect-contract.md`
- `docs/contracts/hook-lifecycle-contract.md` only where the core hook contract is explicitly in scope

## Read-Only Inputs

- `docs/contracts/runtime-package-schema.md`
- `docs/contracts/event-schema.md`
- `docs/contracts/journal-replay-schema.md`
- `docs/contracts/loop-state-machine.md`
- `docs/architecture/primitive-map.md`

## Primitive Focus

- Freeze the MVP public API as run control, package refs, context/output projection, events, journals, ports, and lineage.
- Split public API docs into MVP public surface, reserved public contracts, and optional crate APIs.
- Treat hooks as package-resolved lifecycle capabilities; do not let the first run API require every hook, stream, telemetry, isolation, subagent, channel, or extension surface.

## Must Not Own

Runtime-package fingerprint rules, event/journal taxonomy, tool packs, concrete telemetry exporters, product host adapters, or UI routing.

## Validation And Review

- `run_text` and `run_typed` lower into `RunRequest`.
- `RunHandle` completion waits for terminal run state, not only final visible text.
- Public support types required by signatures are named.
- No simple helper bypasses package, policy, journal, event, redaction, or lineage requirements.
