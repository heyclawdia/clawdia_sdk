# Source Migration Map

> Historical migration audit only. Some old sources mention retired product-specific files. Active implementation agents should use [../contracts/README.md](../contracts/README.md), [../workstreams/README.md](../workstreams/README.md), and [../examples/README.md](../examples/README.md).

Date: 2026-05-23

The Agent SDK packet moved from the Clawdia application checkout into `/Users/clawdia/clawdia_sdk`.

The old checkout path must remain a pointer only:

- `/Users/clawdia/clawdia/docs/architecture/agent-sdk`
- `/Volumes/Clawdia/docs/architecture/agent-sdk`

## Architecture

| Old source | New target |
| --- | --- |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/README.md` | `/Users/clawdia/clawdia_sdk/docs/start-here.md` and `/Users/clawdia/clawdia_sdk/README.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/architecture-proposal.md` | `/Users/clawdia/clawdia_sdk/docs/architecture/architecture-proposal.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/coding-standards.md` | `/Users/clawdia/clawdia_sdk/docs/architecture/coding-standards.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/primitive-map.md` | `/Users/clawdia/clawdia_sdk/docs/architecture/primitive-map.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/observability-and-lineage.md` | `/Users/clawdia/clawdia_sdk/docs/architecture/observability-and-lineage.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/external-sdk-lessons.md` | `/Users/clawdia/clawdia_sdk/docs/architecture/external-sdk-lessons.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/coverage-gap-matrix.md` | `/Users/clawdia/clawdia_sdk/docs/architecture/coverage-gap-matrix.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/current-clawdia-coverage.md` | retired from active handoff; product-neutral coverage now lives in `/Users/clawdia/clawdia_sdk/docs/examples/README.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/clawdia-flow-examples.md` | retired from active handoff; product-neutral scenarios now live in `/Users/clawdia/clawdia_sdk/docs/examples/` |

## Contracts

| Old source | New target |
| --- | --- |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/contracts/README.md` | `/Users/clawdia/clawdia_sdk/docs/contracts/README.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/contracts/api-contracts.md` | `/Users/clawdia/clawdia_sdk/docs/contracts/api-contracts.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/contracts/event-schema.md` | `/Users/clawdia/clawdia_sdk/docs/contracts/event-schema.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/contracts/run-handle-reconnect-contract.md` | `/Users/clawdia/clawdia_sdk/docs/contracts/run-handle-reconnect-contract.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/contracts/loop-state-machine.md` | `/Users/clawdia/clawdia_sdk/docs/contracts/loop-state-machine.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/contracts/runtime-package-schema.md` | `/Users/clawdia/clawdia_sdk/docs/contracts/runtime-package-schema.md` |
| new standalone context-memory pass | `/Users/clawdia/clawdia_sdk/docs/contracts/context-memory-contract.md` |
| new standalone lifecycle-hooks pass | `/Users/clawdia/clawdia_sdk/docs/contracts/hook-lifecycle-contract.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/contracts/journal-replay-schema.md` | `/Users/clawdia/clawdia_sdk/docs/contracts/journal-replay-schema.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/contracts/tool-approval-contract.md` | `/Users/clawdia/clawdia_sdk/docs/contracts/tool-approval-contract.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/contracts/structured-output-contract.md` | `/Users/clawdia/clawdia_sdk/docs/contracts/structured-output-contract.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/contracts/stream-rule-contract.md` | `/Users/clawdia/clawdia_sdk/docs/contracts/stream-rule-contract.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/contracts/tool-pack-contract.md` | `/Users/clawdia/clawdia_sdk/docs/contracts/tool-pack-contract.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/contracts/isolation-runtime-contract.md` | `/Users/clawdia/clawdia_sdk/docs/contracts/isolation-runtime-contract.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/contracts/subagent-contract.md` | `/Users/clawdia/clawdia_sdk/docs/contracts/subagent-contract.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/contracts/extension-sdk-contract.md` | `/Users/clawdia/clawdia_sdk/docs/contracts/extension-sdk-contract.md` |
| new standalone output-delivery pass | `/Users/clawdia/clawdia_sdk/docs/contracts/output-delivery-contract.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/contracts/otel-mapping-contract.md` | `/Users/clawdia/clawdia_sdk/docs/contracts/otel-mapping-contract.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/contracts/telemetry-privacy-contract.md` | `/Users/clawdia/clawdia_sdk/docs/contracts/telemetry-privacy-contract.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/contracts/review-matrix.md` | `/Users/clawdia/clawdia_sdk/docs/contracts/review-matrix.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/contracts/clawdia-host-integration-map.md` | retired from active handoff; generic scenario mapping now lives in `/Users/clawdia/clawdia_sdk/docs/examples/` and `/Users/clawdia/clawdia_sdk/docs/workstreams/06-scenario-coverage/06a-generic-scenario-coverage.md` |

## Examples, Notes, Plans, And Risks

| Old source | New target |
| --- | --- |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/examples/*.md` | `/Users/clawdia/clawdia_sdk/docs/examples/*.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/notes/*.md` | `/Users/clawdia/clawdia_sdk/docs/reference/notes/*.md` |
| `/Volumes/Clawdia/docs/architecture/agent-sdk/notes/agent-start-flow.excalidraw` | `/Users/clawdia/clawdia_sdk/docs/reference/notes/agent-start-flow.excalidraw` |
| `/Volumes/Clawdia/docs/plans/*agent-sdk*.md` | `/Users/clawdia/clawdia_sdk/docs/reference/plans/*.md` |
| `/Volumes/Clawdia/docs/potential_problems/agent-sdk-phase1-2026-05-21.md` | `/Users/clawdia/clawdia_sdk/docs/reference/risks/agent-sdk-phase1-2026-05-21.md` |

## Standalone Additions After Migration

| Source | Target |
| --- | --- |
| new phase-first workstream launch docs | `/Users/clawdia/clawdia_sdk/docs/workstreams/[0-9][0-9]-*/**` |
| new owner role docs | `/Users/clawdia/clawdia_sdk/docs/workstreams/_roles/*.md` |
| new primitive hardening artifact | `/Users/clawdia/clawdia_sdk/docs/reference/feature-to-primitive-matrix.md` |
| new primitive/source/phase hardening plans | `/Users/clawdia/clawdia_sdk/docs/plans/*.md` |

## Intentional Exclusions

Product-specific host-adapter packets are intentionally excluded from the active SDK handoff after the product-neutral cleanup. Historical migration notes may mention old paths, but implementation agents should use the generic scenario examples and contracts instead.
