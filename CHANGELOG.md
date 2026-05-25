# Changelog

## 0.1.0-alpha.1

Status: first public alpha crates.io release.

### Added

- `agent-sdk-core` Rust crate with the product-neutral primitive kernel: typed IDs/refs, runtime packages, content/context records, events, journals, policy, run control, P0 text runs, P1 typed output, P2 side-effect coordination, replay/recovery, scenario tests, and public API docs.
- `agent-sdk-core::testing` namespace with deterministic fake providers, content resolvers, journal stores, event sinks, scripted approval/tool/output/hook/realtime/isolation/extension/telemetry helpers, and conformance-oriented fixtures.
- `agent-sdk-toolkit` optional helper crate for filesystem workspace tools, resource reads, discovery, and shell helper contracts layered over core policy, content refs, capabilities, and effect lineage.
- Golden fixtures for event, journal, package, replay, OTel, extension, output delivery, scenario, privacy, and typed-output contract surfaces.
- Phase-gated implementation reports under `docs/implementation-workstreams`.
- GitHub Actions publish workflow for release-triggered crates.io publication in dependency order.
- Public-repo release audit for personal/sensitive content and `.gitignore` guardrails.

### Package Boundaries

- `agent-sdk-core` has an empty default feature set and does not depend on the optional toolkit crate.
- `agent-sdk-toolkit` depends on `agent-sdk-core`; core never imports toolkit helpers.
- Release metadata is configured for crates.io publication from the public GitHub repository.

### Unsupported

- No live provider adapters are included.
- No concrete container, VM, Firecracker, Docker, Apple Containerization, or remote sandbox adapter is included.
- No product UI, desktop/window, remote-channel, marketplace, or host-specific approval adapter is included.
- No network telemetry exporter, trace-store service, workflow engine, or product-owned memory backend is included.
- Live-provider, concrete-container, product-UI, and host-adapter support must not be claimed without matching contracts, tests, fixtures, and release notes.
