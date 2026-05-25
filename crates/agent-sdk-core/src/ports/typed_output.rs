//! Host adapter boundaries for the SDK core. Use these traits and registries when
//! hosts provide providers, journals, sinks, tools, isolation, extensions, telemetry,
//! or subscriptions. Implementations may perform external side effects and must honor
//! policy, redaction, idempotency, and replay contracts. This file contains the typed
//! output portion of that contract.
//!
use crate::{
    content::ContentRef,
    output::{OutputSchemaDialect, OutputSchemaRef, SchemaVersion},
    validated_output::{DecodedTypedOutput, TypedOutputError},
};

/// Port or behavior contract for typed output model. Implementors
/// should preserve policy, redaction, idempotency, and replay
/// expectations from the surrounding module. Implementations may
/// perform side effects only as described by the trait methods.
pub trait TypedOutputModel {
    /// Stable schema identifier for this typed output model.
    const SCHEMA_ID: &'static str;
    /// Schema version expected by this typed output model.
    const SCHEMA_VERSION: SchemaVersion;
    /// Schema dialect used when lowering this model into an
    /// `OutputContract`.
    const DIALECT: OutputSchemaDialect = OutputSchemaDialect::JsonSchema2020_12Subset;

    /// Returns an updated value with schema ref configured.
    /// This returns schema or decoder metadata used by typed-output validation and performs no
    /// model call.
    fn schema_ref() -> OutputSchemaRef;
}

/// Port or behavior contract for typed output deserializer.
/// Implementors should preserve policy, redaction, idempotency, and
/// replay expectations from the surrounding module. Implementations may
/// perform side effects only as described by the trait methods.
pub trait TypedOutputDeserializer<T> {
    /// Decodes already validated JSON into the caller's typed output model.
    /// Implementations decode validated JSON into the typed model and must not call a provider
    /// or repair adapter.
    fn deserialize(
        &self,
        canonical_value_ref: &ContentRef,
    ) -> Result<DecodedTypedOutput<T>, TypedOutputError>;
}
