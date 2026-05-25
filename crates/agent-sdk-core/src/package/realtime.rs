//! Runtime-package records and builders. Use these items to describe the immutable
//! per-run package that freezes provider route, capabilities, policies, sidecars,
//! catalogs, and fingerprints. Builders are data-only and must not perform discovery
//! or execution side effects. This file contains the realtime portion of that
//! contract.
//!
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    capability::PackageSidecarRef,
    domain::{AgentError, PolicyRef},
    package::PackageSidecarSnapshot,
};

/// Constant value for the package::realtime contract. Use it to keep
/// SDK records and tests aligned on the same stable value.
pub const REALTIME_SESSION_SIDECAR_KIND: &str = "realtime_session";
/// Constant value for the package::realtime contract. Use it to keep
/// SDK records and tests aligned on the same stable value.
pub const REALTIME_SESSION_SIDECAR_VERSION: &str = "realtime_session.sidecar.v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the realtime session sidecar portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct RealtimeSessionSidecar {
    /// Identifier for the typed package sidecar.
    pub sidecar_id: String,
    /// Typed provider route ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub provider_route_ref: String,
    /// Typed realtime capability ref reference. Resolving or executing it is
    /// a separate policy-gated step.
    pub realtime_capability_ref: String,
    /// Typed media policy ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub media_policy_ref: PolicyRef,
    /// Typed send policy ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub send_policy_ref: PolicyRef,
    /// Typed receive policy ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub receive_policy_ref: PolicyRef,
    /// Typed restart policy ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub restart_policy_ref: PolicyRef,
    /// Typed backpressure policy ref reference. Resolving or executing it is
    /// a separate policy-gated step.
    pub backpressure_policy_ref: PolicyRef,
    /// Typed interruption policy ref reference. Resolving or executing it is
    /// a separate policy-gated step.
    pub interruption_policy_ref: PolicyRef,
    /// Typed close policy ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub close_policy_ref: PolicyRef,
    /// Queue capacity used by this record or request.
    pub queue_capacity: usize,
    /// Overflow policy used by this record or request.
    pub overflow_policy: String,
}

impl RealtimeSessionSidecar {
    /// Returns voice defaults for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn voice_defaults(
        sidecar_id: impl Into<String>,
        provider_route_ref: impl Into<String>,
        realtime_capability_ref: impl Into<String>,
        media_policy_ref: PolicyRef,
    ) -> Result<Self, AgentError> {
        let sidecar_id = sidecar_id.into();
        let sidecar = Self {
            sidecar_id: sidecar_id.clone(),
            provider_route_ref: provider_route_ref.into(),
            realtime_capability_ref: realtime_capability_ref.into(),
            media_policy_ref: media_policy_ref.clone(),
            send_policy_ref: PolicyRef::new(format!("policy.{sidecar_id}.send")),
            receive_policy_ref: PolicyRef::new(format!("policy.{sidecar_id}.receive")),
            restart_policy_ref: PolicyRef::new(format!("policy.{sidecar_id}.restart")),
            backpressure_policy_ref: PolicyRef::new(format!("policy.{sidecar_id}.backpressure")),
            interruption_policy_ref: PolicyRef::new(format!("policy.{sidecar_id}.interrupt")),
            close_policy_ref: PolicyRef::new(format!("policy.{sidecar_id}.close")),
            queue_capacity: 32,
            overflow_policy: "gate_during_restart".to_string(),
        };
        sidecar.validate()?;
        Ok(sidecar)
    }

    /// Validates the package::realtime invariants and returns a typed
    /// error on failure. Validation is pure and does not perform I/O,
    /// dispatch, journal appends, or adapter calls.
    pub fn validate(&self) -> Result<(), AgentError> {
        if self.sidecar_id.is_empty() {
            return Err(AgentError::missing_required_field(
                "realtime_session_sidecar.sidecar_id",
            ));
        }
        if self.provider_route_ref.is_empty() {
            return Err(AgentError::missing_required_field(
                "realtime_session_sidecar.provider_route_ref",
            ));
        }
        if self.realtime_capability_ref.is_empty() {
            return Err(AgentError::missing_required_field(
                "realtime_session_sidecar.realtime_capability_ref",
            ));
        }
        if self.queue_capacity == 0 {
            return Err(AgentError::contract_violation(
                "realtime queue capacity must be bounded and nonzero",
            ));
        }
        Ok(())
    }

    /// Computes the stable content hash for this package::realtime
    /// value. The computation is deterministic and side-effect free so
    /// it can be used in package, journal, or test evidence.
    pub fn content_hash(&self) -> Result<String, AgentError> {
        self.validate()?;
        let bytes = serde_json::to_vec(self)
            .map_err(|error| AgentError::contract_violation(error.to_string()))?;
        Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
    }

    /// Returns policy refs for the current value.
    /// This is a read-only or data-construction helper unless the method body explicitly calls
    /// a port or store.
    pub fn policy_refs(&self) -> Vec<PolicyRef> {
        vec![
            self.media_policy_ref.clone(),
            self.send_policy_ref.clone(),
            self.receive_policy_ref.clone(),
            self.restart_policy_ref.clone(),
            self.backpressure_policy_ref.clone(),
            self.interruption_policy_ref.clone(),
            self.close_policy_ref.clone(),
        ]
    }

    /// Converts this value into package sidecar snapshot data.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn to_package_sidecar_snapshot(&self) -> Result<PackageSidecarSnapshot, AgentError> {
        self.validate()?;
        Ok(PackageSidecarSnapshot {
            sidecar_id: self.sidecar_id.clone(),
            kind: REALTIME_SESSION_SIDECAR_KIND.to_string(),
            version: REALTIME_SESSION_SIDECAR_VERSION.to_string(),
            refs: vec![PackageSidecarRef {
                sidecar_id: self.realtime_capability_ref.clone(),
                kind: "provider_realtime_capability".to_string(),
                version: "v1".to_string(),
                content_hash: None,
            }],
            policy_refs: self.policy_refs(),
            content_hash: self.content_hash()?,
        })
    }
}
