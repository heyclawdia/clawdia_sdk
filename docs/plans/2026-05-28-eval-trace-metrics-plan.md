# Eval Trace Metrics And Session Comparison Plan

## Objective

Add deterministic trace metrics and a simple session-comparison API so users can inspect agent runs without writing their own counting logic. The API should expose tool call counts, provider call counts, token usage, and tool elapsed timing derived from durable records, then pass those metrics into optional evaluator calls when users explicitly run AI-based checks.

## Relevant Existing Context

- `AGENTS.md`: keep the SDK product-neutral, do not branch without approval, start from README/docs/standards, and preserve explicit SDK-owned vs host-owned boundaries.
- `coding_standards.md`: public API changes need clear ownership, mockability, deterministic fakes, small facades, and the clippy workspace gate.
- `docs/workstreams/validation-gates.md`: implementation work must prove primitive fit, no mini-SDK, lineage, mockability, package architecture, and explicit validation evidence.
- `docs/architecture/primitive-map.md`: eval-layer reports are derived from journals, traces, context projection audits, events, outputs, and `agent-sdk-eval`; core must not grow dashboard or product scoring ownership.
- `docs/implementation-workstreams/14-evaluation-metrics/14a-trace-metrics-and-comparison.md`: this is the launch target for this post-release evaluation-metrics slice. It explicitly owns deterministic metrics in `agent-sdk-eval`, optional toolkit ergonomics, and user-facing docs for local metrics versus optional AI evaluator interpretation.
- Current code: `JournalRecord.timestamp_millis` already provides durable timing evidence, `ToolCallRecord` carries tool status/name/effect data, and `ModelAttemptRecord` carries provider usage. `agent-sdk-eval` owns evaluation reports; toolkit owns the provider-backed evaluator.

## Launch Target And Owner Authority

Primary launch target: `docs/implementation-workstreams/14-evaluation-metrics/14a-trace-metrics-and-comparison.md`.

This task is a post-Phase-13 user-requested extension. Existing launch targets cover toolkit tool packs, telemetry mapping, scenario API review, and release-readiness packaging, but none owns `crates/agent-sdk-eval/**`. The new Phase 14 launch target is intentionally narrow so this implementation does not overload unrelated historical targets.

## Behavior Contract

New behavior:

- `agent-sdk-eval` exposes deterministic `TraceMetrics` and `TraceMetricsComparison` helpers for `TurnTrace`, `RunTrace`, and `SessionTimeline`.
- Metrics include record/run/turn counts, provider call count, provider token totals, tool call count, tool terminal status counts, per-tool timing rows, and total tool elapsed milliseconds when timing can be derived.
- Tool elapsed timing is computed from explicit start/end evidence: tool intent/result journal timestamps, with no AI involved.
- Toolkit exposes an easy `AgentTraceEvaluation::compare_sessions(observed, baseline, expected)` helper and getters for observed metrics and comparison metrics.
- `agent-sdk-eval` adds an eval-layer `ComparisonDesign::PairedScopes { observed_scope, comparison_scope }` so session comparison does not require a new core `EntityKind::Session` or fake session entity refs.
- Toolkit passes deterministic metric deltas into the optional AI evaluator prompt/report path.
- Deterministic metric deltas are request-owned authority. `AiTraceEvaluator` may comment on, cite, or select support for those deltas, but it must not invent metric deltas from provider JSON. `measured` confidence validates against request-owned deterministic deltas.
- Docs make clear that deterministic checks and counters happen locally; LLM calls are only for explicit evaluator interpretation.

Preserved behavior:

- Normal agent runs do not perform eval work or extra provider calls.
- `agent-sdk-core` remains the durable source of journal timestamps and trace records; it does not own eval reports, comparison logic, or model-judge prompts.
- Existing `AgentTraceEvaluation::turn/run/session` APIs continue to work.

Removed behavior:

- None.

Tests proving behavior:

- Eval crate tests for provider/tool count extraction from mock trace records.
- Eval crate tests for tool elapsed timing from journal start/end timestamps.
- Eval crate tests for session comparison deltas.
- Toolkit tests proving session comparison attaches deterministic metric deltas and does not call the provider until `.evaluate(&AiTraceEvaluator)` is invoked.
- Toolkit tests proving AI evaluator can keep `measured` confidence only when deterministic comparison metric deltas exist.

## Scope

Writable implementation surfaces:

- `docs/implementation-workstreams/README.md`
- `docs/implementation-workstreams/14-evaluation-metrics/README.md`
- `docs/implementation-workstreams/14-evaluation-metrics/14a-trace-metrics-and-comparison.md`
- `crates/agent-sdk-eval/src/lib.rs`
- `crates/agent-sdk-eval/src/metrics.rs`
- `crates/agent-sdk-eval/src/request.rs`
- `crates/agent-sdk-eval/src/comparison.rs`
- `crates/agent-sdk-eval/src/report.rs`
- `crates/agent-sdk-eval/tests/eval_contract.rs`
- `crates/agent-sdk-toolkit/src/evaluation.rs`
- `crates/agent-sdk-toolkit/src/lib.rs` if new exports are needed
- `crates/agent-sdk-toolkit/tests/evaluation.rs`
- `README.md`
- `crates/agent-sdk-eval/README.md`
- `crates/agent-sdk-toolkit/README.md`
- `docs/architecture/observability-and-lineage.md`

Core source is read-only for this slice unless a compile break proves a tiny public re-export is required. The planned implementation should not add timing fields, eval DTOs, evaluator traits, metrics logic, or prompts to `agent-sdk-core` because journal timestamps and records already carry the needed durable evidence.

Shared stitching-owned references such as `docs/reference/feature-to-primitive-matrix.md`, `docs/reference/open-questions-and-ambiguities.md`, and `docs/architecture/primitive-map.md` are out of implementation scope for this slice. If implementation reveals a cross-cutting change for those files, record it in this plan or the final handoff as a stitching proposal rather than editing them directly.

## Tool Timing Pairing Contract

- Timing source of truth is `JournalRecord.timestamp_millis` on caller-supplied journal records.
- Start evidence is the first `JournalRecordPayload::Tool` for a `tool_call_id` whose `ToolCallRecord.status` is `IntentRecorded`, whose `effect_intent` exists, and whose `effect_intent.kind` is `EffectKind::ToolExecution`.
- End evidence is the terminal `JournalRecordPayload::Tool` for the same `tool_call_id` and same `effect_id` where `effect_result` exists and the status is terminal.
- `elapsed_ms = end_timestamp_millis - start_timestamp_millis` only when both timestamps are non-zero and end is greater than start.
- If timestamps are zero, missing, non-monotonic, or only embedded on the same record without a distinct start/end envelope, elapsed timing is unavailable rather than fabricated.
- Metrics may still count tool calls and terminal statuses when elapsed timing is unavailable.

## Workstreams

1. Add deterministic metrics primitives in `agent-sdk-eval`.
2. Extend `EvaluationRequest` to carry deterministic metric deltas as evaluator inputs.
3. Add toolkit session comparison ergonomics and include metric inputs in the AI evaluator prompt/report path.
4. Update docs to show deterministic checks first and optional LLM evaluator calls second.
5. Add focused tests and run the workspace validation gate.

## Risk / Gotcha Carry-Forward

- Do not infer causality from metrics alone. Tool count and elapsed time are diagnostics and eval inputs, not proof a step helped.
- Do not make the AI evaluator count tools or compute elapsed time. Deterministic code must compute those from records first.
- Do not require raw content to compute metrics.
- Do not create a second trace store. Metrics are derived views over caller-supplied records/traces.
- Do not make session comparison product-specific; host rubrics and dashboards stay outside the SDK.
- If timestamps are absent, zero, or non-monotonic, metrics should surface missing elapsed timing rather than fabricate precision.

## Validation

- `cargo fmt --check`
- `cargo test -p agent-sdk-eval`
- `cargo test -p agent-sdk-toolkit`
- `cargo test -p agent-sdk-core`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `git diff --check`
- `scripts/public-release-audit.sh`
