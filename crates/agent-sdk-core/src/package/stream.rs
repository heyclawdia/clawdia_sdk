use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    capability::PackageSidecarRef,
    domain::{AgentError, PolicyRef, SourceRef},
    package::PackageSidecarSnapshot,
    stream_records::{StreamRule, hash_rule_fingerprint},
};

pub const STREAM_RULE_SIDECAR_KIND: &str = "stream_rule";
pub const STREAM_RULE_SIDECAR_VERSION: &str = "stream_rule.sidecar.v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StreamRuleSidecar {
    pub sidecar_id: String,
    pub source: SourceRef,
    pub rules: Vec<StreamRule>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub default_policy_refs: Vec<PolicyRef>,
    pub redaction_policy_ref: PolicyRef,
    pub content_capture_policy_ref: PolicyRef,
    pub matcher_engine_ref: String,
}

impl StreamRuleSidecar {
    pub fn new(
        sidecar_id: impl Into<String>,
        source: SourceRef,
        rules: Vec<StreamRule>,
        redaction_policy_ref: PolicyRef,
        content_capture_policy_ref: PolicyRef,
    ) -> Result<Self, AgentError> {
        let sidecar = Self {
            sidecar_id: sidecar_id.into(),
            source,
            rules,
            default_policy_refs: Vec::new(),
            redaction_policy_ref,
            content_capture_policy_ref,
            matcher_engine_ref: "sdk.safe_stream_matcher.v1".to_string(),
        };
        sidecar.validate()?;
        Ok(sidecar)
    }

    pub fn default_policy_ref(mut self, policy_ref: PolicyRef) -> Self {
        self.default_policy_refs.push(policy_ref);
        self
    }

    pub fn validate(&self) -> Result<(), AgentError> {
        if self.sidecar_id.is_empty() {
            return Err(AgentError::missing_required_field(
                "stream_rule_sidecar.sidecar_id",
            ));
        }
        if self.rules.is_empty() {
            return Err(AgentError::missing_required_field(
                "stream_rule_sidecar.rules",
            ));
        }
        if self.matcher_engine_ref.is_empty() {
            return Err(AgentError::missing_required_field(
                "stream_rule_sidecar.matcher_engine_ref",
            ));
        }
        for rule in &self.rules {
            rule.validate()?;
        }
        Ok(())
    }

    pub fn content_hash(&self) -> Result<String, AgentError> {
        self.validate()?;
        let bytes = serde_json::to_vec(self)
            .map_err(|error| AgentError::contract_violation(error.to_string()))?;
        Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
    }

    pub fn to_package_sidecar_snapshot(&self) -> Result<PackageSidecarSnapshot, AgentError> {
        self.validate()?;
        let mut refs = Vec::new();
        for rule in &self.rules {
            refs.push(PackageSidecarRef {
                sidecar_id: rule.id.as_str().to_string(),
                kind: "stream_rule".to_string(),
                version: format!("v{}", rule.version.0),
                content_hash: Some(hash_rule_fingerprint(rule)?),
            });
        }
        let mut policy_refs = self.default_policy_refs.clone();
        policy_refs.push(self.redaction_policy_ref.clone());
        policy_refs.push(self.content_capture_policy_ref.clone());
        for rule in &self.rules {
            policy_refs.extend(rule.policy_refs.clone());
        }
        Ok(PackageSidecarSnapshot {
            sidecar_id: self.sidecar_id.clone(),
            kind: STREAM_RULE_SIDECAR_KIND.to_string(),
            version: STREAM_RULE_SIDECAR_VERSION.to_string(),
            refs,
            policy_refs,
            content_hash: self.content_hash()?,
        })
    }
}
