# agent-sdk-eval

`agent-sdk-eval` contains optional evaluation framework primitives for Agent SDK consumers.

The crate is layered over `agent-sdk-core` traces, entity refs, privacy classes, and provider usage records. It does not run agents, append journals, publish events, choose evaluator models, store dashboards, or define product-specific scoring rubrics.

## Deterministic Trace Metrics

`TraceMetrics` derives local metrics from caller-supplied `TurnTrace`, `RunTrace`, and `SessionTimeline` records:

- trace `started_at_millis`, `ended_at_millis`, and `elapsed_ms`;
- provider call counts and provider-reported token totals;
- tool call counts, terminal status counts, and per-tool start/end/elapsed timing;
- session comparison deltas through `TraceMetricsComparison`.

These metrics are computed without provider calls. `EvaluationRequest::metric_deltas` carries deterministic deltas into evaluators so an optional AI evaluator can explain or judge them, but the model is not the authority for counts, timing, or token totals.

`ComparisonDesign::PairedScopes` supports comparing two sessions or other durable scopes without inventing a core session entity ref. Measured confidence still requires comparison evidence plus at least one deterministic metric delta.
