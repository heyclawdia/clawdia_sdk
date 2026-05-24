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

## Reference Shapes And Resolver Policy

```rust
// Non-compiling contract sketch.
pub struct ArtifactRef {
    pub artifact_id: ArtifactId,
    pub version: ArtifactVersion,
    pub scope: ContentScope,
    pub storage_ref: ContentStorageRef,
    pub mime: Option<MimeType>,
    pub size_bytes: Option<u64>,
    pub content_hash: Option<ContentHash>,
    pub producer_ref: EntityRef,
    pub source_ref: SourceRef,
    pub destination_ref: Option<DestinationRef>,
    pub privacy_class: PrivacyClass,
    pub retention_class: RetentionClass,
    pub trust_class: TrustClass,
    pub redacted_summary: RedactedSummary,
}

pub struct ContentRef {
    pub content_id: ContentId,
    pub version: ContentVersion,
    pub kind: ContentKind,
    pub artifact_ref: Option<ArtifactRef>,
    pub scope: ContentScope,
    pub producer_ref: EntityRef,
    pub source_ref: SourceRef,
    pub privacy_class: PrivacyClass,
    pub retention_class: RetentionClass,
    pub trust_class: TrustClass,
    pub resolver_ref: ContentResolverRef,
    pub redacted_summary: RedactedSummary,
}

pub trait ContentResolver {
    async fn resolve(
        &self,
        request: ContentResolveRequest,
        policy: ContentResolutionPolicy,
    ) -> Result<ResolvedContent, ContentResolutionError>;
}

pub struct ContentResolutionPolicy {
    pub caller_ref: EntityRef,
    pub destination_ref: DestinationRef,
    pub purpose: ContentResolutionPurpose,
    pub allowed_privacy_classes: Vec<PrivacyClass>,
    pub max_bytes: u64,
    pub require_hash_match: bool,
    pub retention_use: RetentionUse,
    pub on_missing: MissingContentPolicy,
}

pub enum MissingContentPolicy {
    Fail,
    RecoverableReplayGap,
    OmitWithProjectionAudit,
    RequestHostRepair,
}
```

Resolution policy is evaluated before bytes are loaded. A provider adapter can receive raw resolved content only when the item has already passed through context admission and the projection policy allows raw content for that provider destination. Structured-output validators may resolve schema or candidate refs under validation policy; output sinks may resolve final output refs under sink policy. None of those resolver uses make the ref globally visible.

## Rules

- A ref does not imply provider visibility. Provider context requires `ContextContribution` admission and `ContextProjection`.
- Default events, telemetry, and journals use refs, hashes, sizes, statuses, and redacted summaries instead of raw content.
- Resolving raw content requires an explicit policy, a permitted caller/sink, and retention metadata.
- Resolver decisions are recorded as policy refs or projection audit entries when they affect model context, output validation, replay, or sink delivery.
- Ref resolution failures are typed: missing, expired, permission denied, storage unavailable, version mismatch, corrupted hash, and unsupported MIME/type.
- Durable replay uses refs plus journal records. If content is missing at replay time, the replay should surface a recoverable missing-ref state instead of inventing replacement content.
- `MissingContentPolicy::OmitWithProjectionAudit` can continue a run only when the context/output contract allows omission and records the omitted item with causal refs. It cannot silently drop required input, protected context, schemas, tool results, output candidates, or side-effect evidence.
- Hosts own backing stores and deletion/retention enforcement. The SDK owns the ref shape, resolver port, policy checks, and no-raw-content defaults.

## Acceptance Tests

- `content_ref_does_not_imply_provider_visibility`
- `artifact_ref_requires_version_scope_privacy_retention_and_summary`
- `default_event_uses_ref_not_raw_content`
- `raw_resolution_requires_policy_and_sink_permission`
- `resolver_records_policy_ref_for_raw_resolution`
- `missing_ref_replay_returns_recoverable_state`
- `missing_required_ref_blocks_provider_projection`
- `missing_optional_context_ref_records_projection_omission`
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
