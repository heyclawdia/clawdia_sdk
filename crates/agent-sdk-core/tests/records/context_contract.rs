use agent_sdk_core::{
    AgentMessage, ArtifactId, ArtifactRef, ArtifactVersion, ContentId, ContentKind, ContentRef,
    ContentResolutionPolicy, ContentResolver, ContentScope, ContentVersion, ContextBudgetSummary,
    ContextContribution, ContextContributionId, ContextContributionKind, ContextItem,
    ContextProjection, ContextSelectionDecision, ContextSelectionReason, DestinationKind,
    DestinationRef, EntityKind, EntityRef, MessageId, PolicyKind, PolicyRef, PrivacyClass,
    ProjectionRole, RetentionClass, SourceKind, SourceRef, testing::FakeContentResolver,
};
use serde_json::json;

fn source() -> SourceRef {
    SourceRef::with_kind(SourceKind::Host, "source.context.test")
}

fn provider_destination() -> DestinationRef {
    DestinationRef::with_kind(DestinationKind::Provider, "destination.provider.test")
}

fn policy() -> PolicyRef {
    PolicyRef::with_kind(PolicyKind::Context, "policy.context.test")
}

fn redaction_policy() -> PolicyRef {
    PolicyRef::with_kind(PolicyKind::Redaction, "policy.redaction.test")
}

fn producer() -> EntityRef {
    EntityRef::new(EntityKind::Content, "producer.context.test")
}

fn content_ref(id: &str, summary: &str) -> ContentRef {
    let mut content_ref = ContentRef::new(
        ContentId::new(id),
        ContentVersion::new("v1"),
        ContentKind::Document,
        ContentScope::Run,
        producer(),
        source(),
        "resolver.fake.context".into(),
        summary,
    );
    content_ref.mime = Some("text/markdown".to_string());
    content_ref
}

fn contribution(id: &str, content_ref: ContentRef) -> ContextContribution {
    ContextContribution::new(
        ContextContributionId::new(id),
        ContextContributionKind::HostContext,
        producer(),
        source(),
        policy(),
        content_ref.redacted_summary.clone(),
    )
    .with_content_ref(content_ref)
}

#[test]
fn artifact_ref_requires_version_scope_privacy_retention_and_summary() {
    let artifact = ArtifactRef::new(
        ArtifactId::new("artifact.release_notes"),
        ArtifactVersion::new("v1"),
        ContentScope::Run,
        "store.fake.artifacts".into(),
        producer(),
        source(),
        "release notes artifact",
    );

    assert_eq!(artifact.version.as_str(), "v1");
    assert_eq!(artifact.scope, ContentScope::Run);
    assert_eq!(artifact.privacy_class, PrivacyClass::ContentRefsOnly);
    assert_eq!(artifact.retention_class, RetentionClass::RunScoped);
    assert_eq!(artifact.redacted_summary, "release notes artifact");
}

#[test]
fn content_ref_does_not_imply_provider_visibility() {
    let content_ref = content_ref("content.release_notes", "release notes context");

    assert!(!content_ref.provider_visible_without_context_admission());
    assert!(
        ContextProjection::default()
            .provider_visible_content_ids()
            .is_empty()
    );
}

#[test]
fn context_contribution_is_not_projected_without_admission() {
    let contribution = contribution(
        "contribution.release_notes",
        content_ref("content.release_notes", "release notes context"),
    );

    let projection = ContextProjection::build(
        "context.projection.empty".into(),
        Vec::new(),
        Vec::new(),
        vec![ContextSelectionDecision::omitted(
            &contribution,
            ContextSelectionReason::OmittedPolicy,
        )],
        provider_destination(),
        ContextBudgetSummary::default(),
        redaction_policy(),
        "runtime.package.fingerprint.test",
    )
    .expect("optional omitted contribution can be audited");

    assert!(projection.projected_parts.is_empty());
    assert_eq!(projection.audit.omitted_count, 1);
    assert_eq!(projection.audit.policy_denied_count, 1);
}

#[test]
fn admitted_context_item_becomes_provider_visible_projection() {
    let content_ref = content_ref("content.release_notes", "release notes context");
    let contribution = contribution("contribution.release_notes", content_ref.clone());
    let item = ContextItem::admit(
        contribution,
        "context.item.release_notes".into(),
        provider_destination(),
        ProjectionRole::User,
    );
    let message = AgentMessage::user_text(
        MessageId::new("message.user.1"),
        "Summarize the release notes",
        source(),
        policy(),
    );

    let projection = ContextProjection::build(
        "context.projection.release_notes".into(),
        vec![message],
        vec![item],
        Vec::new(),
        provider_destination(),
        ContextBudgetSummary {
            max_tokens: Some(8_000),
            ..ContextBudgetSummary::default()
        },
        redaction_policy(),
        "runtime.package.fingerprint.test",
    )
    .expect("admitted item projects");

    assert_eq!(
        projection.provider_visible_content_ids(),
        vec![content_ref.content_id]
    );
    assert_eq!(projection.audit.included_count, 1);
    assert_eq!(projection.audit.source_message_ids.len(), 1);
}

#[test]
fn missing_required_content_ref_blocks_provider_projection() {
    let content_ref = content_ref("content.missing.required", "missing required context");
    let contribution =
        contribution("contribution.missing.required", content_ref.clone()).required();
    let omitted =
        ContextSelectionDecision::omitted(&contribution, ContextSelectionReason::OmittedMissingRef);

    let result = ContextProjection::build(
        "context.projection.missing".into(),
        Vec::new(),
        Vec::new(),
        vec![omitted],
        provider_destination(),
        ContextBudgetSummary::default(),
        redaction_policy(),
        "runtime.package.fingerprint.test",
    );

    assert!(result.is_err());
}

#[test]
fn missing_optional_context_ref_records_projection_omission() {
    let contribution = contribution(
        "contribution.missing.optional",
        content_ref("content.missing.optional", "missing optional context"),
    );
    let projection = ContextProjection::build(
        "context.projection.optional-missing".into(),
        Vec::new(),
        Vec::new(),
        vec![ContextSelectionDecision::omitted(
            &contribution,
            ContextSelectionReason::OmittedMissingRef,
        )],
        provider_destination(),
        ContextBudgetSummary::default(),
        redaction_policy(),
        "runtime.package.fingerprint.test",
    )
    .expect("optional missing context can be omitted with audit");

    assert_eq!(projection.audit.missing_ref_count, 1);
    assert_eq!(projection.audit.omitted_count, 1);
    assert!(projection.projected_parts.is_empty());
}

#[test]
fn protected_missing_context_fails_closed() {
    let contribution = contribution(
        "contribution.protected",
        content_ref("content.protected", "protected context"),
    )
    .protected();
    let omitted = ContextSelectionDecision::omitted(
        &contribution,
        ContextSelectionReason::ProtectedOmittedByPolicy,
    );

    let result = ContextProjection::build(
        "context.projection.protected".into(),
        Vec::new(),
        Vec::new(),
        vec![omitted],
        provider_destination(),
        ContextBudgetSummary::default(),
        redaction_policy(),
        "runtime.package.fingerprint.test",
    );

    assert!(result.is_err());
}

#[test]
fn raw_content_resolution_requires_explicit_opt_in_policy() {
    let content_ref = content_ref("content.raw.opt_in", "private note summary");
    let resolver = FakeContentResolver::default();
    resolver.insert_text(&content_ref, "raw private note");

    let redacted = resolver
        .resolve(
            agent_sdk_core::ContentResolveRequest::new(content_ref.clone()),
            ContentResolutionPolicy::redacted_context(producer(), provider_destination(), policy()),
        )
        .expect("redacted resolution succeeds without bytes");
    assert!(!redacted.raw_content_included);
    assert!(redacted.bytes.is_none());
    assert_eq!(redacted.redacted_summary, "private note summary");

    let raw = resolver
        .resolve(
            agent_sdk_core::ContentResolveRequest::new(content_ref),
            ContentResolutionPolicy::raw_context(
                producer(),
                provider_destination(),
                policy(),
                1024,
            ),
        )
        .expect("explicit raw policy resolves bytes");
    assert!(raw.raw_content_included);
    assert_eq!(raw.bytes.expect("raw bytes"), b"raw private note");
}

#[test]
fn fake_content_resolver_exposes_sdk_consumer_conformance_helper() {
    let present_ref = content_ref("content.present", "present context");
    let missing_ref = content_ref("content.missing", "missing context");
    let resolver = FakeContentResolver::default();
    resolver.insert_text(&present_ref, "present text");

    FakeContentResolver::assert_conformance(
        &resolver,
        present_ref,
        missing_ref,
        ContentResolutionPolicy::redacted_context(producer(), provider_destination(), policy()),
    );
}

#[test]
fn projection_audit_golden_fixture_records_redacted_counts_only() {
    let fixture_content_ref = content_ref("content.fixture", "fixture context summary");
    let fixture_contribution = contribution("contribution.fixture", fixture_content_ref);
    let item = ContextItem::admit(
        fixture_contribution,
        "context.item.fixture".into(),
        provider_destination(),
        ProjectionRole::User,
    );
    let duplicate = contribution(
        "contribution.duplicate",
        content_ref("content.duplicate", "duplicate context summary"),
    );
    let projection = ContextProjection::build(
        "context.projection.fixture".into(),
        vec![AgentMessage::user_text(
            MessageId::new("message.fixture"),
            "Use the attached context",
            source(),
            policy(),
        )],
        vec![item],
        vec![ContextSelectionDecision::omitted(
            &duplicate,
            ContextSelectionReason::OmittedDuplicate,
        )],
        provider_destination(),
        ContextBudgetSummary {
            max_items: Some(2),
            max_tokens: Some(1_000),
            used_tokens: 42,
            ..ContextBudgetSummary::default()
        },
        redaction_policy(),
        "runtime.package.fingerprint.fixture",
    )
    .expect("fixture projection builds");

    let value = json!(projection.audit);
    let expected: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/context/projection_audit_redacted.json"
    ))
    .expect("fixture parses");
    assert_eq!(value, expected);

    let rendered = serde_json::to_string(&value).expect("audit serializes");
    assert!(!rendered.contains("Use the attached context"));
    assert!(!rendered.contains("raw"));
}
