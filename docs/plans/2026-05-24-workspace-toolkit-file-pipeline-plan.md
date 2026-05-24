# Workspace Toolkit File Pipeline Plan

Date: 2026-05-24

## Objective

Make the SDK toolkit's essential workspace tools easy for agents to use while keeping them typed, policy-bound, and maintainable. The immediate slice extracts read/search/edit/write behavior into meaningful modules, adds real file-kind detection and reader modules to `workspace_read`, and documents the full format-aware reader pipeline for PDFs, images, Apple Photos RAW/ProRAW-style media, Office OpenXML documents, ZIP archives, SQLite, URLs, and future resource readers.

## Relevant Existing Context

- `AGENTS.md`: no branches without approval, product-neutral SDK boundaries, no catch-all implementation files.
- `README.md` and `docs/start-here.md`: optional toolkit helpers layer over the primitive kernel and stay out of `agent-sdk-core`.
- `coding_standards.md` and `docs/workstreams/validation-gates.md`: helpers must lower into canonical contracts, be mockable, and provide tests once code exists.
- `docs/contracts/tool-pack-contract.md`: read/search/edit/write/shell/resource tools are optional toolkit packs with policy, refs, hashes, anchors, and effect lineage.
- `docs/agent-sdk-toolkit/adapter-and-runtime-plan.md`: toolkit adapters must not create a second runtime, package registry, policy path, event stream, or journal.
- oh-my-pi read/search docs: one simple `read`/`search` surface can route through typed internal readers, selectors, limits, anchors, archives, SQLite, documents, images, URLs, and internal resources.
- OpenAI Codex apply-patch/source docs: file editing should use structured patch/application contracts and a small model-facing surface, with tool dispatch, hooks, telemetry, and image viewing as typed tool behavior.

## Behavior Contract

New behavior:

- `workspace/mod.rs` and protocol/testing `mod.rs` files stay small facades; implementation lives in files named after responsibilities.
- `workspace_read` detects text, Markdown, JSON, PDF, common image formats, HEIC/AVIF, RAW/ProRAW-style extensions such as DNG/CR2/CR3/NEF/ARW, Office documents, archives, and fallback binary files.
- PDF reads extract text through `pdf-extract` and expose document metadata and warnings.
- Image reads extract dimensions/color metadata through `image`, with HEIC/AVIF-style BMFF dimension probing when full decoding is unavailable.
- RAW/DNG reads extract TIFF/DNG dimensions where available and warn when sensor pixels or Apple Photos adjustment stacks are not decoded.
- DOCX/PPTX/XLSX reads extract bounded OpenXML text through ZIP/XML parsing.
- ZIP reads list archive entries safely and warn on unsupported archive formats.
- Binary reads return bounded metadata, MIME/type detection, source hash, pipeline steps, and a summary instead of lossy text or fake anchors.
- `workspace_search` skips detected binary files instead of grep-scanning images/PDF/RAW/documents as text.
- Workspace bounds deny symlink escapes for read and write, enforce file-size checks before loading file bytes, clamp caller read limits to policy output limits, and bound search previews.
- `workspace_edit` apply requires a preview hash tied to the preview diff; raw edit/write methods are not public SDK-consumer side-effect paths.
- Public exports preserve a simple import path from `agent_sdk_toolkit`.
- Checked-in parser fixtures cover PDF, PNG, DNG/RAW, DOCX, oversized DOCX, and ZIP reads.
- Documentation defines the full reader pipeline, validation criteria, and remaining future adapter modules for OCR/vision, richer RAW/Apple Photos handling, legacy documents, SQLite, URLs, and resource refs.

Preserved behavior:

- `agent-sdk-core` remains untouched by toolkit implementation details.
- The model-facing common path remains simple: read/search/edit/write/shell/resource helpers.
- Read/search results stay behind content refs when executed as tools.
- Edits remain hashline-anchored with preview/apply separation.
- Writes remain create/overwrite scoped with before/after hashes.

Removed behavior:

- The read path no longer treats all bytes as UTF-8 lossy text.
- Future agents are instructed not to hide broad toolkit behavior in one `mod.rs`.

## Scope

Writable files for this pass:

- `AGENTS.md`
- `README.md`
- `docs/architecture/external-sdk-lessons.md`
- `docs/contracts/tool-pack-contract.md`
- `docs/agent-sdk-toolkit/README.md`
- `docs/agent-sdk-toolkit/workspace-toolkit-plan.md`
- `docs/plans/2026-05-24-workspace-toolkit-file-pipeline-plan.md`
- `crates/agent-sdk-toolkit/README.md`
- `crates/agent-sdk-toolkit/Cargo.toml`
- `crates/agent-sdk-toolkit/src/lib.rs`
- `crates/agent-sdk-toolkit/src/workspace/**`
- `crates/agent-sdk-toolkit/tests/toolkit_packs.rs`

Out of scope:

- OCR for scanned PDFs/images, RAW sensor-pixel decoding, Apple Photos adjustment-stack interpretation, legacy Office binary formats, TAR/TGZ readers, SQLite readers, and URL readers.
- Live host filesystem integrations beyond the existing bounded workspace tests.
- Product-specific host adapters, UI, remote file systems, browser profiles, or credential stores.

## Validation Plan

- `cargo fmt --check`
- `cargo test -p agent-sdk-toolkit --test toolkit_packs`
- `cargo test -p agent-sdk-toolkit`
- `cargo test`
- `git diff --check`
- Independent implementation review against SDK simplicity, product-neutrality, mockability, module layout, binary safety, and future adapter criteria.

## Risk / Gotcha Carry-Forward

- Do not claim PDF/image/RAW extraction support until parser adapters and deterministic fixtures exist.
- Do not add parser dependencies to the common toolkit path if they should be optional features or separate adapter crates.
- Do not emit raw binary content into model-visible text, events, telemetry, or journals by default.
- Do not let format detection rely on extension alone when magic bytes are available; magic wins, extension is a fallback or a subtype hint.
- Do not make `workspace_read` an unbounded document conversion product. Parser adapters must emit bounded text, metadata, content refs, parser version, warnings, and deterministic failure reasons.
- Do not bypass hash/stale guards for edit or preview-before-apply behavior when adding patch-style tools.
