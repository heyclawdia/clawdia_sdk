//! Domain primitives for stable SDK vocabulary. Use these items for IDs, refs,
//! policy, privacy, trust, and errors that cross crate or host boundaries. They are
//! data-only and must not perform provider, filesystem, network, or UI side effects.
//! This file contains the ids portion of that contract.
//!
use core::fmt;
use serde::{Deserialize, Deserializer, Serialize, de::Error as DeError};

/// Constant value for the domain::ids contract. Use it to keep SDK
/// records and tests aligned on the same stable value.
pub const MAX_ID_LEN: usize = 512;

#[derive(Clone, Debug, Eq, PartialEq)]
/// Enumerates the finite id validation error cases.
/// Serialized names are part of the SDK contract; update fixtures when variants change.
pub enum IdValidationError {
    /// Use this variant when the contract needs to represent empty; selecting it has no side effect by itself.
    Empty,
    /// Use this variant when the contract needs to represent too long; selecting it has no side effect by itself.
    TooLong {
        /// Max used by this record or request.
        max: usize,
        /// Actual used by this record or request.
        actual: usize,
    },
    /// Use this variant when the contract needs to represent control character; selecting it has no side effect by itself.
    ControlCharacter {
        /// Index used by this record or request.
        index: usize,
    },
}

impl fmt::Display for IdValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("identifier is empty"),
            Self::TooLong { max, actual } => {
                write!(formatter, "identifier length {actual} exceeds max {max}")
            }
            Self::ControlCharacter { index } => {
                write!(
                    formatter,
                    "identifier contains control character at byte {index}"
                )
            }
        }
    }
}

impl std::error::Error for IdValidationError {}

/// Validates the domain::ids invariants and returns a typed error on
/// failure. Validation is pure and does not perform I/O, dispatch,
/// journal appends, or adapter calls.
pub(crate) fn validate_identifier(value: &str) -> Result<(), IdValidationError> {
    if value.is_empty() {
        return Err(IdValidationError::Empty);
    }
    if value.len() > MAX_ID_LEN {
        return Err(IdValidationError::TooLong {
            max: MAX_ID_LEN,
            actual: value.len(),
        });
    }
    if let Some((index, _)) = value
        .char_indices()
        .find(|(_, character)| character.is_control())
    {
        return Err(IdValidationError::ControlCharacter { index });
    }
    Ok(())
}

macro_rules! id_newtype {
    ($name:ident) => {
        #[doc = concat!(
                            "Typed SDK identifier for `",
                            stringify!($name),
                            "`. Use this newtype at public boundaries instead of a raw string; ",
                            "constructing or cloning it is data-only and performs no side effects."
                        )]
        #[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Creates a new domain::ids value with explicit
            /// caller-provided inputs. This constructor is data-only
            /// and performs no I/O or external side effects.
            ///
            /// # Panics
            ///
            /// Panics if constructor invariants fail, such as invalid identifier
            /// text or constructor-specific bounds. Use a fallible constructor such as
            /// `try_new` when one is available for untrusted input.
            pub fn new(value: impl Into<String>) -> Self {
                Self::try_new(value).expect(concat!(stringify!($name), " must be valid"))
            }

            /// Creates a new domain::ids value after validation.
            /// Returns an SDK error instead of panicking when the
            /// identifier or input does not satisfy the contract.
            pub fn try_new(value: impl Into<String>) -> Result<Self, IdValidationError> {
                let value = value.into();
                validate_identifier(&value)?;
                Ok(Self(value))
            }

            /// Returns this value as str. The accessor is side-effect
            /// free and keeps ownership with the caller.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self::try_new(value).expect(concat!(stringify!($name), " must be valid"))
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::try_new(value).map_err(D::Error::custom)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(concat!(stringify!($name), "(redacted)"))
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(concat!(stringify!($name), "(redacted)"))
            }
        }
    };
}

id_newtype!(AgentId);
id_newtype!(AgentPoolId);
id_newtype!(RunId);
id_newtype!(TopicId);
id_newtype!(TurnId);
id_newtype!(AttemptId);
id_newtype!(EventId);
id_newtype!(MessageId);
id_newtype!(WakeConditionId);
id_newtype!(ApprovalRequestId);
id_newtype!(OutputSchemaId);
id_newtype!(ValidatedOutputId);
id_newtype!(ValidationAttemptId);
id_newtype!(RepairAttemptId);
id_newtype!(ContextItemId);
id_newtype!(ContextProjectionId);
id_newtype!(RuntimePackageId);
id_newtype!(EffectId);
id_newtype!(ToolCallId);
id_newtype!(SpanId);
id_newtype!(TraceId);
id_newtype!(SessionId);
id_newtype!(ContentRef);
id_newtype!(ArtifactRef);
id_newtype!(ContentId);
id_newtype!(ArtifactId);
id_newtype!(LineageId);
id_newtype!(IdempotencyKey);
id_newtype!(DedupeKey);
id_newtype!(CorrelationKey);
id_newtype!(CorrelationValue);
id_newtype!(EventCursorId);
id_newtype!(JournalCursorId);
id_newtype!(ArchiveCursorId);

#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
/// Defines the journal cursor SDK value.
/// Construction records local state only; documented runtimes, executors, or ports own side effects.
pub struct JournalCursor(String);

impl JournalCursor {
    /// Creates a new domain::ids value with explicit caller-provided
    /// inputs. This constructor is data-only and performs no I/O or
    /// external side effects.
    ///
    /// # Panics
    ///
    /// Panics if constructor invariants fail, such as invalid identifier
    /// text or constructor-specific bounds. Use a fallible constructor such as
    /// `try_new` when one is available for untrusted input.
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("JournalCursor must be valid")
    }

    /// Creates a new domain::ids value after validation. Returns an SDK
    /// error instead of panicking when the identifier or input does not
    /// satisfy the contract.
    pub fn try_new(value: impl Into<String>) -> Result<Self, IdValidationError> {
        let value = value.into();
        validate_identifier(&value)?;
        Ok(Self(value))
    }

    /// Returns this value as str. The accessor is side-effect free and
    /// keeps ownership with the caller.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for JournalCursor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::try_new(value).map_err(D::Error::custom)
    }
}

impl fmt::Debug for JournalCursor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("JournalCursor(redacted)")
    }
}

impl fmt::Display for JournalCursor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("JournalCursor(redacted)")
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Defines the correlation entry SDK value.
/// Construction records local state only; documented runtimes, executors, or ports own side effects.
pub struct CorrelationEntry {
    /// Key used by this record or request.
    pub key: CorrelationKey,
    /// Value used by this record or request.
    pub value: CorrelationValue,
}
