# Agent SDK Workspace Instructions

This workspace is the authoritative documentation, planning, and implementation home for the standalone Rust-first Agent SDK.

## Scope

- Treat `<repo-root>` as the source of truth for Agent SDK architecture, contracts, examples, workstreams, and open questions.
- Treat `docs/workstreams/` as the historical contract-packet launch map and `docs/implementation-workstreams/` as the active Rust implementation launch map.
- Keep the SDK packet product-neutral. Do not add product-specific host adapters or examples to the active handoff unless the user explicitly requests a separate external task.
- Do not create or maintain parallel Agent SDK packets outside this workspace.
- Do not create branches unless the user explicitly approves.
- Do not create Rust source files, executable tests, package manifests, or fixtures while the task is documentation-only.

## Public Repository Criteria

- Treat this repository as public by default. Before any release, publish, or broad documentation handoff, run `scripts/public-release-audit.sh` or an equivalent checked audit.
- The public audit must fail on likely personal information, local absolute paths such as `/Users/<name>/...`, tracked env/credential/private-key files, common secret token formats, and missing `.gitignore` coverage for local, credential, build, log, and temporary artifacts.
- Do not publish when the public audit fails. Fix the source-of-truth docs or ignore rules first, then rerun the audit and record the evidence.

## Workstream Discipline

- Start from `README.md` and `docs/start-here.md`.
- Read `coding_standards.md` and `docs/workstreams/validation-gates.md` before editing any implementation contract.
- Use `docs/reference/sdk-review-checklist.md` when reviewing SDK changes, especially for simplicity and product-neutrality.
- Use `docs/architecture/primitive-map.md` when deciding whether a new concept is a core primitive, a feature layer, an optional adapter, or host-owned behavior.
- Use `docs/workstreams/README.md` for historical contract-packet work and `docs/implementation-workstreams/README.md` for current Rust implementation work. Numbered folders define dependency phases; all direct launch files inside the current numbered folder are parallel-safe.
- Pick exactly one launch file under the relevant numbered phase folder as your launch target.
- Only edit files listed as writable in that launch file and its owner/phase authority. For broad current-state guidance, explicitly name the stitching-owned files before editing.
- Read dependencies listed by the goal and owner role before editing.
- Non-stitching workstreams should record cross-cutting proposals in their handoff unless their writable list explicitly includes `docs/reference/cross-cutting-proposals.md`. The stitching owner reconciles accepted proposals into that file and shared indices.
- The integration/stitching owner is the only role that should reconcile public names, ID taxonomy, shared indices, event/journal alignment, runtime-package fingerprint inputs, and final whole-packet validation.
- The integration/stitching owner owns the phase launch docs under `docs/workstreams/[0-9][0-9]-*/**`, role docs under `docs/workstreams/_roles/`, and `docs/reference/feature-to-primitive-matrix.md`.
- Each phase goal must finish with the validation evidence named in its goal file and owner role doc. A prose review without tests, fixtures, or audits is not enough once code exists.

## Contract Rules

- SDK contracts live in `docs/contracts/`.
- Examples live in `docs/examples/`.
- Every normative contract must preserve explicit SDK-owned and host-owned boundaries.
- Ergonomic helpers are only thin lowering layers into canonical contracts; they must not bypass validation, policy, journal, event, telemetry, or redaction behavior.
- Higher-level features should layer on the primitive kernel instead of creating parallel registries, side-effect paths, or hidden runtime state.
- Do not let implementation accumulate in catch-all `mod.rs` files. `mod.rs` should be a small facade with module declarations and re-exports only; real behavior belongs in meaningfully named files such as `read.rs`, `grep.rs`, `edit.rs`, `write.rs`, `policy.rs`, `transport.rs`, or `codec.rs`.
- When adding toolkit operations, name files after the operation a future agent will search for, preserve stable public exports from the crate facade, and add or update tests at the matching responsibility path rather than hiding all behavior in one large module.
- File-reading helpers should be format-aware pipelines, not one text-only blob. Keep detection, text extraction, media metadata, OCR/PDF/image/RAW adapters, and fallback binary summaries in separate modules with bounded output, content refs, redaction policy, and deterministic fixtures.
- Do not make safe oversized reads fail by default. Read a bounded prefix, mark the output truncated, and include guidance to use search/grep or a narrower/range read; only fail hard for unsafe paths, denied policy, unsupported ambient host access, or parser/security errors.
- Any new reader claim needs explicit checked-in fixtures and tests for success, truncation, malformed/corrupt input, and no raw binary leakage.
- Learn from mature coding-agent tools before adding new toolkit operations: prefer one simple model-facing surface such as `read`/`grep`/`write`/`patch`, but implement it through typed internal routes, hash/stale-write guards, preview-before-apply behavior, and format-specific readers behind stable SDK contracts.
