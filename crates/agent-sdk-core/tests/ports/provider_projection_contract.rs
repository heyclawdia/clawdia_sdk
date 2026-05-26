use agent_sdk_core::{
    ContextBudgetSummary, ContextContribution, ContextContributionId, ContextContributionKind,
    ContextProjection, DestinationKind, DestinationRef, EntityKind, EntityRef, PolicyKind,
    PolicyRef, PrivacyClass, ProjectionRole, ProviderAdapter, ProviderCapabilities,
    ProviderConformanceCase, ProviderMessage, ProviderMessageRole, ProviderProjectionPolicy,
    ProviderRequest, ProviderResponse, ProviderStopReason, ProviderStreamChunk,
    ProviderStreamDelta, ProviderToolCall, ProviderUsage, RetentionClass, SchemaVersion,
    SourceKind, SourceRef, ToolCallId, TrustClass,
    ids::{ContentRef as ContentRefId, ContextItemId, ContextProjectionId},
    output::OutputContract,
    testing::{FakeProvider, normalize_json_value},
    tool_records::CanonicalToolName,
};
use serde_json::json;

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

#[test]
fn provider_tool_use_response_carries_canonical_tool_call_material() {
    let tool_call = ProviderToolCall::new(
        ToolCallId::new("tool.call.provider.1"),
        CanonicalToolName::new("workspace_read"),
        "read docs/start-here.md",
    )
    .with_args_ref(ContentRefId::new("content.args.provider.1"));

    let response = ProviderResponse::tool_use([tool_call.clone()]);
    let serialized = normalize_json_value(serde_json::to_value(&response).expect("json"));

    assert_eq!(response.output_text, "");
    assert_eq!(response.stop_reason, ProviderStopReason::ToolUse);
    assert_eq!(response.tool_calls, vec![tool_call]);
    assert_eq!(
        serialized,
        serde_json::json!({
            "schema_version": 1,
            "output_text": "",
            "stop_reason": "tool_use",
            "tool_calls": [
                {
                    "tool_call_id": "tool.call.provider.1",
                    "canonical_tool_name": "workspace_read",
                    "requested_args_refs": ["content.args.provider.1"],
                    "redacted_args_summary": "read docs/start-here.md"
                }
            ]
        })
    );
}

#[test]
fn provider_debug_output_redacts_prompt_output_schema_and_tool_material() {
    let contract = OutputContract::inline_json_schema(
        agent_sdk_core::OutputSchemaId::new("schema.provider.debug"),
        SchemaVersion::new(1, 0, 0),
        json!({
            "type": "object",
            "properties": {
                "debug_secret_schema": { "const": "schema-debug-secret" }
            }
        }),
    );
    let request = ProviderRequest {
        schema_version: ProviderRequest::SCHEMA_VERSION,
        projection_policy_ref: "policy.provider.debug".to_string(),
        messages: vec![ProviderMessage {
            role: ProviderMessageRole::User,
            content: "provider-prompt-debug-secret".to_string(),
            privacy: PrivacyClass::ContentRefsOnly,
            projected_metadata: None,
        }],
        projection_item_count: 1,
        structured_output_hint: None,
    }
    .with_structured_output_hint(&contract);
    let response = ProviderResponse::text("provider-output-debug-secret");
    let tool_call = ProviderToolCall::new(
        ToolCallId::new("tool.call.debug"),
        CanonicalToolName::new("workspace_read"),
        "provider-tool-args-debug-secret",
    )
    .with_args_ref(ContentRefId::new("content.args.debug"));
    let text_chunk = ProviderStreamChunk::text(0, "provider-stream-debug-secret");
    let tool_chunk = ProviderStreamChunk::final_tool_calls(
        1,
        [tool_call.clone()],
        ProviderStopReason::ToolUse,
        None,
    );

    let debug = format!("{request:?}\n{response:?}\n{tool_call:?}\n{text_chunk:?}\n{tool_chunk:?}");

    for secret in [
        "schema-debug-secret",
        "debug_secret_schema",
        "provider-prompt-debug-secret",
        "provider-output-debug-secret",
        "provider-tool-args-debug-secret",
        "provider-stream-debug-secret",
    ] {
        assert!(
            !debug.contains(secret),
            "provider Debug output leaked {secret}: {debug}"
        );
    }
}

#[test]
fn default_provider_stream_preserves_tool_use_as_terminal_delta() {
    let provider = ToolUseProvider;
    let request = agent_sdk_core::project_context_projection(
        &admitted_projection(),
        &ProviderProjectionPolicy::redacted("policy.provider.redacted"),
    )
    .expect("projection");

    let chunks = ProviderAdapter::stream(&provider, &request).expect("stream chunks");

    assert_eq!(chunks.len(), 1);
    assert!(chunks[0].is_terminal);
    assert_eq!(chunks[0].usage.as_ref().unwrap().total_tokens, Some(9));
    assert_eq!(
        chunks[0].delta,
        ProviderStreamDelta::ToolCalls {
            tool_calls: vec![
                ProviderToolCall::new(
                    ToolCallId::new("tool.call.provider.2"),
                    CanonicalToolName::new("workspace_read"),
                    "read README.md",
                )
                .with_args_ref(ContentRefId::new("content.args.provider.2"))
            ],
            stop_reason: Some(ProviderStopReason::ToolUse),
        }
    );
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
            tool_calls: Vec::new(),
            usage: Some(ProviderUsage {
                input_tokens: Some(1),
                output_tokens: Some(1),
                total_tokens: Some(2),
            }),
        })
    }
}

struct ToolUseProvider;

impl ProviderAdapter for ToolUseProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::text_only("provider.consumer.tool_use")
    }

    fn complete(
        &self,
        _request: &ProviderRequest,
    ) -> Result<ProviderResponse, agent_sdk_core::AgentError> {
        Ok(ProviderResponse::tool_use([ProviderToolCall::new(
            ToolCallId::new("tool.call.provider.2"),
            CanonicalToolName::new("workspace_read"),
            "read README.md",
        )
        .with_args_ref(ContentRefId::new("content.args.provider.2"))])
        .with_usage(ProviderUsage {
            input_tokens: Some(4),
            output_tokens: Some(5),
            total_tokens: Some(9),
        }))
    }
}
