# Agent SDK Core Fixtures

Golden fixtures are grouped by contract surface. Later phases add concrete JSON
fixtures under these directories as event, journal, package, context, provider,
output, side-effect, feature-port, replay, and scenario contracts become active.

## Manifest Contract

Fixture manifests use `schema_version: 1` until a later phase explicitly bumps
the golden format. Paths should be grouped by contract surface:

- `events/<family>-<kind>.json`
- `journals/<record-kind>.json`
- `packages/<case-name>.json`
- `context/<case-name>.json`
- `providers/<case-name>.json`
- `replay/<case-name>.json`

Fixtures should be deterministic: no wall-clock timestamps, randomness, host
paths, network state, or external-provider output. If an implementation must
record time or IDs, generate them through the Phase 01 fake clock and
deterministic ID generator before writing the fixture.

Raw content is not part of default golden output. Store content through content
refs and write redacted summaries or byte-length metadata unless a future
contract explicitly opts into bounded raw-content capture for that fixture.
