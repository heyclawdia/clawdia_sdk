# Goal 05b: Isolation Execution

## Phase

[Phase 05: Feature Layers](README.md)

## Owner Role

[Isolation Execution](../_roles/06-isolation-execution.md)

## Parallelism

Parallel-safe with every other goal in Phase 05 after Phase 04 exits. Do not start Phase 06 until all Phase 05 goals finish.

## Required Reading

- `README.md`
- `docs/start-here.md`
- `coding_standards.md`
- `docs/workstreams/validation-gates.md`
- `docs/reference/sdk-review-checklist.md`
- `docs/architecture/primitive-map.md`
- phase README
- owner role doc
- owner role doc read-only inputs

## Writable Files

- `docs/contracts/isolation-runtime-contract.md`

## Primitive Focus

- Isolation is `ExecutionEnvironment` plus `IsolationRuntime` adapter port, package requirements, policy checks, events, and journal records.
- Concrete containers, VMs, and remote sandboxes are optional adapters.

## Must Not Own

Approval UI, tool semantics, provider routing, or concrete runtime implementation in core.

## Validation And Review

- Unsupported adapter and downgrade fail closed.
- Mount/network/process lifecycle is journaled.
- Cleanup and detached process behavior matches child lifecycle policy.
