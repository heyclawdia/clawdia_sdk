//! Stable identifiers for evaluation framework records.

use serde::{Deserialize, Deserializer, Serialize, de::Error as DeError};

use agent_sdk_core::domain::{EntityId, IdValidationError};

#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
/// Stable identifier for an evaluation request or report.
pub struct EvaluationId(EntityId);

impl EvaluationId {
    /// Creates a new evaluation id.
    ///
    /// # Panics
    ///
    /// Panics when the identifier is invalid. Use `try_new` for untrusted input.
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("EvaluationId must be valid")
    }

    /// Creates a new evaluation id and returns validation errors.
    pub fn try_new(value: impl Into<String>) -> Result<Self, IdValidationError> {
        EntityId::try_new(value).map(Self)
    }

    /// Returns the id as a string slice.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<&str> for EvaluationId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl<'de> Deserialize<'de> for EvaluationId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::try_new(value).map_err(D::Error::custom)
    }
}

impl core::fmt::Debug for EvaluationId {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("EvaluationId(redacted)")
    }
}
