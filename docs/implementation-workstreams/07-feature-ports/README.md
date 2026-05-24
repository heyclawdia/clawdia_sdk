# Phase 07: Feature Ports

Implement reserved feature-layer ports and optional crates after P2 side effects pass. All launch targets in this folder may run in parallel after Phase 06 exits.

## Launch Targets

| Title | Run in parallel? | Purpose |
| --- | --- | --- |
| [Stream Realtime](07a-stream-realtime.md) | yes | Stream rules, realtime session records, backpressure, interruption, and restart ports. |
| [Isolation Port](07b-isolation-port.md) | yes | Execution environment sidecars, isolation runtime trait, capability matching, and fail-closed downgrade. |
| [Subagents](07c-subagents.md) | yes | Parent-owned child runs, package stripping, event wrapping, mailbox, and usage rollup. |
| [Extension SDK](07d-extension-sdk.md) | yes | Core extension capabilities, action protocol, package smoke, and host-manifest boundary. |
| [Tool Packs](07e-tool-packs.md) | yes | Optional toolkit packs for read/search/edit/write/shell/resource/discovery over P2 tools. |

## Exit Gate

- [ ] Every feature layer is optional or behind a typed port/sidecar.
- [ ] No feature becomes required for P0, P1, or P2 core profiles.
- [ ] Feature-layer events, journals, package fingerprints, and policy refs are fixture-gated.
- [ ] Phase exit report records reviewer PASS.
