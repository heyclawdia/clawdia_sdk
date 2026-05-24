use crate::{
    content::ContentRef,
    output::{OutputSchemaDialect, OutputSchemaRef, SchemaVersion},
    validated_output::{DecodedTypedOutput, TypedOutputError},
};

pub trait TypedOutputModel {
    const SCHEMA_ID: &'static str;
    const SCHEMA_VERSION: SchemaVersion;
    const DIALECT: OutputSchemaDialect = OutputSchemaDialect::JsonSchema2020_12Subset;

    fn schema_ref() -> OutputSchemaRef;
}

pub trait TypedOutputDeserializer<T> {
    fn deserialize(
        &self,
        canonical_value_ref: &ContentRef,
    ) -> Result<DecodedTypedOutput<T>, TypedOutputError>;
}
