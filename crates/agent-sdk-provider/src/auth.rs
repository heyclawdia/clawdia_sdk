use std::{env, fmt};

use agent_sdk_core::AgentError;

use crate::error::host_configuration_needed;

#[derive(Clone, Eq, PartialEq)]
/// Runtime API-key material for a provider adapter.
///
/// Hosts choose how to resolve and authorize this value. The wrapper only keeps
/// accidental debug output redacted and reports missing configuration through
/// the SDK error contract.
pub struct ProviderApiKey {
    value: String,
    source: ProviderSecretSource,
}

impl ProviderApiKey {
    /// Creates an API key from an already resolved host secret.
    pub fn new(value: impl Into<String>) -> Result<Self, AgentError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(host_configuration_needed(
                "provider API key must not be empty",
            ));
        }
        Ok(Self {
            value,
            source: ProviderSecretSource::Direct,
        })
    }

    /// Resolves an API key from an environment variable.
    pub fn from_env(env_var: impl Into<String>) -> Result<Self, AgentError> {
        let env_var = env_var.into();
        let value = env::var(&env_var).map_err(|_| {
            host_configuration_needed(format!(
                "missing provider API key environment variable {env_var}"
            ))
        })?;
        if value.trim().is_empty() {
            return Err(host_configuration_needed(format!(
                "provider API key environment variable {env_var} is empty"
            )));
        }
        Ok(Self {
            value,
            source: ProviderSecretSource::Env(env_var),
        })
    }

    pub(crate) fn expose_secret(&self) -> &str {
        &self.value
    }

    /// Returns a non-secret source label suitable for diagnostics.
    pub fn source_label(&self) -> &str {
        match &self.source {
            ProviderSecretSource::Direct => "direct",
            ProviderSecretSource::Env(env_var) => env_var.as_str(),
        }
    }
}

impl fmt::Debug for ProviderApiKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderApiKey")
            .field("source", &self.source)
            .field("value", &"<redacted>")
            .finish()
    }
}

#[derive(Clone, Eq, PartialEq)]
enum ProviderSecretSource {
    Direct,
    Env(String),
}

impl fmt::Debug for ProviderSecretSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Direct => formatter.write_str("direct"),
            Self::Env(env_var) => formatter.debug_tuple("env").field(env_var).finish(),
        }
    }
}
