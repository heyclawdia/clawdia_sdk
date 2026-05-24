use core::fmt;
use serde::{Deserialize, Deserializer, Serialize, de::Error as DeError};

pub const MAX_ID_LEN: usize = 512;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IdValidationError {
    Empty,
    TooLong { max: usize, actual: usize },
    ControlCharacter { index: usize },
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
        #[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self::try_new(value).expect(concat!(stringify!($name), " must be valid"))
            }

            pub fn try_new(value: impl Into<String>) -> Result<Self, IdValidationError> {
                let value = value.into();
                validate_identifier(&value)?;
                Ok(Self(value))
            }

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
pub struct JournalCursor(String);

impl JournalCursor {
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("JournalCursor must be valid")
    }

    pub fn try_new(value: impl Into<String>) -> Result<Self, IdValidationError> {
        let value = value.into();
        validate_identifier(&value)?;
        Ok(Self(value))
    }

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
pub struct CorrelationEntry {
    pub key: CorrelationKey,
    pub value: CorrelationValue,
}
