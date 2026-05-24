# Structured Output Contract

Structured output lets a host or user request a specific result shape without every product surface building its own parser/retry loop.

## External Lessons

- Strands handles structured-output retry inside the loop. The SDK should do the same but make every validation/repair attempt visible.
- Cursor and Claude SDKs expose simple high-level APIs over lower-level run/client primitives. The SDK should let callers request typed output without bypassing events, usage, journal, or policy.
- Workflow builders, form-like tools, CLI flows, and typed API callers need validated structures, but product rendering and business scoring stay host-owned.

## Output Contract Schema

```rust
// Non-compiling contract sketch.
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
}

pub enum OutputSchemaDialect {
    JsonSchema2020_12Subset,
    RustSerdeTypeName,
    HostSemanticValidator,
}

pub enum OutputSchemaRef {
    InlineJson {
        content_hash: ContentHash,
        bytes: RedactedBytes,
    },
    ContentStore {
        content_ref: ContentRef,
        content_hash: ContentHash,
    },
    RustSerde {
        type_name: TypeName,
        crate_name: CrateName,
        crate_version: Semver,
        generated_schema_ref: ContentRef,
    },
    HostRegistered {
        validator_id: SemanticValidatorId,
        contract_version: SchemaVersion,
    },
}

pub enum OutputMode {
    FinalOnly,
    IncrementalPreview,
    ToolAugmentedFinal,
}

pub struct ValidationPolicy {
    pub validator_ref: ValidatorRef,
    pub max_candidate_bytes: u64,
    pub max_errors_returned: u16,
    pub allow_additional_properties: bool,
    pub semantic_validators: Vec<SemanticValidatorRef>,
    pub timeout_ms: u64,
    pub failure_visibility: ValidationFailureVisibility,
}

pub struct RepairPolicy {
    pub repair_adapter_ref: RepairAdapterRef,
    pub max_repair_attempts: u8,
    pub include_schema_in_prompt: bool,
    pub include_redacted_errors: bool,
    pub include_candidate_content: CandidateContentRepairPolicy,
    pub backoff: RetryBackoff,
    pub on_exhausted: RepairExhaustedBehavior,
}

pub struct StructuredOutputResult<T> {
    pub schema_id: OutputSchemaId,
    pub schema_version: SchemaVersion,
    pub validation_attempts: Vec<ValidationAttemptId>,
    pub repair_attempts: Vec<RepairAttemptId>,
    pub source_attempt_ids: Vec<AttemptId>,
    pub output: T,
    pub output_ref: ContentRef,
    pub lineage: OutputLineage,
}

pub trait OutputValidator {
    fn dialect(&self) -> OutputSchemaDialect;
    async fn validate(
        &self,
        contract: &OutputContract,
        candidate: OutputCandidate,
    ) -> Result<ValidatedOutput, ValidationErrorReport>;
}

pub trait OutputRepairAdapter {
    async fn build_repair_request(
        &self,
        contract: &OutputContract,
        report: ValidationErrorReport,
        projection: ContextProjection,
    ) -> Result<RepairRequest, AgentError>;
}

pub trait TypedOutputModel: DeserializeOwned + Send + Sync + 'static {
    const SCHEMA_ID: &'static str;
    const SCHEMA_VERSION: SchemaVersion;
    type SchemaProvider: OutputSchemaProvider<Self>;

    fn schema_provider() -> Self::SchemaProvider;
}

pub trait OutputSchemaProvider<T: TypedOutputModel> {
    fn descriptor(&self) -> OutputSchemaDescriptor;
    fn schema_ref(&self, registry: &OutputSchemaRegistry) -> Result<OutputSchemaRef, AgentError>;
    fn schema_fingerprint(&self, registry: &OutputSchemaRegistry) -> Result<ContentHash, AgentError>;
}

pub struct OutputSchemaDescriptor {
    pub schema_id: OutputSchemaId,
    pub schema_version: SchemaVersion,
    pub dialect: OutputSchemaDialect,
    pub type_name: TypeName,
    pub compatibility: SchemaCompatibility,
}

pub struct OutputSchemaRegistry {
    pub registered_schemas: Vec<RegisteredOutputSchema>,
    pub drift_policy: SchemaDriftPolicy,
}

pub enum OutputPreset {
    StrictJsonSchema,
    FastLenient,
    ProviderAssisted,
}

pub struct OutputAdvancedConfig {
    pub validation_limits: Option<ValidationLimits>,
    pub failure_visibility: Option<ValidationFailureVisibility>,
    pub retry_budget: Option<RetryBudget>,
    pub repair_behavior: Option<RepairExhaustedBehavior>,
    pub semantic_validators: Vec<SemanticValidatorRef>,
    pub provider_hint_policy: Option<ProviderHintPolicy>,
}

impl OutputContract {
    pub fn for_type<T: TypedOutputModel>() -> TypedOutputContractBuilder<T>;
    pub fn strict_json_schema<T: TypedOutputModel>() -> TypedOutputContractBuilder<T>;
    pub fn fast_lenient<T: TypedOutputModel>() -> TypedOutputContractBuilder<T>;
    pub fn provider_assisted<T: TypedOutputModel>() -> TypedOutputContractBuilder<T>;
}
```

Phase 2 starts with a JSON Schema 2020-12 subset plus optional host semantic validators. New dialects require contract updates.

Replaceable pieces:

- `OutputValidator` can be swapped per dialect.
- `OutputRepairAdapter` can be swapped per provider or host policy.
- `SemanticValidatorRef` can point to host-owned validators, but those validators cannot execute tools or network calls unless modeled as separate tool calls.
- `OutputSchemaRef` can move schema storage from inline bytes to content store without changing the loop.
- `RepairPolicy` can be changed per runtime package without changing provider adapters.

## Typed Model Ergonomics

The easy API mirrors Pydantic-style usage: callers pass a typed model, not a hand-built parser.

```rust
// Non-compiling contract sketch.
#[derive(Deserialize, JsonSchema)]
struct TodoExtraction {
    title: String,
    priority: Priority,
    due_date: Option<String>,
}

impl TypedOutputModel for TodoExtraction {
    const SCHEMA_ID: &'static str = "todo_extraction";
    const SCHEMA_VERSION: SchemaVersion = SchemaVersion::new(1, 0, 0);
    type SchemaProvider = DerivedJsonSchema<TodoExtraction>;

    fn schema_provider() -> Self::SchemaProvider {
        DerivedJsonSchema::new()
    }
}

let todo: TodoExtraction = agent
    .run_typed::<TodoExtraction>("Extract the todo from this note.", &runtime)
    .await?;
```

This one-liner lowers to:

1. `TypedOutputModel::schema_provider()`.
2. `OutputSchemaRegistry` lookup or derive-time schema generation.
3. `OutputContract::for_type::<TodoExtraction>()`.
4. `RunRequest { output_contract: Some(contract), ... }`.
5. Local validation.
6. Construction of `TodoExtraction` only after validation succeeds.

The typed value is never built directly from raw provider text. The SDK first parses into a canonical value, validates schema and semantic validators, records events/journal records, and then deserializes into `T`.

Schema ID/version rules:

- `SCHEMA_ID` is stable and snake_case or namespaced.
- `SCHEMA_VERSION` changes when validation meaning changes.
- The schema fingerprint is computed from canonical schema bytes or a registered content ref.
- If a derived schema fingerprint differs from a registered schema with the same ID/version, package validation fails unless `SchemaDriftPolicy` explicitly allows migration.
- The provider receives a schema hint or schema ref, never Rust type metadata as authority.

## Presets And Builders

Preset mapping:

| Preset | Validation | Repair | Provider hint | Use when |
| --- | --- | --- | --- | --- |
| `strict_json_schema` | JSON schema subset, additional properties denied, conservative limits | up to 2 repairs, redacted errors only | send schema if supported | Durable workflow outputs, automation inputs, CLI JSON |
| `fast_lenient` | JSON schema subset, extra fields ignored after validation, smaller error set | 0 or 1 repair by default | hint optional | Low-risk UI summaries or local-only helpers |
| `provider_assisted` | Same local validation as strict unless overridden | up to 2 repairs | use provider-native structured output when available | Providers with strong native schema support |

Defaulted builders:

```rust
// Non-compiling contract sketch.
let contract = OutputContract::strict_json_schema::<TodoExtraction>()
    .advanced(|cfg| {
        cfg.validation_limits(ValidationLimits::default_safe())
           .failure_visibility(ValidationFailureVisibility::RedactedSummary)
           .retry_budget(RetryBudget::attempts(2));
    })
    .build(&schema_registry)?;

let policy = ValidationPolicy::builder()
    .strict_json_schema_defaults()
    .max_candidate_bytes(32 * 1024)
    .build()?;

let repair = RepairPolicy::builder()
    .safe_defaults()
    .max_repair_attempts(2)
    .candidate_content(CandidateContentRepairPolicy::ContentRefOnly)
    .build()?;
```

Helper constructors:

- `OutputContract::for_type::<T>()` uses the type provider and conservative defaults.
- `OutputContract::strict_json_schema::<T>()` denies extra properties and remote refs.
- `OutputContract::fast_lenient::<T>()` accepts provider noise only after local validation strips unknown fields.
- `OutputContract::provider_assisted::<T>()` enables provider schema hints but still performs local validation.
- `RunRequestBuilder::output::<T>()` attaches a typed output contract to a normal run.
- `Agent::run_typed::<T>()` is the one-line convenience for simple callers.

Advanced configuration can override limits, failure visibility, retry behavior, semantic validators, and provider hint policy, but it cannot bypass local validation or journal/event emission.

## Flow

```mermaid
sequenceDiagram
  participant Host
  participant Loop as "AgentLoop"
  participant Provider
  participant Validator
  participant Journal

  Host->>Loop: "RunRequest + OutputContract"
  Loop->>Journal: "StructuredOutputRequested"
  Loop->>Provider: "project schema hint if supported"
  Provider-->>Loop: "ModelOutputCandidate"
  Loop->>Journal: "StructuredOutputValidationStarted"
  Loop->>Validator: "validate candidate"
  alt valid
    Validator-->>Loop: "ValidatedOutput"
    Loop->>Journal: "StructuredOutputValidated"
  else invalid and retries remain
    Validator-->>Loop: "ValidationErrorReport"
    Loop->>Journal: "StructuredOutputValidationFailed"
    Loop->>Journal: "StructuredOutputRepairRequested"
    Loop->>Provider: "repair prompt with redacted errors"
  else invalid and budget exhausted
    Loop->>Journal: "StructuredOutputFailed"
    Loop-->>Host: "Typed validation failure"
  end
```

## Validation Rules

- Provider-native structured output is an optimization, not the source of truth.
- Local validation is authoritative.
- Streaming partials are not validated as final output unless mode explicitly supports incremental validation.
- Tool requests pause final validation until tool results are appended and a new final candidate exists.
- Repair retries are new model attempts with new attempt IDs.
- Repair prompts include schema, bounded validation errors, and minimal context needed by projection policy.
- Semantic validators must return bounded, redacted error summaries.
- Exhausted retries return a typed validation error, not best-effort parsed content.

## Schema Safety And Validator Sandbox

User-provided schemas are untrusted input.

Default schema limits:

- max schema bytes: 64 KiB
- max object depth: 24
- max properties per object: 256
- max enum values per field: 512
- max string pattern length: 2 KiB
- remote `$ref`: denied
- local `$ref`: allowed only inside the schema document
- custom `format` validators: disabled unless host registers them
- regex validators: bounded by timeout and Rust `regex` compatibility unless host registers a stronger sandbox

Semantic validators:

- run behind a timeout
- receive validated candidate data and redacted context refs, not raw hidden prompt context
- cannot execute tools or network calls unless modeled as separate tool calls
- return structured validation errors with bounded summaries
- timeout returns a redacted validation error and can trigger repair within budget

Repair prompt safety:

- include schema ID/version
- include relevant validation errors
- include no raw private candidate content unless policy allows it
- include no hidden system/developer/memory context beyond normal projection policy

## Failure Shape

Required failure fields:

- `schema_id`
- `schema_version`
- `validation_attempts`
- `repair_attempts`
- `source_attempt_ids`
- `redacted_error_summary`
- `candidate_content_ref`
- `retry_exhausted`
- `privacy`

## Acceptance Tests

- `valid_json_object_returns_validated_output`
- `missing_required_field_triggers_repair_attempt`
- `invalid_enum_value_triggers_redacted_repair_prompt`
- `tool_call_pauses_structured_validation_until_final_candidate`
- `provider_native_schema_output_is_still_locally_validated`
- `streaming_partial_is_not_committed_as_validated_output`
- `retry_budget_exhaustion_returns_typed_validation_error`
- `validated_output_preserves_source_attempt_lineage`
- `structured_output_rejects_remote_ref`
- `semantic_validator_timeout_returns_redacted_validation_error`
- `repair_prompt_excludes_raw_candidate_private_context`
- `run_typed_lowers_to_output_contract_for_type`
- `typed_result_constructed_only_after_local_validation`
- `strict_json_schema_preset_uses_conservative_defaults`
- `provider_assisted_still_locally_validates`
- `schema_registry_detects_fingerprint_drift`
- `advanced_config_cannot_disable_required_journal_events`

## Complete Example

Typed shape:

```rust
// Non-compiling contract sketch.
#[derive(Deserialize)]
struct TodoExtraction {
    title: String,
    priority: Priority,
    due_date: Option<String>,
    assignee: Option<String>,
}

let output_contract = OutputContract {
    schema_id: OutputSchemaId::new("todo_extraction"),
    schema_version: SchemaVersion::new(1, 0, 0),
    dialect: OutputSchemaDialect::JsonSchema2020_12Subset,
    schema: OutputSchemaRef::ContentStore {
        content_ref: ContentRef::new("schema/todo_extraction/v1"),
        content_hash: ContentHash::sha256("..."),
    },
    mode: OutputMode::FinalOnly,
    validation: ValidationPolicy {
        validator_ref: ValidatorRef::new("json_schema_2020_12_subset"),
        max_candidate_bytes: 32 * 1024,
        max_errors_returned: 8,
        allow_additional_properties: false,
        semantic_validators: vec![SemanticValidatorRef::new("host.todo_business_rules")],
        timeout_ms: 750,
        failure_visibility: ValidationFailureVisibility::RedactedSummary,
    },
    repair: RepairPolicy {
        repair_adapter_ref: RepairAdapterRef::new("default_provider_repair"),
        max_repair_attempts: 2,
        include_schema_in_prompt: true,
        include_redacted_errors: true,
        include_candidate_content: CandidateContentRepairPolicy::ContentRefOnly,
        backoff: RetryBackoff::FixedMillis(250),
        on_exhausted: RepairExhaustedBehavior::ReturnTypedError,
    },
    retry_budget: RetryBudget::attempts(2),
    content_policy: ContentCapturePolicy::default_redacted_refs(),
};
```

Replaceable ports:

- `OutputValidator` is selected by `validator_ref`; the core ships a JSON-schema subset validator and accepts host validators through the trait.
- `OutputRepairAdapter` is selected by `repair_adapter_ref`; provider-native JSON mode and repair prompting are optimizations, not validation authority.
- `SemanticValidatorRef` can be swapped by runtime package delta without changing the agent loop.

Wiring:

1. Host adds `output_contract` to `RunRequest`.
2. Runtime package fingerprint includes schema ID/version, schema hash, validator refs, repair policy, and retry budget.
3. Provider adapter receives only a projection hint if supported.
4. Model candidate returns as `OutputCandidate`.
5. SDK validator returns `StructuredOutputResult<TodoExtraction>` or a typed failure.

Events:

- `StructuredOutputRequested`
- `StructuredOutputValidationStarted`
- `StructuredOutputValidationFailed`
- `StructuredOutputRepairRequested`
- `ModelAttemptRetried`
- `StructuredOutputValidated` or `StructuredOutputFailed`

Journal:

- `StructuredOutputRecord { schema_id, schema_version, validation_attempt_id, candidate_ref }`
- `StructuredOutputRecord { validation_error_report_ref, redacted_error_count }`
- `ModelAttemptRecord { repair_attempt_id, source_attempt_id }`
- `StructuredOutputRecord { validated_output_ref, output_schema_hash }`

Policies and failures:

- Remote `$ref` fails package validation before the run starts.
- Semantic validator timeout records a redacted validation error and can trigger repair while budget remains.
- Exhausted repair budget returns `AgentError::StructuredOutputFailure` with the failure shape above.
- Publication of a validated output passes through `PolicyStage::Output` before it becomes the terminal typed result or an output-delivery candidate.
- Cancellation between validation and repair records `StructuredOutputFailed` with cancellation causal refs and does not start a repair attempt.

SDK owns / Host owns:

- SDK owns schema validation, repair retry accounting, lineage, events, journal records, and typed result/failure shape.
- Host owns business meaning of `TodoExtraction`, UI rendering, semantic validator implementation, retention policy, and what it does with the validated object.

Tests:

- `valid_json_object_returns_validated_output`
- `missing_required_field_triggers_repair_attempt`
- `semantic_validator_timeout_returns_redacted_validation_error`
- `retry_budget_exhaustion_returns_typed_validation_error`
- `policy_stage_output_records_decision_before_validated_output_publication`
- golden fixture: `structured_output/todo_extraction_validated_v1.json`

## Ergonomics

To keep the contract authoritative but make common usage simpler, provide a thin ergonomic layer:

- Defaulted builders for validation and repair policies, with conservative safe defaults.
- A small set of presets, for example `strict_json_schema`, `fast_lenient`, and `provider_assisted`, that map to recommended policy combinations.
- Helper constructors that infer common schema references and apply standard limits.
- An optional advanced configuration block for overriding limits, failure visibility, and retry behavior.

This keeps the wire and validation model stable while letting most callers use a one-liner and only opt into extra controls when needed.

Note: The desired ergonomics can mirror Pydantic: callers pass a typed model, the SDK derives or looks up the schema, sends only the schema or a reference to the model, then validates the response before constructing the typed result. This preserves a simple developer experience while keeping schema-first validation as the source of truth.

Simple API:

```rust
// Non-compiling contract sketch.
let todo: TodoExtraction = agent
    .run_typed::<TodoExtraction>("Extract the todo from this note.", &runtime)
    .await?;
```

Advanced API:

```rust
// Non-compiling contract sketch.
let contract = OutputContract::provider_assisted::<TodoExtraction>()
    .advanced(|cfg| {
        cfg.validation_limits(ValidationLimits { max_candidate_bytes: 16 * 1024, ..Default::default() })
           .semantic_validator(SemanticValidatorRef::new("host.todo_business_rules"))
           .retry_budget(RetryBudget::attempts(1));
    })
    .build(&schema_registry)?;

let result: RunResult = agent
    .request("Extract the todo from this note.")
    .output_contract(contract)
    .run(&runtime)
    .await?;

let todo: StructuredOutputResult<TodoExtraction> =
    result.structured_output::<TodoExtraction>()?;
```

Canonical lowering:

- `run_typed::<T>()` lowers into `RunRequestBuilder::output::<T>()`.
- `RunRequestBuilder::output::<T>()` lowers into `OutputContract::for_type::<T>()`.
- `OutputContract::for_type::<T>()` resolves `OutputSchemaProvider` through `OutputSchemaRegistry`.
- The resulting `RunRequest` carries the same canonical `OutputContract` a power user could build manually.
- `.run(&runtime)` returns `RunResult`; typed extraction is explicit through `RunResult::structured_output::<T>()`.
- `.run_typed::<T>(&runtime)` is shorthand for `.run(&runtime).await?.into_typed_output::<T>()`.

Equivalence:

- The simple and advanced paths emit the same structured-output event kinds.
- The simple and advanced paths append the same `StructuredOutputRecord` shapes.
- Retry attempts, validation failures, repair prompts, telemetry, and cost accounting are identical after lowering.
- Typed construction of `T` happens only after local validation succeeds in both paths.

SDK owns / Host owns:

- SDK owns typed-model lowering, schema registry validation, presets, local validation, repair retries, events, journal records, and typed result construction boundary.
- Host owns the model type, business semantics, optional semantic validator implementation, and product rendering of the typed value.

Tests:

- `run_typed_lowers_to_output_contract_for_type`
- `typed_result_constructed_only_after_local_validation`
- `provider_assisted_still_locally_validates`
