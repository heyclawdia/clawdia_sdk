# Phase 10: Feature Ports

Implement reserved feature-layer ports and optional crates after P2 side effects pass. All launch targets in this folder may run in parallel after Phase 09 exits.

## Launch Targets

| Title | Run in parallel? | Purpose |
| --- | --- | --- |
| [Stream Realtime](10a-stream-realtime.md) | yes | Stream rules, realtime session records, backpressure, interruption, and restart ports. |
| [Isolation Port](10b-isolation-port.md) | yes | Execution environment sidecars, isolation runtime trait, capability matching, and fail-closed downgrade. |
| [Subagents](10c-subagents.md) | yes | Parent-owned child-run helper over AgentPool, package stripping, event wrapping, and usage rollup. |
| [Extension SDK](10d-extension-sdk.md) | yes | Core extension capabilities, action protocol, package smoke, and host-manifest boundary. |
| [Tool Packs](10e-tool-packs.md) | yes | Optional toolkit packs for read/search/edit/write/shell/resource/discovery over P2 tools. |

## Exit Gate

- [x] Every feature layer is optional or behind a typed port/sidecar.
- [x] No feature becomes required for P0, P1, or P2 core profiles.
- [x] Feature-layer events, journals, package fingerprints, and policy refs are fixture-gated.
- [x] Phase exit report records reviewer PASS.
