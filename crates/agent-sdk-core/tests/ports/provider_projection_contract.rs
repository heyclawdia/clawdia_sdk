use agent_sdk_core::{
    ContextBudgetSummary, ContextContribution, ContextContributionId, ContextContributionKind,
    ContextProjection, DestinationKind, DestinationRef, EntityKind, EntityRef, PolicyKind,
    PolicyRef, PrivacyClass, ProjectionRole, ProviderAdapter, ProviderCapabilities,
    ProviderConformanceCase, ProviderProjectionPolicy, ProviderRequest, ProviderResponse,
    ProviderStopReason, ProviderUsage, RetentionClass, SourceKind, SourceRef, TrustClass,
    ids::{ContextItemId, ContextProjectionId},
    testing::{FakeProvider, normalize_json_value},
};

fn admitted_projection() -> ContextProjection {
    let mut source = SourceRef::with_kind(SourceKind::User, "source.secret.ui");
    source.trust = TrustClass::UserProvided;
    source.privacy = PrivacyClass::Sensitive;
    source.redacted_summary = Some("user chat surface".to_string());

    let mut destination =
        DestinationRef::with_kind(DestinationKind::Provider, "destination.secret.provider");
    destination.privacy = PrivacyClass::ContentRefsOnly;
    destination.retention = RetentionClass::RunScoped;
    destination.redacted_summary = Some("provider context".to_string());

    let mut subject = EntityRef::new(EntityKind::Message, "message.secret.private");
    subject.privacy = PrivacyClass::Sensitive;
    subject.redacted_summary = Some("release notes request".to_string());

    let policy = PolicyRef::with_kind(PolicyKind::Context, "policy.context.provider-test");
    let mut contribution = ContextContribution::new(
        ContextContributionId::new("contribution.secret.private"),
        ContextContributionKind::UserInput,
        subject,
        source,
        policy,
        "release notes request",
    );
    contribution.inline_redacted_summary = Some("release notes request".to_string());
    contribution.privacy_class = PrivacyClass::Sensitive;
    contribution.trust_class = TrustClass::UserProvided;

    let item = agent_sdk_core::ContextItem::admit(
        contribution,
        ContextItemId::new("context.item.secret.private"),
        destination.clone(),
        ProjectionRole::User,
    );

    ContextProjection::build(
        ContextProjectionId::new("context.projection.provider-test"),
        Vec::new(),
        vec![item],
        Vec::new(),
        destination,
        ContextBudgetSummary::default(),
        PolicyRef::with_kind(PolicyKind::Redaction, "policy.redaction.provider-test"),
        "runtime.package.provider-test",
    )
    .expect("context projection")
}

#[test]
fn provider_request_projects_from_context_projection_only_and_strips_private_metadata() {
    let request = agent_sdk_core::project_context_projection(
        &admitted_projection(),
        &ProviderProjectionPolicy::redacted("policy.provider.redacted"),
    )
    .expect("projection");

    let serialized = normalize_json_value(serde_json::to_value(&request).expect("json"));
    let rendered = serde_json::to_string(&serialized).expect("rendered json");

    assert_eq!(
        serialized,
        serde_json::from_str::<serde_json::Value>(include_str!(
            "../fixtures/provider/projected_request_redacted.json"
        ))
        .expect("fixture")
    );
    assert!(!rendered.contains("source.secret.ui"));
    assert!(!rendered.contains("destination.secret.provider"));
    assert!(!rendered.contains("message.secret.private"));
    assert_eq!(request.messages[0].content, "release notes request");
    assert!(request.messages[0].projected_metadata.is_none());
}

#[test]
fn explicit_projection_policy_allows_private_metadata_shell() {
    let request = agent_sdk_core::project_context_projection(
        &admitted_projection(),
        &ProviderProjectionPolicy::allow_private_metadata("policy.provider.metadata"),
    )
    .expect("projection");

    let metadata = request.messages[0]
        .projected_metadata
        .as_ref()
        .expect("metadata projected by explicit policy");

    assert_eq!(metadata.source_id, "source.secret.ui");
    assert_eq!(metadata.destination_id, "destination.secret.provider");
    assert_eq!(metadata.subject_id, "message.secret.private");
}

#[test]
fn fake_provider_records_typed_requests_and_extracts_usage_without_network() {
    let provider = FakeProvider::with_responses(["deterministic fake response"]);
    let request = agent_sdk_core::project_context_projection(
        &admitted_projection(),
        &ProviderProjectionPolicy::redacted("policy.provider.redacted"),
    )
    .expect("projection");

    let response = ProviderAdapter::complete(&provider, &request).expect("provider response");
    let usage = provider.extract_usage(&response);

    assert_eq!(response.output_text, "deterministic fake response");
    assert_eq!(provider.requests(), vec![request]);
    assert_eq!(usage.input_tokens, Some(3));
    assert_eq!(usage.output_tokens, Some(3));
    assert_eq!(usage.total_tokens, Some(6));
}

#[test]
fn sdk_consumer_mock_can_run_shared_provider_conformance_helper() {
    let mock = SdkConsumerMockProvider;
    let usage = ProviderConformanceCase::new(admitted_projection())
        .assert_adapter(&mock)
        .expect("conformance");

    assert_eq!(usage.total_tokens, Some(2));
}

struct SdkConsumerMockProvider;

impl ProviderAdapter for SdkConsumerMockProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::text_only("provider.consumer.mock")
    }

    fn complete(
        &self,
        request: &ProviderRequest,
    ) -> Result<ProviderResponse, agent_sdk_core::AgentError> {
        assert_eq!(request.projection_item_count, 1);
        assert!(request.messages[0].projected_metadata.is_none());
        Ok(ProviderResponse {
            schema_version: ProviderResponse::SCHEMA_VERSION,
            output_text: "consumer ok".to_string(),
            stop_reason: ProviderStopReason::EndTurn,
            usage: Some(ProviderUsage {
                input_tokens: Some(1),
                output_tokens: Some(1),
                total_tokens: Some(2),
            }),
        })
    }
}
