# Workspace Toolkit Plan

## Objective

The workspace toolkit should give agents a small, familiar surface:

- `workspace_read`
- `workspace_search`
- `workspace_edit`
- `workspace_write`
- later `workspace_patch` for structured multi-file patches

Internally, those tools are typed pipelines with policy, refs, hashes, bounds, redaction, journal/effect records, and deterministic tests. The common path should be easy enough for a model to learn, while the implementation remains hard to misuse.

## Source Lessons

- [oh-my-pi read](https://github.com/can1357/oh-my-pi/blob/main/docs/tools/read.md) keeps one `read` input surface but routes through URL, internal-resource, archive, SQLite, directory, notebook, document, image, and text readers with selectors, limits, artifacts, and hashline anchors.
- [oh-my-pi search](https://github.com/can1357/oh-my-pi/blob/main/docs/tools/search.md) keeps one search tool but uses a native search pipeline, per-file grouping, match limits, context lines, truncation metadata, and sparse read-cache anchoring.
- [OpenAI apply_patch](https://developers.openai.com/api/docs/guides/tools-apply-patch) models code editing as structured patch calls whose results are applied by the host and returned to the model.
- [OpenAI Codex source](https://github.com/openai/codex) shows a small local tool surface with typed tool specs, tool dispatch, hook/telemetry integration, and a dedicated `view_image` tool rather than pretending all file reads are text.

Accepted SDK lessons:

- Prefer one simple model-facing operation per job.
- Put complexity behind typed internal readers and adapters.
- Use hashes, anchors, previews, parser versions, truncation metadata, and content refs as the stable handoff between reads and later edits.
- Keep binary/media/document parsing optional and feature-gated when dependency weight or platform support warrants it.

Rejected behavior:

- Do not copy a coding-agent product UI, session cache, marketplace, prompt recipes, or product-specific path resolution into the SDK.
- Do not make `agent-sdk-core` depend on broad filesystem, PDF, OCR, RAW, browser, SQLite, or archive parsing.

## Module Layout

`crates/agent-sdk-toolkit/src/workspace/mod.rs` is a facade only. Real behavior belongs in responsibility files:

| File | Owns |
| --- | --- |
| `read.rs` | `workspace_read` executor and read output shaping |
| `read_pipeline.rs` | file-kind detection, reader-step planning, binary-vs-text routing |
| `grep.rs` | `workspace_search` executor and match output shaping |
| `edit.rs` | hashline-anchored preview/apply edits |
| `write.rs` | create/overwrite writes and before/after hashes |
| `policy.rs` | workspace bounds and tool-pack policy snapshots |
| `bounds.rs` | root, hidden-file, symlink, traversal, and relative-path enforcement |
| `anchor.rs` | hashline anchor DTOs |
| `util.rs` | shared hashing, content-ref, truncation, and error helpers |

Format readers live under named files in `workspace/readers/`, including `pdf.rs`, `media.rs`, `office.rs`, `archive.rs`, and `text.rs`. Future SQLite, URL, OCR, and richer RAW readers should follow the same pattern. Do not put parser logic in `mod.rs`.

## Read Pipeline

`workspace_read` should route every target through these stages:

1. Resolve the path or resource ref through policy.
2. Enforce root, symlink, hidden-file, max-byte, and mount boundaries.
3. Detect file type by magic bytes first, extension second, UTF-8 text third, fallback binary last.
4. Select a reader adapter and record the planned reader pipeline.
5. Emit bounded output with MIME/type, detected kind, byte length, content hash, parser version when applicable, truncation metadata, warnings, anchors when editable, and content refs.
6. Return binary/media/document summaries when a parser is absent, instead of lossy text.
7. Admit read output into model context only through the normal `ContextContribution` -> policy -> `ContextItem` pipeline.

Oversized safe files should not fail by default. `workspace_read` stats first, reads only a bounded prefix when the file is above the policy full-read cap, marks the result as truncated, and tells the agent to use `workspace_search`/grep or a narrower/range read. Readers that need full-file structure, such as PDF, OpenXML, archive, image, and SQLite readers, downgrade to bounded summaries for oversized inputs.

Initial detection should recognize:

| Kind | Examples | Default behavior |
| --- | --- | --- |
| Text/code | `.txt`, `.rs`, `.ts`, `.py`, `.toml`, UTF-8 files | Decode UTF-8, emit anchors and bounded content |
| Markdown/JSON | `.md`, `.json` | Decode UTF-8 with specific MIME/type |
| PDF | `%PDF-`, `.pdf` | Extract text with `pdf-extract`; use bounded `.ocr.txt` sidecar fallback for scanned/empty text |
| Images | PNG, JPEG, GIF, WebP, TIFF, HEIC, AVIF | Decode common image metadata; probe HEIC/AVIF dimensions; include bounded OCR sidecar text when present |
| Apple Photos / RAW | `.dng` ProRAW, `.heic`, `.cr2`, `.cr3`, `.nef`, `.arw`, `.raf`, `.rw2` | Read TIFF/DNG or BMFF dimensions; report uncompressed strip metadata, embedded JPEG preview tags, and bounded `.AAE` sidecar summaries |
| Office/docs | `.docx`, `.xlsx`, `.pptx`, `.doc`, `.xls`, `.ppt`, `.rtf`, `.epub` | Extract DOCX/PPTX/XLSX OpenXML text; route legacy `.doc`/`.xls`/`.ppt` through bounded sidecar/fallback reader with limited-fidelity warnings |
| Archives | `.zip`, `.tar`, `.tgz`, `.tar.gz`, `.gz` | List ZIP/TAR/TGZ safely; preview single-file GZIP text when UTF-8; enforce traversal and decompression caps |
| SQLite | `.sqlite`, `.sqlite3`, `.db`, SQLite magic | Open read-only/query-only; list schema and bounded sample rows |
| URI/resource | `data:`, `resource://`, `https://` | Decode local `data:` URLs; fail closed for resource/network URIs until a host resolver/network policy is attached |
| Binary | unknown non-text | Summary only |

## Reader Adapter Status

Implemented in this slice:

| Adapter | Required output | Validation criteria |
| --- | --- | --- |
| Text/code | bounded UTF-8 text, anchors, max-byte truncation | covered by toolkit pack tests |
| PDF | extracted text, page count, parser warnings | covered by checked-in `brief.pdf` fixture |
| Image | dimensions, color mode, MIME/type, parser warnings | covered by checked-in `pixel.png` fixture |
| OCR fallback | bounded `.ocr.txt` sidecar text for scanned PDFs/images; OCR-needed warnings without sidecar | covered by checked-in `ocr.png`, `scanned.pdf`, and sidecar fixtures |
| RAW / Apple Photos | RAW kind, DNG/TIFF dimensions, uncompressed strip metadata, embedded JPEG preview metadata, HEIC/AVIF-style dimension probing, bounded `.AAE` summary | covered by checked-in `photo.dng` and `photo-preview.dng` fixtures |
| Office/docs | DOCX/PPTX/XLSX OpenXML text extraction; bounded legacy `.doc`/`.xls`/`.ppt` sidecar fallback | covered by checked-in `doc.docx`, `huge.docx`, and `legacy.*` fixtures |
| Archive | ZIP/TAR/TGZ/GZIP entry listing with traversal/decompression warnings | covered by checked-in `bundle.zip`, `sample.tar`, `sample.tgz`, `sample.txt.gz`, and corrupt archive fixtures |
| SQLite | read-only/query-only schema and bounded sample row extraction | covered by checked-in `sample.sqlite` and `malformed.sqlite` fixtures |
| URI/resource | local `data:` URL read and fail-closed external/resource URI behavior | covered by explicit URI tests |

Still future work:

- Live OCR engines such as Tesseract, Vision, or cloud OCR as host-provided adapters.
- Full proprietary RAW demosaicing for camera-specific formats.
- Applying Apple Photos library adjustment stacks to pixels or reading a Photos library package ambiently.
- High-fidelity legacy binary Office layout rendering without sidecars or host converters.
- Archive entry extraction/writeback, nested archive traversal, and user-selected archive ranges.
- Arbitrary SQLite query selectors beyond bounded schema/sample reads.
- Live URL fetch/render/cache readers with network allowlists, timeouts, content-type policy, and host-owned credentials.

Release-hardening rule: before any parser is treated as release-complete, it needs deterministic fixtures for success, malformed input, oversized input, truncation, redaction, and policy denial. This slice adds checked-in success fixtures plus shared bounds/search/edit/write/symlink coverage; malformed-media and parser-denial matrices remain explicit follow-up gates.

## Search Pipeline

`workspace_search` should remain one model-facing search tool, but the implementation must:

- reject empty or invalid regex inputs with typed errors;
- skip detected binary/media/document files by default;
- honor hidden-file, symlink, root, max-file, and match-limit policy;
- report per-match path, line, line hash, preview, truncation, and pagination cursor once paging exists;
- add context-line support and gitignore/glob policy before broad repo search is considered release-ready;
- use deterministic fixtures before any native grep or AST search acceleration is accepted.

## Edit, Write, And Patch

Current primitives:

- `workspace_edit` applies one hashline-anchored replacement and can preview without writing.
- `workspace_write` creates or overwrites only when policy grants the scope and records before/after hashes.

Future `workspace_patch` should learn from Codex `apply_patch`:

- accept structured add/update/delete/move hunks;
- parse and validate before applying;
- preview changed paths, before/after hashes, and inverse candidates;
- apply only after approval policy;
- preserve partial-failure and reconciliation metadata;
- never route patching through shell text or ad hoc command strings.

## Validation Matrix

Before a workspace tool slice is complete, it needs:

| Area | Required evidence |
| --- | --- |
| Module layout | `mod.rs` facade only; behavior in named files |
| Detection | magic-vs-extension mismatch, text, PDF, image, RAW, archive, Office, fallback binary |
| Binary safety | no lossy binary text, no raw binary in events/telemetry/model output by default |
| Bounds | traversal, absolute paths, symlinks, hidden paths, max bytes |
| Read output | MIME/type, detected kind, content hash, truncation, anchors where editable |
| Search output | regex errors, binary skip, max matches, line hashes |
| Edit/write | stale anchor denial, preview-only no-write, create/overwrite policy |
| Tool execution | content refs, policy refs, effect intent/result, deterministic fakes |
| Docs | public imports, unsupported parser caveats, future adapter criteria |

## Simplicity Gate

The SDK user should not have to choose between `read_text`, `read_pdf`, `inspect_image`, and `read_raw` for the common path. They should call `workspace_read`; the SDK should detect and route. Advanced callers may request a specific reader adapter later, but that override must still lower into the same package, policy, event, journal, redaction, and content-ref contracts.
