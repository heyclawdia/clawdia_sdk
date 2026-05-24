use core::fmt;
use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize, de::Error as DeError};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::{
    domain::{AgentError, ContentRef, IdValidationError, OutputSchemaId, PolicyKind, PolicyRef},
    ids::validate_identifier,
    policy::ContentCapturePolicy,
    typed_output_ports::TypedOutputModel,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OutputContract {
    pub schema_id: OutputSchemaId,
    pub schema_version: SchemaVersion,
    pub dialect: OutputSchemaDialect,
    pub schema: OutputSchemaRef,
    pub mode: OutputMode,
    pub validation: ValidationPolicy,
    pub repair: RepairPolicy,
    pub retry_budget: RetryBudget,
    pub content_policy: ContentCapturePolicy,
    pub projection_hint: OutputProjectionHint,
}

impl OutputContract {
    pub fn for_type<T: TypedOutputModel>() -> Self {
        Self::from_typed_model::<T>(OutputPreset::StrictJsonSchema)
    }

    pub fn strict_json_schema<T: TypedOutputModel>() -> Self {
        Self::from_typed_model::<T>(OutputPreset::StrictJsonSchema)
    }

    pub fn fast_lenient<T: TypedOutputModel>() -> Self {
        Self::from_typed_model::<T>(OutputPreset::FastLenient)
    }

    pub fn provider_assisted<T: TypedOutputModel>() -> Self {
        Self::from_typed_model::<T>(OutputPreset::ProviderAssisted)
    }

    pub fn inline_json_schema(
        schema_id: OutputSchemaId,
        schema_version: SchemaVersion,
        redacted_schema: Value,
    ) -> Self {
        let content_hash = canonical_content_hash(&redacted_schema);
        Self::new(
            schema_id,
            schema_version,
            OutputSchemaDialect::JsonSchema2020_12Subset,
            OutputSchemaRef::InlineJson {
                content_hash,
                redacted_schema,
            },
            OutputPreset::StrictJsonSchema,
        )
    }

    pub fn new(
        schema_id: OutputSchemaId,
        schema_version: SchemaVersion,
        dialect: OutputSchemaDialect,
        schema: OutputSchemaRef,
        preset: OutputPreset,
    ) -> Self {
        let mut validation = ValidationPolicy::strict_defaults();
        let mut repair = RepairPolicy::safe_defaults();
        let mut retry_budget = RetryBudget::attempts(2);
        let mut projection_hint = OutputProjectionHint::schema_ref_only();

        match preset {
            OutputPreset::StrictJsonSchema => {}
            OutputPreset::FastLenient => {
                validation.allow_additional_properties = true;
                validation.max_errors_returned = 8;
                repair.max_repair_attempts = 1;
                retry_budget = RetryBudget::attempts(1);
                projection_hint.provider_hint_policy = ProviderHintPolicy::SchemaOptional;
            }
            OutputPreset::ProviderAssisted => {
                projection_hint.provider_hint_policy = ProviderHintPolicy::ProviderNativeHint;
            }
        }

        Self {
            schema_id,
            schema_version,
            dialect,
            schema,
            mode: OutputMode::FinalOnly,
            validation,
            repair,
            retry_budget,
            content_policy: ContentCapturePolicy::safe_defaults(PolicyRef::with_kind(
                PolicyKind::Privacy,
                "policy.output.content_capture.safe_defaults",
            )),
            projection_hint,
        }
    }

    pub fn schema_fingerprint(&self) -> ContentHash {
        self.schema.content_hash()
    }

    pub fn validate_shape(&self) -> Result<(), AgentError> {
        if self.retry_budget.max_attempts < self.repair.max_repair_attempts {
            return Err(AgentError::contract_violation(
                "output retry budget must cover repair attempts",
            ));
        }
        if self.validation.max_candidate_bytes == 0 {
            return Err(AgentError::missing_required_field(
                "output.validation.max_candidate_bytes",
            ));
        }
        self.schema.validate_shape()
    }

    fn from_typed_model<T: TypedOutputModel>(preset: OutputPreset) -> Self {
        Self::new(
            OutputSchemaId::new(T::SCHEMA_ID),
            T::SCHEMA_VERSION,
            T::DIALECT,
            T::schema_ref(),
            preset,
        )
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct SchemaVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl SchemaVersion {
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputSchemaDialect {
    JsonSchema2020_12Subset,
    RustSerdeTypeName,
    HostRegistered,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutputSchemaRef {
    InlineJson {
        content_hash: ContentHash,
        redacted_schema: Value,
    },
    ContentStore {
        content_ref: ContentRef,
        content_hash: ContentHash,
    },
    RustSerde {
        type_name: TypeName,
        crate_name: CrateName,
        crate_version: String,
        generated_schema_ref: ContentRef,
        content_hash: ContentHash,
    },
    HostRegistered {
        validator_ref: OutputValidatorRef,
        contract_version: SchemaVersion,
        content_hash: ContentHash,
    },
}

impl Eq for OutputSchemaRef {}

impl OutputSchemaRef {
    pub fn content_hash(&self) -> ContentHash {
        match self {
            Self::InlineJson { content_hash, .. }
            | Self::ContentStore { content_hash, .. }
            | Self::RustSerde { content_hash, .. }
            | Self::HostRegistered { content_hash, .. } => content_hash.clone(),
        }
    }

    pub fn validate_shape(&self) -> Result<(), AgentError> {
        match self {
            Self::InlineJson {
                content_hash,
                redacted_schema,
            } => {
                content_hash.validate_shape()?;
                if redacted_schema.is_null() {
                    return Err(AgentError::missing_required_field(
                        "output.schema.redacted_schema",
                    ));
                }
            }
            Self::ContentStore { content_hash, .. }
            | Self::RustSerde { content_hash, .. }
            | Self::HostRegistered { content_hash, .. } => content_hash.validate_shape()?,
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputMode {
    FinalOnly,
    IncrementalPreview,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ValidationPolicy {
    pub validator_ref: OutputValidatorRef,
    pub max_candidate_bytes: u64,
    pub max_errors_returned: u16,
    pub allow_additional_properties: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub semantic_validators: Vec<SemanticValidatorRef>,
    pub timeout_ms: u64,
    pub failure_visibility: ValidationFailureVisibility,
}

impl ValidationPolicy {
    pub fn strict_defaults() -> Self {
        Self {
            validator_ref: OutputValidatorRef::new("validator.output.json_schema.local.v1"),
            max_candidate_bytes: 32 * 1024,
            max_errors_returned: 32,
            allow_additional_properties: false,
            semantic_validators: Vec::new(),
            timeout_ms: 10_000,
            failure_visibility: ValidationFailureVisibility::RedactedSummary,
        }
    }

    pub fn validator_ref_policy(&self) -> PolicyRef {
        PolicyRef::with_kind(PolicyKind::RuntimePackage, self.validator_ref.as_str())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationFailureVisibility {
    RedactedSummary,
    ErrorCodesOnly,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RepairPolicy {
    pub repair_adapter_ref: RepairAdapterRef,
    pub max_repair_attempts: u8,
    pub include_schema_in_prompt: bool,
    pub include_redacted_errors: bool,
    pub include_candidate_content: CandidateContentRepairPolicy,
    pub backoff: RetryBackoff,
    pub on_exhausted: RepairExhaustedBehavior,
}

impl RepairPolicy {
    pub fn safe_defaults() -> Self {
        Self {
            repair_adapter_ref: RepairAdapterRef::new("repair.output.provider_redacted.v1"),
            max_repair_attempts: 2,
            include_schema_in_prompt: true,
            include_redacted_errors: true,
            include_candidate_content: CandidateContentRepairPolicy::ContentRefOnly,
            backoff: RetryBackoff::None,
            on_exhausted: RepairExhaustedBehavior::FailRun,
        }
    }

    pub fn repair_adapter_ref_policy(&self) -> PolicyRef {
        PolicyRef::with_kind(PolicyKind::RuntimePackage, self.repair_adapter_ref.as_str())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RetryBudget {
    pub max_attempts: u8,
}

impl RetryBudget {
    pub fn attempts(max_attempts: u8) -> Self {
        Self { max_attempts }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateContentRepairPolicy {
    ContentRefOnly,
    RedactedCandidate,
    OmitCandidate,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RetryBackoff {
    None,
    Linear,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RepairExhaustedBehavior {
    FailRun,
    ReturnValidationError,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OutputProjectionHint {
    pub provider_hint_policy: ProviderHintPolicy,
    pub include_schema_ref: bool,
}

impl OutputProjectionHint {
    pub fn schema_ref_only() -> Self {
        Self {
            provider_hint_policy: ProviderHintPolicy::SchemaRequired,
            include_schema_ref: true,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderHintPolicy {
    SchemaRequired,
    SchemaOptional,
    ProviderNativeHint,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutputPreset {
    StrictJsonSchema,
    FastLenient,
    ProviderAssisted,
}

macro_rules! output_string {
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

output_string!(ContentHash);
output_string!(OutputValidatorRef);
output_string!(RepairAdapterRef);
output_string!(SemanticValidatorRef);
output_string!(TypeName);
output_string!(CrateName);

impl ContentHash {
    fn validate_shape(&self) -> Result<(), AgentError> {
        let Some(digest) = self.as_str().strip_prefix("sha256:") else {
            return Err(AgentError::contract_violation(
                "output schema content_hash must be sha256-prefixed",
            ));
        };
        if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(AgentError::contract_violation(
                "output schema content_hash must be a sha256 hex digest",
            ));
        }
        Ok(())
    }
}

fn canonical_content_hash(value: &Value) -> ContentHash {
    let normalized = normalize_json_value(value.clone());
    let bytes = serde_json::to_vec(&normalized).expect("serde_json::Value serializes");
    ContentHash::new(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn normalize_json_value(value: Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.into_iter().map(normalize_json_value).collect()),
        Value::Object(fields) => {
            let mut sorted = BTreeMap::new();
            for (key, value) in fields {
                sorted.insert(key, value);
            }
            let mut normalized = Map::new();
            for (key, value) in sorted {
                normalized.insert(key, normalize_json_value(value));
            }
            Value::Object(normalized)
        }
        scalar => scalar,
    }
}
