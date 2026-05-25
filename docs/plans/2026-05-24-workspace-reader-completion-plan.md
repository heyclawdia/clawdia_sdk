# Workspace Reader Completion Plan

Date: 2026-05-24

## Objective

Close the remaining workspace-read gaps called out after the first toolkit slice: TAR/TGZ/GZIP archive readers, SQLite readers, URL/resource-style reads through the same bounded output shape, OCR hooks for scanned PDFs/images, RAW preview/sensor metadata, Apple Photos adjustment sidecars, legacy Office fallbacks, hostile parser fixtures, and a safer default for oversized files.

The common model-facing API stays `workspace_read`. Format complexity belongs in named reader modules and typed metadata, not in `workspace/mod.rs` or `readers/mod.rs`.

## Relevant Existing Context

- `AGENTS.md`: no branch creation without explicit approval; implementation belongs in `<repo-root>`; avoid confusing catch-all files.
- `README.md` and `docs/start-here.md`: the toolkit is optional and product-neutral; `agent-sdk-core` must not absorb broad file, OCR, SQLite, archive, URL, or product host behavior.
- `coding_standards.md` and `docs/workstreams/validation-gates.md`: helpers must lower into package/policy/content-ref/effect contracts, keep `mod.rs` facades small, and prove implementation with tests/fixtures once code exists.
- `docs/contracts/tool-pack-contract.md`: read/search/edit/write/resource tools need policy refs, hashes, bounded output, parser metadata, content refs, truncation state, and no raw binary exposure by default.
- `docs/agent-sdk-toolkit/adapter-and-runtime-plan.md`: toolkit adapters must reuse existing runtime package, policy, event, journal, and content-ref primitives instead of creating a second runtime.
- `docs/agent-sdk-toolkit/workspace-toolkit-plan.md`: one `workspace_read` operation may route through typed internal readers; future SQLite, URL, OCR, RAW, and richer archive readers should live under `workspace/readers/`.
- `docs/plans/2026-05-24-workspace-toolkit-file-pipeline-plan.md`: the first slice intentionally left OCR, RAW sensor pixels/previews, Apple Photos adjustments, legacy Office, TAR/TGZ/GZIP, SQLite, URL/resource readers, and malformed fixture matrices as follow-up work.
- External lessons in `docs/architecture/external-sdk-lessons.md`: oh-my-pi demonstrates a single read surface with typed internal readers, selectors, limits, archives, SQLite, URL/resource routing, and cache/content refs; Codex demonstrates small tool surfaces, structured patching, and typed image/file handling.

## Behavior Contract

New behavior:

- `workspace_read` no longer fails just because a regular file exceeds `max_file_bytes`. It reads a bounded prefix, marks `truncated: true`, and emits model-facing guidance to use `workspace_search` or a narrower/range read for the rest. Hard failures remain for unsafe paths, hidden/symlink policy, directories, and parser/security denial.
- Text and extracted document output adds an explicit truncation warning/guidance when content is bounded.
- Archive reading supports ZIP, TAR, TGZ/TAR.GZ, and single-file GZIP with entry limits, decompression caps, path traversal filtering, parser metadata, and fixtures.
- SQLite files are detected by magic bytes or `.sqlite`/`.db` extensions and read in read-only/query-only mode. Output includes schema/table/view summaries and bounded sample rows without mutating the database.
- URL/resource-like reads are accepted through the same `WorkspaceReadRequest.path` string for safe URI classes that do not add ambient power: `data:` URLs are decoded locally and `resource://`/non-file network URLs fail closed with explicit host-adapter guidance until a host-provided resolver/network policy is attached.
- OCR fallback support is represented as a real reader stage with deterministic local behavior: adjacent `.ocr.txt` sidecars are read as host/precomputed OCR artifacts for scanned PDFs/images, and image/PDF readers emit an OCR-needed warning when no text is available. The interface leaves room for host Tesseract/Vision adapters without making them ambient dependencies.
- RAW readers parse TIFF/DNG dimensions, uncompressed strip metadata, basic embedded JPEG preview tags when present, and report sensor/preview metadata without dumping raw pixels into model-visible content.
- Apple Photos adjustment handling reads bounded `.AAE` sidecars adjacent to media files and reports adjustment metadata/warnings; Photos library databases are handled by the SQLite reader when an explicit SQLite file is read.
- Legacy Office `.doc`, `.xls`, and `.ppt` route to a named legacy-office fallback reader. When plain text can be extracted from a deterministic fixture or bounded companion text, it is emitted with warnings; unsupported binary CFB streams fail as structured warnings, not lossy bytes. This is not full legacy Office layout fidelity.
- Malformed/corrupt/encrypted-style fixtures for archive, SQLite, image/PDF/document cases prove readers do not panic, emit bounded warnings, and keep binary bytes out of model-visible content.
- Public crate exports expose the new metadata types through the crate root and `workspace` facade.

Preserved behavior:

- `agent-sdk-core` remains untouched by parser dependencies.
- Tool execution still returns a `ContentRef` envelope, not full read bodies in the tool envelope.
- Binary/media/document/archive bytes are not emitted raw by default.
- Bounds still deny traversal, hidden paths, and symlink escapes.
- Search still skips binary/media/document files by default and truncates previews.
- Edit/write semantics and preview-hash protection are unchanged.

Removed behavior:

- The previous `max_file_bytes` read failure for otherwise safe regular files is replaced by bounded-prefix truncation.
- TAR/TGZ/GZIP and legacy Office no longer share a misleading "only ZIP/OpenXML implemented" path.

## Scope

Writable files:

- `docs/agent-sdk-toolkit/**`
- `docs/contracts/tool-pack-contract.md`
- `docs/architecture/external-sdk-lessons.md`
- `docs/plans/2026-05-24-workspace-reader-completion-plan.md`
- `crates/agent-sdk-toolkit/Cargo.toml`
- `crates/agent-sdk-toolkit/README.md`
- `crates/agent-sdk-toolkit/src/lib.rs`
- `crates/agent-sdk-toolkit/src/workspace/**`
- `crates/agent-sdk-toolkit/tests/toolkit_packs.rs`
- `crates/agent-sdk-toolkit/tests/fixtures/workspace/readers/**`

Out of scope:

- Live network fetching without host policy and allowlists.
- Product-specific Apple Photos library UX or host photo-database access.
- Full proprietary RAW demosaicing for every camera vendor.
- Full binary Office fidelity/layout rendering.
- Emitting raw OCR/image/SQLite/archive content directly into model context outside the normal content-ref and context-admission pipeline.

## Workstreams

1. Bounds and output shaping
   - Replace safe oversized-file read failures with bounded-prefix reads.
   - Add explicit truncation guidance strings and tests.

2. Reader modules
   - Add dedicated reader files for SQLite, URL/resource, OCR, and legacy Office.
   - Extend `archive.rs`, `media.rs`, `pdf.rs`, and `office.rs` without moving logic into `mod.rs`.

3. Detection and metadata
   - Add `SqliteDatabase`, `UrlResource`, OCR/SQLite metadata, RAW preview/sensor fields, and reader steps.
   - Keep public metadata stable and serializable.

4. Fixtures and hostile cases
   - Add TAR/TGZ/GZIP, SQLite, data URL, OCR sidecar, AAE sidecar, legacy Office text fixture, RAW preview metadata fixture, and corrupt parser fixtures.
   - Verify malformed inputs produce typed warnings/errors without panics or raw binary leakage.

5. Docs and guidance
   - Update toolkit docs and contract criteria so future agents do not reintroduce catch-all modules or claim unsupported parser completeness.

## Oversized Read Algorithm

`workspace_read` must never `fs::read` an unsafe or oversized regular file into memory.

1. Resolve and policy-check the target path before opening it.
2. `stat` the file and record full `byte_len`.
3. Compute `max_output_bytes = request.max_bytes.unwrap_or(policy.max_output_bytes).min(policy.max_output_bytes)`.
4. If `byte_len <= policy.max_file_bytes`, read the full file and let the detected reader run normally.
5. If `byte_len > policy.max_file_bytes`, open the file and read only `min(policy.max_file_bytes, max_output_bytes).max(1)` bytes into a prefix buffer.
6. Detect file type from the prefix, set `truncated: true`, and add guidance: `file exceeds workspace max_file_bytes; returned a bounded prefix. Use workspace_search or a narrower/range read for more.`
7. Text/UTF-8 readers may render the prefix. Full-file parsers that need random access or complete structure, including PDF, Office ZIP/OpenXML, archives, SQLite, and image decoders, must downgrade to a bounded summary for oversized inputs instead of attempting full parse.
8. The output hash remains the hash of the bytes actually read into the content store unless a future streaming whole-file hash is added. The output must still include `byte_len` for the full file and a truncation warning so agents do not confuse prefix hash with whole-file hash.
9. Hard failures remain for path traversal, hidden-file denial, symlink denial, directory reads, unsupported URI policy, malformed parser errors classified as unsafe to continue, and direct parser security denial.

## Fixture And Assertion Matrix

| Feature | Fixtures / test inputs | Required assertions |
| --- | --- | --- |
| Oversized text default | temp `big.txt` larger than `max_file_bytes` | read succeeds; content length is bounded; `truncated=true`; warning tells agent to use `workspace_search` or narrower/range read; no full-file read is required |
| TAR archive | `sample.tar` | detected as archive; parser `tar`; entry listing includes safe entries; traversal entries are skipped with warnings |
| TGZ/TAR.GZ archive | `sample.tgz` | detected as archive; parser `tar+gzip`; entry listing is bounded; decompression cap is enforced |
| Single GZIP | `sample.txt.gz` | detected as archive; decompressed text preview is bounded; synthetic entry metadata is present; no raw compressed bytes emitted |
| ZIP regression | existing `bundle.zip` | ZIP behavior still passes and traversal filtering stays active |
| SQLite success | `sample.sqlite` | detected as SQLite; opens read-only/query-only; output lists tables/schema and bounded rows; no mutations or extension loading |
| SQLite malformed | `malformed.sqlite` | returns structured warning/error without panic; no raw bytes emitted |
| Data URL | request path `data:text/plain;base64,...` | routes through same read output shape; MIME/type and content hash are present; output respects `max_bytes` |
| Resource/network URL fail-closed | request paths `resource://summary` and `https://example.invalid/data.txt` | fails closed with host-adapter/network-policy guidance; no ambient resolver or network request occurs |
| Image OCR fallback | `ocr.png` plus `ocr.png.ocr.txt` | image metadata remains available; OCR sidecar text is emitted as derived document content with parser/version/warnings; output is bounded |
| PDF OCR fallback | `scanned.pdf` plus `scanned.pdf.ocr.txt` | empty/no-text PDF uses bounded OCR sidecar; no sidecar case emits OCR-needed warning |
| RAW preview/sensor metadata | `photo-preview.dng` | reports dimensions, uncompressed strip/sensor metadata when parsable, embedded preview metadata when JPEG tags are present, and does not emit raw sensor bytes |
| Apple Photos sidecar | `pixel.png.aae` or `photo-preview.dng.aae` | media output reports bounded adjustment sidecar metadata and warnings; no Photos library traversal occurs |
| Legacy Office fallbacks | `legacy.doc`, `legacy.xls`, `legacy.ppt` plus deterministic companion text where needed | each routes to legacy-office fallback; output warns about limited fidelity; bounded companion/extracted text is emitted only when available |
| Corrupt media/PDF/archive | `corrupt.png`, `corrupt.pdf`, `corrupt.zip`, `corrupt.tar`, `encrypted-placeholder.pdf` | readers do not panic; output is bounded warning or typed parser failure; raw binary bytes are not emitted |
| Truncation/no raw binary leakage | apply to each binary/parser fixture with tiny `max_bytes` | `truncated=true` where expected; `content` contains summaries/text only; binary bytes are not rendered as lossy text |

## Validation Plan

- `cargo fmt --check`
- `cargo test -p agent-sdk-toolkit --test toolkit_packs`
- `cargo test -p agent-sdk-toolkit`
- `cargo test`
- `git diff --check`
- Source-layout and boundary audits:
  - `git diff --name-only -- crates/agent-sdk-core` must be empty for this pass.
  - `find crates/agent-sdk-toolkit/src/workspace -maxdepth 3 -type f | sort` must show reader behavior in named files, not catch-all `mod.rs` files.
  - `wc -l crates/agent-sdk-toolkit/src/lib.rs crates/agent-sdk-toolkit/src/workspace/mod.rs crates/agent-sdk-toolkit/src/workspace/readers/mod.rs` must stay small enough to be clear facades.
  - Public exports for new metadata are reviewed in `crates/agent-sdk-toolkit/src/lib.rs` and `crates/agent-sdk-toolkit/src/workspace/mod.rs`.
- Independent plan review before implementation.
- Independent implementation review after tests pass, specifically checking SDK simplicity, product-neutrality, bounded output, parser failure behavior, fixture coverage, and module layout.

## Risk / Gotcha Carry-Forward

- If a reader needs host power (network, Apple Photos library access, platform OCR, proprietary RAW demosaic), expose a fail-closed adapter seam and a deterministic fixture path. Do not add ambient host access.
- If a parser extracts text from binary formats, bound both input bytes and extracted output bytes independently.
- If archive or compressed readers are extended later, preserve decompression caps and traversal filtering before entry extraction.
- If URL fetching is added later, require network permission, host allowlist, timeout, content-type policy, byte caps, and content refs.
- If OCR providers are attached later, treat OCR text as derived content with parser version, source hash, privacy policy, and truncation metadata.
- If SQLite query selectors are added later, keep read-only/query-only mode, statement allowlists, row/column/cell caps, and no extension loading.
- If legacy Office parsing grows beyond fixtures/sidecars, prefer a named optional adapter crate or host converter rather than broad native parser complexity in the common path.
