# Phase 14: Evaluation Metrics

Add optional evaluation-framework surfaces after release readiness. Phase 14 work
must remain post-hoc or explicitly invoked, derive from existing core traces and
journals, and avoid adding evaluator behavior to normal agent runs.

| Launch Target | Run In Parallel? | Purpose |
| --- | --- | --- |
| [Trace Metrics And Comparison](14a-trace-metrics-and-comparison.md) | no | Add deterministic run/session metrics and optional toolkit comparison helpers over `agent-sdk-eval`. |

## Exit Gate

- `cargo fmt --check` passes.
- `cargo test -p agent-sdk-eval` passes.
- `cargo test -p agent-sdk-toolkit` passes.
- `cargo test --workspace` passes.
- `cargo clippy --workspace --all-targets -- -D warnings` passes.
- Public-release audit passes.
- Reviewer confirms metrics are derived views over supplied evidence and do not create a second runtime, journal, trace store, or implicit LLM call path.
