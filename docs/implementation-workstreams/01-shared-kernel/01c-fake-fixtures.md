# Fake Fixtures

## Phase

[Phase 01: Shared Kernel](README.md)

## Parallelism

Parallel-safe with [Typed IDs](01a-typed-ids.md) and [Errors Policy](01b-errors-policy.md). Coordinate fixture file names only through the phase exit report.

## Contract Inputs

- [validation-gates.md](../../workstreams/validation-gates.md)
- [review-matrix.md](../../contracts/review-matrix.md)
- [event-schema.md](../../contracts/event-schema.md)
- [journal-replay-schema.md](../../contracts/journal-replay-schema.md)

## Implementation Objective

Build deterministic test utilities so later phases can prove behavior without live providers, real containers, product UI, or network telemetry.

## Owned Implementation Surface

- `crates/agent-sdk-core/src/fakes/`
- `crates/agent-sdk-core/tests/support/`
- `crates/agent-sdk-core/tests/fixtures/README.md`
- fixture writer/verifier utilities under `crates/agent-sdk-core/tests/support/`

## Must Deliver

- Deterministic ID generator, clock, fake content store, fake journal store, fake event sink, fake provider shell, and fixture normalization helpers.
- Golden fixture writer/readback helpers with stable ordering.
- Test support that later phases can reuse without depending on live services.
- Fixture manifest documenting schema version, path convention, and redaction expectations.

## Validation

- `cargo test -p agent-sdk-core --test fake_fixture_harness`
- fixture writer round-trip tests
- audit that test utilities do not require network, live providers, real containers, or product host state

## Must Not

- Hide implementation behavior inside fakes that production paths do not exercise.
- Use randomness or wall-clock time in golden output unless explicitly normalized.
