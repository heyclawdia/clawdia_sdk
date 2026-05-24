use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    capability::PackageSidecarRef,
    domain::{AgentError, PolicyRef},
    package::PackageSidecarSnapshot,
};

pub const REALTIME_SESSION_SIDECAR_KIND: &str = "realtime_session";
pub const REALTIME_SESSION_SIDECAR_VERSION: &str = "realtime_session.sidecar.v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RealtimeSessionSidecar {
    pub sidecar_id: String,
    pub provider_route_ref: String,
    pub realtime_capability_ref: String,
    pub media_policy_ref: PolicyRef,
    pub send_policy_ref: PolicyRef,
    pub receive_policy_ref: PolicyRef,
    pub restart_policy_ref: PolicyRef,
    pub backpressure_policy_ref: PolicyRef,
    pub interruption_policy_ref: PolicyRef,
    pub close_policy_ref: PolicyRef,
    pub queue_capacity: usize,
    pub overflow_policy: String,
}

impl RealtimeSessionSidecar {
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

    pub fn content_hash(&self) -> Result<String, AgentError> {
        self.validate()?;
        let bytes = serde_json::to_vec(self)
            .map_err(|error| AgentError::contract_violation(error.to_string()))?;
        Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
    }

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
