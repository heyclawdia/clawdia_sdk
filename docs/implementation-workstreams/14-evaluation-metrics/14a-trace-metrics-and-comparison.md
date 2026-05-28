# Trace Metrics And Comparison

## Phase

[Phase 14: Evaluation Metrics](README.md)

## Parallelism

Only launch target in this phase.

## Contract Inputs

- [observability-and-lineage.md](../../architecture/observability-and-lineage.md)
- [primitive-map.md](../../architecture/primitive-map.md)
- [validation-gates.md](../../workstreams/validation-gates.md)

## Implementation Objective

Add deterministic trace/session metrics and a simple comparison API to the
optional evaluation framework and toolkit. Counts, token totals, and elapsed
tool timing must be computed locally from supplied records. AI evaluator calls
remain explicit, optional interpretation over those deterministic inputs.

## Owned Implementation Surface

- `crates/agent-sdk-eval/src/lib.rs`
- `crates/agent-sdk-eval/src/metrics.rs`
- `crates/agent-sdk-eval/src/request.rs`
- `crates/agent-sdk-eval/src/comparison.rs`
- `crates/agent-sdk-eval/src/report.rs`
- `crates/agent-sdk-eval/tests/eval_contract.rs`
- `crates/agent-sdk-toolkit/src/evaluation.rs`
- `crates/agent-sdk-toolkit/src/lib.rs`
- `crates/agent-sdk-toolkit/tests/evaluation.rs`
- `README.md`
- `crates/agent-sdk-eval/README.md`
- `crates/agent-sdk-toolkit/README.md`
- `docs/architecture/observability-and-lineage.md`

## Must Deliver

- Deterministic metrics over `TurnTrace`, `RunTrace`, and `SessionTimeline`.
- Provider call count and token totals from provider attempt records.
- Tool call counts, terminal status counts, per-tool start/end timestamps, and elapsed time when durable journal evidence supports it.
- Session comparison deltas exposed without requiring users to write their own counting logic.
- Toolkit ergonomics for `AgentTraceEvaluation::compare_sessions`.
- Optional AI evaluator path that receives deterministic metric deltas but cannot invent them.

## Validation

- `cargo fmt --check`
- `cargo test -p agent-sdk-eval`
- `cargo test -p agent-sdk-toolkit`
- `cargo test -p agent-sdk-core`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `git diff --check`
- `scripts/public-release-audit.sh`

## Must Not

- Add metrics, evaluator traits, model-judge prompts, or comparison behavior to `agent-sdk-core`.
- Make normal agent runs perform evaluation work or extra provider calls.
- Create a second trace store, event stream, journal path, dashboard, or host scoring system.
- Treat AI support citations or local metric deltas as causal proof that a context item or tool caused the outcome.
