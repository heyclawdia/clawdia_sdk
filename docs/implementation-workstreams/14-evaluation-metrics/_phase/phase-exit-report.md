# Phase 14 Exit Report: Evaluation Metrics

## Phase Objective

Phase 14 added optional post-hoc evaluation metrics and comparison helpers over
released trace and journal primitives. The phase keeps evaluation out of normal
agent execution: metrics are deterministic projections over supplied evidence,
and AI evaluator calls remain explicit and budgeted.

## Dependency Status

- Phases 00 through 13 already have phase exit reports in the implementation
  workstream tree.
- Phase 14 has one launch target:
  [14a Trace Metrics And Comparison](../14a-trace-metrics-and-comparison.md).
- Phase 15 must not start until this report has reviewer PASS.

## Goal Status

`14a-trace-metrics-and-comparison`: complete.

Delivered surfaces:

- `agent-sdk-eval::TraceMetrics` over `TurnTrace`, `RunTrace`, and
  `SessionTimeline`.
- Provider attempt counts and token totals from model-attempt journal records.
- Tool call counts, terminal status counts, and elapsed-time summaries from
  durable journal evidence.
- `TraceMetricsComparison` deterministic deltas for run/session comparisons.
- Toolkit ergonomics through `AgentTraceEvaluation::compare_sessions`.
- Evaluator validation that measured confidence must be backed by comparison
  evidence and deterministic metric deltas.

## Validation Commands

Passed:

```bash
cargo fmt --check
cargo test -p agent-sdk-eval
cargo test -p agent-sdk-toolkit
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
scripts/public-release-audit.sh
```

Notable test evidence:

- `cargo test -p agent-sdk-eval`: 9 eval contract tests passed, including
  provider/tool metrics, session comparison deltas, non-monotonic tool timing,
  cited support validation, and measured-confidence rejection cases.
- `cargo test -p agent-sdk-toolkit`: 44 toolkit tests and 1 doctest passed,
  including `compare_sessions_exposes_metrics_and_defers_provider_calls` and
  measured AI confidence from deterministic deltas.
- `cargo test --workspace`: workspace tests and doctests passed across
  `agent-sdk-core`, `agent-sdk-eval`, `agent-sdk-provider`,
  `agent-sdk-toolkit`, and `clawdia-sdk`.
- `scripts/public-release-audit.sh`: `public-release-audit: PASS`.

## Source-Layout Audit

Mandatory audit commands from
`docs/workstreams/validation-gates.md` were run.

Summary:

- `find crates/agent-sdk-core/src -maxdepth 1 -type f -not -name lib.rs -not -name README.md`
  returned no files. Core root source remains limited to the crate facade and
  README.
- `find crates/agent-sdk-core/tests -maxdepth 1 -type f -name '*.rs' ...`
  returned only two-line root Cargo test shims. Full integration bodies remain
  under responsibility folders.
- `find crates -path '*/src/*.rs' -maxdepth 3 -type f` shows optional crate
  source files under crate-local responsibility modules; Phase 14 did not add
  catch-all implementation files.
- `rg -n '#\[path = .*\]\s*pub mod|pub mod ...' crates/agent-sdk-core/src/lib.rs`
  shows the existing documented core facade aliases; Phase 14 did not add new
  core public module aliases.
- `rg -n '\b(Fake|Scripted)...|ConformanceHarness' crates/agent-sdk-core/src`
  shows fake/scripted helpers under `src/testing`, as required.
- `rg -n '\btrait\b|\bAdapter\b|\bResolver\b|\bFake\b|\bScripted\b|ConformanceHarness' crates/agent-sdk-core/src/records`
  returned no matches. Records remain durable DTOs, not adapter or fake
  owners.
- `wc -l crates/agent-sdk-*/src/lib.rs` returned narrow optional crate facades:
  `agent-sdk-eval/src/lib.rs` 29 lines, `agent-sdk-provider/src/lib.rs` 32
  lines, `agent-sdk-toolkit/src/lib.rs` 77 lines. `agent-sdk-core/src/lib.rs`
  remains a larger documented root facade over responsibility modules.

## Public API Review

Rust API Guidelines posture:

- New eval metric types are nouns with explicit constructors/projection methods.
- Reports and comparisons are deterministic data projections; no normal run path
  performs evaluator work.
- Public methods name their evidence source, such as `from_run_trace`,
  `from_session_timeline`, and toolkit comparison helpers.
- Metric fields use concrete integer/optional elapsed-time types instead of
  string parsing or ambient state.
- The eval crate remains optional and does not add dependencies to
  `agent-sdk-core`.
- AI evaluator surfaces require explicit request/usage budgets and validate that
  measured confidence has comparison evidence.

## Primitive And Boundary Review

Primitive decision:

- Reused kernel primitives: `JournalRecord`, `RunTrace`, `TurnTrace`,
  `SessionTimeline`, tool call records, model-attempt records, and evidence
  refs.
- New feature-layer primitives: deterministic `TraceMetrics`,
  `ToolTraceMetric`, and `TraceMetricsComparison` in the optional eval layer.
- New capability variants: none.
- Host-owned behavior kept out: evaluator model choice, rubric design, pricing,
  dashboards, storage, and causal interpretation remain host-owned.

No-mini-SDK gate:

- Phase 14 did not add a runtime, event stream, journal path, trace store,
  telemetry truth store, provider adapter, or dashboard.

Mockability gate:

- Tests use supplied traces, journals, evidence bundles, and scripted evaluator
  paths. No live provider, product UI, or external service is required.

## Accepted, Rejected, Deferred

Accepted:

- Deterministic trace/session metrics belong in `agent-sdk-eval`.
- Toolkit comparison helpers may wrap eval metrics when they remain post-hoc and
  explicit.
- AI evaluator judgment can consume deterministic deltas but cannot invent
  measured evidence.

Rejected:

- Adding metrics or evaluator behavior to `agent-sdk-core`.
- Running evaluation implicitly during normal agent execution.
- Treating model judgments or metric deltas as causal proof.

Deferred:

- Live pricing/rate lookup, dashboards, and external observability exporters.
  These remain host-owned or future optional adapters.

## Reviewer Status

PASS. Dedicated read-only exit review confirmed the report records the Phase 14
gate evidence, metrics remain derived views over supplied traces/journals, and
there is no second runtime, journal path, trace store, dashboard, or implicit
LLM call path.

## Next-Phase Readiness

Phase 15 may start. The Phase 14 exit gate is satisfied.
