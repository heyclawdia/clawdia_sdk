//! Runtime-package records and builders. Use these items to describe the immutable
//! per-run package that freezes provider route, capabilities, policies, sidecars,
//! catalogs, and fingerprints. Builders are data-only and must not perform discovery
//! or execution side effects. This file contains the stream portion of that contract.
//!
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    capability::PackageSidecarRef,
    domain::{AgentError, PolicyRef, SourceRef},
    package::PackageSidecarSnapshot,
    stream_records::{StreamRule, hash_rule_fingerprint},
};

/// Constant value for the package::stream contract. Use it to keep SDK
/// records and tests aligned on the same stable value.
pub const STREAM_RULE_SIDECAR_KIND: &str = "stream_rule";
/// Constant value for the package::stream contract. Use it to keep SDK
/// records and tests aligned on the same stable value.
pub const STREAM_RULE_SIDECAR_VERSION: &str = "stream_rule.sidecar.v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Describes the stream rule sidecar portion of a runtime package snapshot.
/// Use it when package authors or tests need explicit package configuration; validation and activation happen in package/runtime coordinators.
pub struct StreamRuleSidecar {
    /// Identifier for the typed package sidecar.
    pub sidecar_id: String,
    /// Source label or ref for this item; it is metadata and does not fetch
    /// content by itself.
    pub source: SourceRef,
    /// Network or stream-rule entries requested by policy.
    pub rules: Vec<StreamRule>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Typed default policy refs references. Resolving them is separate from
    /// constructing this record.
    pub default_policy_refs: Vec<PolicyRef>,
    /// Typed redaction policy ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub redaction_policy_ref: PolicyRef,
    /// Typed content capture policy ref reference. Resolving or executing it
    /// is a separate policy-gated step.
    pub content_capture_policy_ref: PolicyRef,
    /// Typed matcher engine ref reference. Resolving or executing it is a
    /// separate policy-gated step.
    pub matcher_engine_ref: String,
}

impl StreamRuleSidecar {
    /// Creates a new package::stream value with explicit
    /// caller-provided inputs. This constructor is data-only and
    /// performs no I/O or external side effects.
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

    /// Returns an updated value with default policy ref configured.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
    pub fn default_policy_ref(mut self, policy_ref: PolicyRef) -> Self {
        self.default_policy_refs.push(policy_ref);
        self
    }

    /// Validates the package::stream invariants and returns a typed
    /// error on failure. Validation is pure and does not perform I/O,
    /// dispatch, journal appends, or adapter calls.
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

    /// Computes the stable content hash for this package::stream value.
    /// The computation is deterministic and side-effect free so it can
    /// be used in package, journal, or test evidence.
    pub fn content_hash(&self) -> Result<String, AgentError> {
        self.validate()?;
        let bytes = serde_json::to_vec(self)
            .map_err(|error| AgentError::contract_violation(error.to_string()))?;
        Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
    }

    /// Converts this value into package sidecar snapshot data.
    /// This is data-only and does not perform I/O, call host ports, append journals, publish
    /// events, or start processes.
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
            redacted_payload: None,
        })
    }
}
