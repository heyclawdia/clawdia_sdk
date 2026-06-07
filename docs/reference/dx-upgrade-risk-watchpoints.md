# DX Upgrade Risk Watchpoints

This note captures future-change risks from the DX upgrade packet. The SDK is
alpha, so breaking changes are acceptable when they move the system forward, but
they must be documented before release handoff.

## Facade Crate Risks

- The unpublished `clawdia-sdk` crate is a facade only. It must not own a run
  loop, provider registry authority, package registry authority, event stream,
  journal path, policy path, tool executor, telemetry truth store, workflow
  engine, approval UI, or product adapter.
- If `AgentApp` or a similar builder is added, require lowering tests that
  compare its output to direct `Agent`, `AgentRuntime`, `RunRequest`, and
  `RuntimePackage` construction.
- If the facade uses default features, audit the dependency tree. Default
  installs should not surprise users with provider, database, OTel, protocol,
  browser, workflow, async-runtime, or UI dependencies unless a release decision
  says so explicitly.
- Current facade features are limited to real re-export groups: `providers`,
  `workspace-tools`, `evals`, `test-support`, and `all-stable`. Do not add an
  advertised feature for a future area until a concrete crate, namespace, and
  test matrix exist.
- If the facade is published later, release notes must state that direct split
  crates remain supported and that `agent_sdk_core::prelude` is core-only.

## Tool Authoring Risks

- If a tool macro is added, keep it outside `agent-sdk-core` and make it
  optional.
- Generated tool schema must be deterministic and covered by fixtures or golden
  tests.
- Generated tool identity, namespace, version, executor ref, schema ref, and
  policy refs must be stable enough for runtime package fingerprints.
- Macro-generated execution must still go through `ToolExecutionCoordinator`,
  policy, approval, effect intent/result records, journals, and events.
- Structured tool errors must preserve typed error kinds; do not collapse them
  to strings for the provider-facing path.

## Install Feature Risks

- Feature names must correspond to real crates, modules, or stable grouping
  decisions. Avoid advertising hosted stores, protocol adapters, exporters,
  workflow helpers, or isolation runtimes until the crates and tests exist.
- An all-inclusive feature should stay explicit and should not be used as the
  default path until dependency weight and experimental support are documented.
- Provider feature flags should prove that unused providers are not compiled or
  pulled into dependency trees.

## Persistence Risks

- Do not introduce a global state store. Journals, checkpoints, content refs,
  event archives, agent-pool state, tool-execution caches, and provider
  arguments have separate ownership.
- Session helpers must project from journals and checkpoints; they must not
  replace the run journal as durable truth.
- Durable store examples need crash/replay, missing-content, redaction, cursor,
  and interrupted-effect fixtures before they are advertised as production
  ready.

## Example And Documentation Risks

- Do not claim numbered examples are runnable until their directories, READMEs,
  commands, expected output, failure modes, and CI gates exist.
- Live-provider examples need deterministic fake or transport-injected paths for
  CI. Live credentials remain user/host-owned and must not enter runtime package
  fingerprints, journals, events, logs, or docs output.
- Every example should state SDK-owned and host-owned boundaries so onboarding
  docs do not accidentally make product UI, credentials, workflow state, or
  storage look like core responsibilities.

## Follow-Up Watchpoints

- If later work extends `clawdia-sdk`, keep `agent-sdk-core` dependency-light and
  run `cargo tree` for core, facade default features, and each real facade
  feature group.
- If later work adds `AgentApp::builder`, include a test that records the same
  event and journal vocabulary as direct runtime usage.
- If later work adds macros, include schema-generation, async execution, sync
  execution, structured error, and doc-example tests.
- If later work adds persistence backends, map each backend to the persistence
  ownership map and avoid a state-store umbrella.
- If later work breaks split-crate imports, document the alpha breaking change
  in release notes and this risk file before publishing.
