use std::{collections::BTreeMap, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::{
    domain::{AgentError, ContentRef, DedupeKey, EffectId, IdempotencyKey},
    effect::{EffectIntent, EffectResult, EffectTerminalStatus},
    package_extension::{
        ExtensionActionId, ExtensionActionRef, ExtensionActionRequestId, ExtensionBridgeRef,
        ExtensionId, ResolvedExtensionActionSidecar, ResolvedExtensionPackage,
    },
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExtensionActionRegistrySnapshot {
    pub runtime_package_fingerprint: String,
    pub routes: Vec<ExtensionActionRoute>,
}

impl ExtensionActionRegistrySnapshot {
    pub fn from_resolved_package(package: &ResolvedExtensionPackage) -> Result<Self, AgentError> {
        let mut routes = Vec::new();
        for sidecar in &package.action_sidecars {
            sidecar.validate()?;
            routes.push(ExtensionActionRoute {
                action_ref: sidecar.action_ref.clone(),
                sidecar: sidecar.clone(),
            });
        }
        routes.sort_by_key(|route| route.action_ref.subject_id());
        Ok(Self {
            runtime_package_fingerprint: package.runtime_package_fingerprint.clone(),
            routes,
        })
    }

    pub fn find(
        &self,
        extension_id: &ExtensionId,
        action_id: &ExtensionActionId,
    ) -> Option<&ExtensionActionRoute> {
        self.routes.iter().find(|route| {
            &route.action_ref.extension_id == extension_id
                && &route.action_ref.action_id == action_id
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExtensionActionRoute {
    pub action_ref: ExtensionActionRef,
    pub sidecar: ResolvedExtensionActionSidecar,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExtensionActionRequest {
    pub request_id: ExtensionActionRequestId,
    pub extension_id: ExtensionId,
    pub action_id: ExtensionActionId,
    pub source: crate::domain::SourceRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input_refs: Vec<ContentRef>,
    pub redacted_input_summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<IdempotencyKey>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dedupe_key: Option<DedupeKey>,
    pub runtime_package_fingerprint: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExtensionActionExecutionRequest {
    pub action_request: ExtensionActionRequest,
    pub route: ExtensionActionRoute,
    pub effect_intent: EffectIntent,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExtensionActionExecutionOutput {
    pub terminal_status: EffectTerminalStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content_refs: Vec<ContentRef>,
    pub redacted_summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_operation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reconciliation_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_ref: Option<String>,
}

impl ExtensionActionExecutionOutput {
    pub fn completed(redacted_summary: impl Into<String>) -> Self {
        Self {
            terminal_status: EffectTerminalStatus::Completed,
            content_refs: Vec::new(),
            redacted_summary: redacted_summary.into(),
            external_operation_id: None,
            reconciliation_ref: None,
            error_ref: None,
        }
    }

    pub fn failed(redacted_summary: impl Into<String>, error_ref: impl Into<String>) -> Self {
        Self {
            terminal_status: EffectTerminalStatus::Failed,
            content_refs: Vec::new(),
            redacted_summary: redacted_summary.into(),
            external_operation_id: None,
            reconciliation_ref: None,
            error_ref: Some(error_ref.into()),
        }
    }

    pub fn to_effect_result(&self, effect_id: EffectId) -> EffectResult {
        EffectResult {
            effect_id,
            terminal_status: self.terminal_status.clone(),
            external_operation_id: self.external_operation_id.clone(),
            reconciliation_ref: self.reconciliation_ref.clone(),
            error_ref: self.error_ref.clone(),
            content_refs: self.content_refs.clone(),
            redacted_summary: self.redacted_summary.clone(),
        }
    }
}

pub trait ExtensionActionExecutor: Send + Sync {
    fn bridge_ref(&self) -> &ExtensionBridgeRef;

    fn execute(
        &self,
        request: &ExtensionActionExecutionRequest,
    ) -> Result<ExtensionActionExecutionOutput, AgentError>;
}

#[derive(Clone, Default)]
pub struct ExtensionActionExecutorRegistry {
    executors: BTreeMap<String, Arc<dyn ExtensionActionExecutor>>,
}

impl ExtensionActionExecutorRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &mut self,
        executor: Arc<dyn ExtensionActionExecutor>,
    ) -> Result<(), AgentError> {
        let bridge_ref = executor.bridge_ref().as_str().to_string();
        if bridge_ref.is_empty() {
            return Err(AgentError::missing_required_field(
                "extension_action_executor.bridge_ref",
            ));
        }
        self.executors.insert(bridge_ref, executor);
        Ok(())
    }

    pub fn get(&self, bridge_ref: &ExtensionBridgeRef) -> Option<Arc<dyn ExtensionActionExecutor>> {
        self.executors.get(bridge_ref.as_str()).cloned()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct ExtensionProtocolRequestId(String);

impl ExtensionProtocolRequestId {
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        assert!(
            !value.is_empty(),
            "ExtensionProtocolRequestId must not be empty"
        );
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExtensionProtocolVersion {
    pub version: u16,
}

impl ExtensionProtocolVersion {
    pub const SUPPORTED_VERSION: u16 = 1;

    pub fn negotiate(version: u16) -> Result<Self, ExtensionProtocolError> {
        if version != Self::SUPPORTED_VERSION {
            return Err(ExtensionProtocolError {
                kind: ExtensionProtocolErrorKind::UnsupportedProtocolVersion,
                request_id: None,
                expected_response_id: None,
                actual_response_id: None,
                protocol_version: Some(version),
                redacted_summary: format!("unsupported extension protocol version {version}"),
            });
        }
        Ok(Self { version })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionProtocolErrorKind {
    UnsupportedProtocolVersion,
    ResponseIdMismatch,
    MalformedMessage,
    TransportClosed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExtensionProtocolError {
    pub kind: ExtensionProtocolErrorKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<ExtensionProtocolRequestId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_response_id: Option<ExtensionProtocolRequestId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_response_id: Option<ExtensionProtocolRequestId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol_version: Option<u16>,
    pub redacted_summary: String,
}

pub fn validate_extension_protocol_response_id(
    expected: &ExtensionProtocolRequestId,
    actual: &ExtensionProtocolRequestId,
) -> Result<(), ExtensionProtocolError> {
    if expected != actual {
        return Err(ExtensionProtocolError {
            kind: ExtensionProtocolErrorKind::ResponseIdMismatch,
            request_id: Some(expected.clone()),
            expected_response_id: Some(expected.clone()),
            actual_response_id: Some(actual.clone()),
            protocol_version: Some(ExtensionProtocolVersion::SUPPORTED_VERSION),
            redacted_summary: "extension protocol response id mismatch".to_string(),
        });
    }
    Ok(())
}
