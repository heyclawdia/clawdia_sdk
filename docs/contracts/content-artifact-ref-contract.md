# Content And Artifact Ref Contract

`ArtifactRef` and `ContentRef` are the SDK's stable references for content-bearing data. They let tools, memory, output delivery, structured output, subagents, journals, and events talk about content without copying raw bytes through every contract or making the content provider-visible by default.

## Boundary

| Primitive | Owns | Must not own |
| --- | --- | --- |
| `ArtifactRef` | A durable or run-scoped generated/retrieved artifact identity, version, storage service, MIME/type, size/hash, privacy, retention, trust, and redacted summary. | The backing byte store, product gallery UX, provider visibility, or host retention implementation. |
| `ContentRef` | A typed pointer to message parts, artifact versions, tool results, schemas, stdout/stderr captures, output payloads, memory records, or external content. | Raw content capture by default, memory authority, or implicit provider projection. |
| `ContentResolver` | Optional port that resolves refs under policy for SDK-owned validation or host adapters. | Bypassing context admission, retention policy, or sink-specific redaction. |

## Required Fields

Every ref must carry or resolve to:

- stable ID and version;
- content kind and MIME/type when known;
- scope: run, session, host workspace, external, or persistent store;
- producer/source ref and optional destination ref;
- size/hash metadata when available;
- privacy, retention, and trust class;
- redacted summary suitable for default events;
- resolver/storage service identity;
- missing-ref and permission-denied behavior.

## Rules

- A ref does not imply provider visibility. Provider context requires `ContextContribution` admission and `ContextProjection`.
- Default events, telemetry, and journals use refs, hashes, sizes, statuses, and redacted summaries instead of raw content.
- Resolving raw content requires an explicit policy, a permitted caller/sink, and retention metadata.
- Ref resolution failures are typed: missing, expired, permission denied, storage unavailable, version mismatch, corrupted hash, and unsupported MIME/type.
- Durable replay uses refs plus journal records. If content is missing at replay time, the replay should surface a recoverable missing-ref state instead of inventing replacement content.
- Hosts own backing stores and deletion/retention enforcement. The SDK owns the ref shape, resolver port, policy checks, and no-raw-content defaults.

## Acceptance Tests

- `content_ref_does_not_imply_provider_visibility`
- `artifact_ref_requires_version_scope_privacy_retention_and_summary`
- `default_event_uses_ref_not_raw_content`
- `raw_resolution_requires_policy_and_sink_permission`
- `missing_ref_replay_returns_recoverable_state`
- `resolver_rejects_version_mismatch_and_hash_mismatch`

## Complete Example

```rust
// Non-compiling contract sketch.
let artifact = ArtifactRef::builder("artifact.release_notes")
    .version("v1")
    .mime("text/markdown")
    .scope(ContentScope::RunScoped(run_id))
    .privacy(PrivacyClass::ContentRefsOnly)
    .retention(RetentionClass::RunScoped)
    .summary("release notes artifact")
    .build()?;

let content = ContentRef::artifact(artifact)
    .kind(ContentKind::Document)
    .producer(EntityRef::tool_call(tool_call_id))
    .build()?;
```

Wiring:

1. Producer creates an artifact/content ref with source, scope, policy, privacy, retention, trust, and summary.
2. Feature contracts pass refs through events, journals, context contributions, output delivery, or validation records.
3. A resolver may load raw content only after policy allows the caller and destination.
4. Context projection admits refs through `ContextContribution` before any provider sees them.
