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

## Writable Files

- `docs/examples/*`

## Primitive Focus

- Prove desktop, CLI/headless, realtime, remote channel, external runtime, memory, subagent, isolation, telemetry, and output-delivery scenarios compose from the same primitives.
- Mark product UI, routing, credentials, stores, marketplaces, dashboards, and workflow policies as host-owned.

## Must Not Own

Core contract changes, shared primitive names, runtime-package fingerprint fields, or product-specific examples in active SDK contracts.

## Validation And Review

- Scenario mapping table: scenario -> SDK primitives -> host-owned boundaries -> events/journals -> validation.
- Proposal blocks for any missing primitive; no direct edits to shared architecture/reference docs.
- Product-neutrality audit over examples.
