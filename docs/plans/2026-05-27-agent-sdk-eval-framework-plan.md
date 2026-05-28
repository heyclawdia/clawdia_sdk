# Agent SDK Eval Framework Implementation Plan

## Objective

Add an optional, product-neutral evaluation framework as a separate crate and expose a focused agent-run evaluation API from the toolkit. The framework must let SDK consumers evaluate recorded agent runs, turns, sessions, expected outcomes, cited support, and comparison designs without adding model-as-judge work to the normal agent runtime path.

## Grounding

- `agent-sdk-core` remains the primitive kernel for journals, events, entity refs, traces, privacy, policy, runtime packages, and provider contracts.
- `agent-sdk-eval` owns evaluation DTOs, evidence bundles, comparison designs, evaluator reports, evaluator traits, ref validation, and deterministic test fakes.
- `agent-sdk-toolkit` owns ergonomic helpers for building eval requests from core traces and one provider-backed AI evaluator implementation.
- Hosts own scheduling, storage, dashboarding, raw-content policy, evaluator model choice, human review queues, and business-specific metrics.

Independent design review agreed with this split and called out the same dependency direction:

```text
agent-sdk-core
  <- agent-sdk-eval
      <- agent-sdk-toolkit
```

## Behavioral Contract

1. Normal agent runs must not do implicit evaluation, attribution, or extra provider calls.
2. Eval requests are post-hoc or explicitly invoked, and operate on durable evidence supplied by callers: `TurnTrace`, `RunTrace`, `SessionTimeline`, or journal records.
3. AI-judged evaluations must distinguish cited or judged support from measured impact.
4. `measured` confidence requires a baseline, paired run, ablation, counterfactual, repeated experiment, or explicit metric comparison; a model citation alone is not causal proof.
5. Provider-backed evaluator prompts should be bounded, content-ref-oriented, and validate cited support refs against the evidence bundle before returning a report.
6. The toolkit API should be easy to use in integration tests: "given these records/traces and this expected outcome, evaluate what happened and validate cited support."

## Writable Surfaces

- `Cargo.toml`
- `README.md`
- `crates/agent-sdk-core/src/application/run.rs` for removal/regression only
- `crates/agent-sdk-core/src/application/loop_driver.rs` for removal/regression only
- `crates/agent-sdk-core/src/lib.rs` for removal/regression only
- `crates/agent-sdk-core/src/records/journal.rs` for removal/regression only
- `crates/agent-sdk-core/tests/**` for removal/regression only
- `crates/agent-sdk-eval/**`
- `crates/agent-sdk-toolkit/Cargo.toml`
- `crates/agent-sdk-toolkit/src/lib.rs`
- `crates/agent-sdk-toolkit/src/evaluation.rs`
- `crates/agent-sdk-toolkit/tests/**`
- `docs/architecture/observability-and-lineage.md`
- `docs/architecture/primitive-map.md`
- `docs/reference/feature-to-primitive-matrix.md`
- `docs/reference/open-questions-and-ambiguities.md`

## Implementation Steps

1. Remove the earlier uncommitted core runtime attribution hook so `RunRequest`, `RunResult`, `JournalRecordPayload`, and P0 loop execution stay evaluation-neutral.
2. Add `agent-sdk-eval` with:
   - `EvaluationId`, `EvaluationScope`, `EvaluationSubject`, `EvaluationSubjectRole`
   - `ExpectedOutcome`, `EvaluationCriterion`
   - `EvidenceBundle`, `EvidenceItem`, `EvidenceRole`
   - `ComparisonDesign`
   - `EvaluationBudget` and `EvaluationUsage`
   - `EvaluationConfidence`, `EvaluationVerdict`, `EvaluationMetricDelta`
   - `EvaluatorJudgment`, `EvaluationReport`, `EvaluationRequest`
   - `Evaluator` trait and `testing::ScriptedEvaluator`
   - evidence construction from `TurnTrace`, `RunTrace`, and `SessionTimeline`
   - accepted/rejected support-ref validation
3. Add `agent-sdk-toolkit::evaluation` with:
   - `AgentTraceEvaluation` builder over a trace-derived `EvaluationRequest`
   - `AiTraceEvaluator` using `ProviderAdapter`
   - bounded JSON prompt generation, provider JSON parsing, support-ref validation, provider usage capture, and report construction
4. Add tests:
   - eval crate support-ref validation and scripted evaluator behavior
   - toolkit expected-vs-actual integration-style evaluation from mock turn traces
   - provider-backed evaluator accepts valid cited refs, rejects unseen refs, and avoids raw final-output leakage in prompts
   - observed-only or AI-cited eval cannot produce `measured`
   - paired, baseline, or comparison eval can produce `measured` only when metric delta and comparison evidence are present
   - invalid measured reports without comparison evidence are rejected or downgraded
   - post-hoc AI eval makes exactly the expected provider call and captures provider usage, while normal runs remain untouched
   - core P0 path remains one provider call with no implicit eval
5. Update documentation to describe outcome attribution as an eval-layer feature built from core traces and optional toolkit evaluators.

## Risks

- Do not make `agent-sdk-eval` a second runtime, journal, or event stream.
- Do not depend on private runtime journal access; start from caller-supplied records or core trace views.
- Do not add eval DTOs, evaluator traits, provider prompts, or eval-specific journal payloads to `agent-sdk-core`.
- Do not put product-specific rubrics, dashboards, or host storage policy in the SDK crates.
- Keep report confidence honest: cited support is useful attribution evidence, not proof that context caused the outcome.
- Keep JSON parsing failures bounded and surfaced as SDK contract errors.

## Verification

- `cargo fmt --check`
- `cargo test -p agent-sdk-core`
- `cargo test -p agent-sdk-eval`
- `cargo test -p agent-sdk-toolkit`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `git diff --check`
- `scripts/public-release-audit.sh`
