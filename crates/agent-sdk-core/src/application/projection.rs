//! Application-layer coordination over core primitives. Use these services to lower
//! helpers, drive runs, validate output, coordinate tools, approvals, delivery,
//! isolation, telemetry, and feature layers. Methods in this layer may call
//! configured ports, mutate in-memory stores, append journals, or publish events as
//! documented. This file contains the projection portion of that contract.
//!
use crate::{
    context::{ContextItem, ContextProjection, ProjectionRole},
    domain::{AgentError, AgentErrorKind, RetryClassification},
    provider::{
        ProviderMessage, ProviderMessageRole, ProviderProjectedMetadata, ProviderProjectionPolicy,
        ProviderRequest,
    },
};

/// Project context projection.
/// This derives a provider projection from admitted context and does not resolve raw content or
/// call the provider.
pub fn project_context_projection(
    projection: &ContextProjection,
    policy: &ProviderProjectionPolicy,
) -> Result<ProviderRequest, AgentError> {
    if projection.items.is_empty() {
        return Err(AgentError::new(
            AgentErrorKind::ProjectionFailure,
            RetryClassification::RepairNeeded,
            "provider projection requires at least one admitted context item",
        ));
    }

    Ok(ProviderRequest {
        schema_version: ProviderRequest::SCHEMA_VERSION,
        projection_policy_ref: policy.projection_policy_ref.clone(),
        projection_item_count: projection.items.len(),
        structured_output_hint: None,
        tools: Vec::new(),
        messages: projection
            .items
            .iter()
            .map(|item| project_item(item, policy))
            .collect(),
    })
}

fn project_item(item: &ContextItem, policy: &ProviderProjectionPolicy) -> ProviderMessage {
    ProviderMessage {
        role: role_for_item(item),
        content: projected_content(item),
        privacy: item.privacy_class,
        projected_metadata: policy
            .allow_private_metadata_projection
            .then(|| projected_metadata(item)),
    }
}

fn projected_content(item: &ContextItem) -> String {
    item.inline_redacted_summary
        .clone()
        .unwrap_or_else(|| item.redacted_summary.clone())
}

fn role_for_item(item: &ContextItem) -> ProviderMessageRole {
    match item.projection_role {
        ProjectionRole::System => ProviderMessageRole::System,
        ProjectionRole::Developer => ProviderMessageRole::Developer,
        ProjectionRole::User => ProviderMessageRole::User,
        ProjectionRole::ToolResult => ProviderMessageRole::Tool,
        _ => ProviderMessageRole::Context,
    }
}

fn projected_metadata(item: &ContextItem) -> ProviderProjectedMetadata {
    ProviderProjectedMetadata {
        source_kind: item.source_ref.kind.clone(),
        source_id: item.source_ref.as_str().to_string(),
        destination_kind: item.destination_ref.kind.clone(),
        destination_id: item.destination_ref.as_str().to_string(),
        subject_kind: format!("{:?}", item.producer_ref.kind),
        subject_id: item.producer_ref.as_str().to_string(),
    }
}
