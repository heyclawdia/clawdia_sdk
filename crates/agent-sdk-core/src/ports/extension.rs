//! Host adapter boundaries for the SDK core. Use these traits and registries when
//! hosts provide providers, journals, sinks, tools, isolation, extensions, telemetry,
//! or subscriptions. Implementations may perform external side effects and must honor
//! policy, redaction, idempotency, and replay contracts. This file contains the
//! extension portion of that contract.
//!
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
/// Carries extension action registry snapshot data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ExtensionActionRegistrySnapshot {
    /// Fingerprint of the runtime package snapshot in force when this value was produced.
    /// Use it for replay, dedupe, and package-lineage checks; the field is evidence and does
    /// not execute package behavior.
    pub runtime_package_fingerprint: String,
    /// Collection of routes values.
    /// Ordering and membership should be treated as part of the serialized contract when
    /// relevant.
    pub routes: Vec<ExtensionActionRoute>,
}

impl ExtensionActionRegistrySnapshot {
    /// Constructs this value from resolved package. Use it when
    /// adapting canonical SDK records without introducing a second
    /// behavior path.
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

    /// Reads the stored find without registry or runtime work.
    /// This reads an in-memory extension registry and does not call the extension bridge.
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
/// Carries extension action route data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ExtensionActionRoute {
    /// Typed action ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub action_ref: ExtensionActionRef,
    /// Sidecar used by this record or request.
    pub sidecar: ResolvedExtensionActionSidecar,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries extension action request data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ExtensionActionRequest {
    /// Stable request id used for typed lineage, lookup, or dedupe.
    pub request_id: ExtensionActionRequestId,
    /// Stable extension id used for typed lineage, lookup, or dedupe.
    pub extension_id: ExtensionId,
    /// Stable action id used for typed lineage, lookup, or dedupe.
    pub action_id: ExtensionActionId,
    /// Source label or ref for this item; it is metadata and does not fetch
    /// content by itself.
    pub source: crate::domain::SourceRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Typed input refs references. Resolving them is separate from
    /// constructing this record.
    pub input_refs: Vec<ContentRef>,
    /// Safe summary of extension or tool input.
    /// It lets events and journals describe the request without exposing raw input.
    pub redacted_input_summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Idempotency setting or key for deduping retries.
    /// Use it to prevent duplicate side effects during replay or repair.
    pub idempotency_key: Option<IdempotencyKey>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Dedupe policy or key for a side-effecting operation.
    /// Replay and repair use it to avoid sending or executing the same effect twice.
    pub dedupe_key: Option<DedupeKey>,
    /// Fingerprint of the runtime package snapshot in force when this value was produced.
    /// Use it for replay, dedupe, and package-lineage checks; the field is evidence and does
    /// not execute package behavior.
    pub runtime_package_fingerprint: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries extension action execution request data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ExtensionActionExecutionRequest {
    /// Action request used by this record or request.
    pub action_request: ExtensionActionRequest,
    /// Route used by this record or request.
    pub route: ExtensionActionRoute,
    /// Effect intent used by this record or request.
    pub effect_intent: EffectIntent,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries extension action execution output data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ExtensionActionExecutionOutput {
    /// Terminal status used by this record or request.
    pub terminal_status: EffectTerminalStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Content references associated with this record; resolving them is a
    /// separate policy-gated step.
    pub content_refs: Vec<ContentRef>,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stable external operation id used for typed lineage, lookup, or
    /// dedupe.
    pub external_operation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Typed reconciliation ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub reconciliation_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Typed error ref reference. Resolving or executing it is a separate
    /// policy-gated step.
    pub error_ref: Option<String>,
}

impl ExtensionActionExecutionOutput {
    /// Returns an updated value with completed configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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

    /// Returns an updated value with failed configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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

    /// Converts this value into effect result data.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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

/// Port or behavior contract for extension action executor.
/// Implementors should preserve policy, redaction, idempotency, and
/// replay expectations from the surrounding module. Implementations may
/// perform side effects only as described by the trait methods.
pub trait ExtensionActionExecutor: Send + Sync {
    /// Returns bridge ref for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    fn bridge_ref(&self) -> &ExtensionBridgeRef;

    /// Executes one policy-approved extension action through this bridge.
    /// Implementations may call the extension transport, but the runtime owns
    /// approval, self-approval denial, and intent/result journal records.
    fn execute(
        &self,
        request: &ExtensionActionExecutionRequest,
    ) -> Result<ExtensionActionExecutionOutput, AgentError>;
}

#[derive(Clone, Default)]
/// Carries extension action executor registry data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ExtensionActionExecutorRegistry {
    executors: BTreeMap<String, Arc<dyn ExtensionActionExecutor>>,
}

impl ExtensionActionExecutorRegistry {
    /// Creates a new ports::extension value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds data to this in-memory ports::extension collection. It does not
    /// perform external I/O, execute tools, or append journals.
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

    /// Looks up an entry in this local store without registry or runtime work.
    /// This reads an in-memory extension registry and does not call the extension bridge.
    pub fn get(&self, bridge_ref: &ExtensionBridgeRef) -> Option<Arc<dyn ExtensionActionExecutor>> {
        self.executors.get(bridge_ref.as_str()).cloned()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
/// Carries extension protocol request id data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ExtensionProtocolRequestId(String);

impl ExtensionProtocolRequestId {
    /// Creates a new ports::extension value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        assert!(
            !value.is_empty(),
            "ExtensionProtocolRequestId must not be empty"
        );
        Self(value)
    }

    /// Returns this value as str. The accessor is side-effect free and
    /// keeps ownership with the caller.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries extension protocol version data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ExtensionProtocolVersion {
    /// Version string for this capability, package, or protocol surface.
    /// Use it for compatibility checks during package or adapter resolution.
    pub version: u16,
}

impl ExtensionProtocolVersion {
    /// Constant value for the ports::extension contract. Use it to keep
    /// SDK records and tests aligned on the same stable value.
    pub const SUPPORTED_VERSION: u16 = 1;

    /// Negotiate.
    /// This validates protocol-version compatibility and returns an error before any bridge
    /// call when unsupported.
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
/// Enumerates the finite extension protocol error kind cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum ExtensionProtocolErrorKind {
    /// Use this variant when the contract needs to represent unsupported protocol version; selecting it has no side effect by itself.
    UnsupportedProtocolVersion,
    /// Use this variant when the contract needs to represent response id mismatch; selecting it has no side effect by itself.
    ResponseIdMismatch,
    /// Use this variant when the contract needs to represent malformed message; selecting it has no side effect by itself.
    MalformedMessage,
    /// Use this variant when the contract needs to represent transport closed; selecting it has no side effect by itself.
    TransportClosed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Carries extension protocol error data across a host-port boundary.
/// Constructing the value does not call the host; the port method that receives it documents any adapter, network, or storage effect.
pub struct ExtensionProtocolError {
    /// Kind/category for this record, capability, event, or detected
    /// resource.
    pub kind: ExtensionProtocolErrorKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stable request id used for typed lineage, lookup, or dedupe.
    pub request_id: Option<ExtensionProtocolRequestId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stable expected response id used for typed lineage, lookup, or dedupe.
    pub expected_response_id: Option<ExtensionProtocolRequestId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Stable actual response id used for typed lineage, lookup, or dedupe.
    pub actual_response_id: Option<ExtensionProtocolRequestId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Version string for this capability, package, or protocol surface.
    /// Use it for compatibility checks during package or adapter resolution.
    pub protocol_version: Option<u16>,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

/// Validates the ports::extension invariants and returns a typed error
/// on failure. Validation is pure and does not perform I/O, dispatch,
/// journal appends, or adapter calls.
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
