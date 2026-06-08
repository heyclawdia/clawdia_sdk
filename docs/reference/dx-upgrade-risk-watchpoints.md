# DX Upgrade Risk Watchpoints

This note captures future-change risks from the DX upgrade packet. The SDK is
alpha, so breaking changes are acceptable when they move the system forward, but
they must be documented before release handoff.

## Facade Crate Risks

- The unpublished `clawdia-sdk` crate is a facade only. It must not own a run
  loop, provider registry authority, package registry authority, event stream,
  journal path, policy path, tool executor, telemetry truth store, workflow
  engine, approval UI, or product adapter.
- `AgentApp` now exists as a sync-first assembly wrapper over
  `AgentRuntimeBuilder`. Future changes must keep its helpers lowering into
  `RunRequest`, `RuntimePackage`, provider registry, journal, event bus,
  content, policy, and tool ports instead of adding a second runtime path.
- If the facade uses default features, audit the dependency tree. Default
  installs should not surprise users with provider, database, OTel, protocol,
  browser, workflow, async-runtime, or UI dependencies unless a release decision
  says so explicitly.
- Current facade features are limited to real re-export groups and optional
  helper crates: `providers`, `workspace-tools`, `evals`, `reports`, `macros`,
  `file-store`, `supabase-store`, `stores`, `test-support`, and `all-stable`.
  Do not add an advertised feature for a future area until a concrete crate,
  namespace, and test matrix exist.
- If the facade is published later, release notes must state that direct split
  crates remain supported and that `agent_sdk_core::prelude` is core-only.
- `AgentAppStores` exposes separate journal write and journal read ports. Keep
  reporting and resume helpers reading from `RunJournalReader` instead of
  downcasting a store bundle or adding facade-only report state.
- Phase 16 read helpers are evidence projections only:
  `event_frames_for_run` is live buffered observation,
  `archived_event_frames` is an event archive read,
  `journal_records_for_run` is durable journal truth,
  `latest_checkpoint` is an accelerator read, and
  `run_report_from_stores` is a report projection from journal records. Do not
  let future helpers merge those surfaces into a facade-owned trace store.

## Tool Authoring Risks

- Tool macros now live in optional `agent-sdk-macros` and are re-exported by
  `clawdia-sdk` only behind the `macros` feature.
- Generated tool schema must be deterministic and covered by fixtures or golden
  tests.
- Generated tool identity, namespace, version, executor ref, schema ref, and
  policy refs must be stable enough for runtime package fingerprints.
- Macro-generated execution must still go through `ToolExecutionCoordinator`,
  policy, approval, effect intent/result records, journals, and events.
- Typed tools that call `require_approval()` now lower into routes with
  `requires_approval = true`. Do not infer approval dispatch from high risk or
  an approval policy ref alone; older toolkit routes may carry approval policy
  metadata without requiring a host approval dispatch.
- Structured tool errors must preserve typed error kinds; do not collapse them
  to strings for the provider-facing path.
- Macro expansions must continue to resolve through either direct split-crate
  dependencies or `clawdia_sdk::tools` facade-only imports.

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
- File and Supabase stores now exist as optional crates. Keep raw provider
  arguments out of journals/events/debug output and return content refs. Keep
  Supabase byte storage binary-safe via base64 and schema-profile headers.
- Provider argument stores must support by-ref JSON readback for typed tool
  execution. Adding a store backend that can only write provider arguments is
  incomplete for `AgentApp::typed_tool`.
- Supabase stores include agent-pool state through explicit PostgREST/RPC
  functions. Supabase project provisioning, RLS policy, service-role rotation,
  and live-host migrations remain host-owned release work.

## Phase 15 Alpha Breaking Changes

- `ToolRoute` and `ToolPackToolSnapshot` now carry `requires_approval`.
  Existing package snapshots that require host approval must set this flag
  explicitly; approval policy refs alone remain policy metadata.
- `PackageSidecarSnapshot` can carry a redacted package payload so provider
  tool schemas can be projected without resolving raw sidecars. Future sidecar
  payloads must stay redacted and fingerprint-stable.
- `ProviderArgumentStore` now includes `load_provider_arguments_json` so typed
  tools can resolve provider argument content refs. Backends must implement
  readback, malformed-JSON errors, and no raw argument leakage in journals or
  events.
- `AgentAppStores` now includes `journal_reader` alongside the write journal so
  facade reports can read durable evidence through a typed port.

## Phase 16 Alpha Breaking Changes

- The first-developer path now treats `clawdia-sdk` as a local checkout facade
  only. Published-alpha docs should keep using the split crates unless the
  facade publish policy changes.
- `AgentApp` now stores the optional `AgentAppStores` bundle so read-side
  helpers can return typed host-configuration diagnostics when durable evidence
  ports are missing.
- Checkpoint examples now claim resume-readiness evidence only. They do not
  claim run continuation or a facade resume API.

## Example And Documentation Risks

- Do not claim numbered examples are runnable until their directories, READMEs,
  commands, expected output, failure modes, and CI gates exist.
- The Phase 15 numbered smoke examples now exist as workspace packages. Keep
  their commands runnable without live credentials:
  `clawdia-sdk-example-01-facade-complex-agent`,
  `clawdia-sdk-example-02-typed-tool-macro`,
  `clawdia-sdk-example-03-file-store`,
  `clawdia-sdk-example-04-supabase-scripted-store`, and
  `clawdia-sdk-example-05-reporting-and-eval`.
- The Phase 16 numbered examples now exist as workspace packages. Keep their
  commands runnable without live credentials:
  `clawdia-sdk-example-06-typed-output-and-events`,
  `clawdia-sdk-example-07-approval-denial`, and
  `clawdia-sdk-example-08-checkpoint-replay`.
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
- If later work expands `AgentApp::builder`, include tests that record the same
  event and journal vocabulary as direct runtime usage.
- If later work expands macros, include schema-generation, async execution,
  sync execution, structured error, facade-only import, and doc-example tests.
- If later work adds persistence backends, map each backend to the persistence
  ownership map and avoid a state-store umbrella.
- If later work breaks split-crate imports, document the alpha breaking change
  in release notes and this risk file before publishing.
