use serde_json::{Value, json};

use agent_sdk_core::{
    Agent, AgentId, ContentHash, OutputContract, OutputContractSnapshot, OutputSchemaId,
    OutputSchemaRef, ProviderHintPolicy, ProviderRouteSnapshot, RunId, RunRequest, RuntimePackage,
    RuntimePackageId, SchemaVersion, SourceKind, SourceRef, TypedOutputModel,
    domain::ContentRef as ContentRefId, testing::read_fixture,
};

struct TodoExtraction;

impl TypedOutputModel for TodoExtraction {
    const SCHEMA_ID: &'static str = "schema.todo_extraction";
    const SCHEMA_VERSION: SchemaVersion = SchemaVersion::new(1, 0, 0);

    fn schema_ref() -> OutputSchemaRef {
        OutputSchemaRef::ContentStore {
            content_ref: ContentRefId::new("content.schema.todo_extraction.v1"),
            content_hash: ContentHash::new(
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            ),
        }
    }
}

#[test]
fn output_contract_schema_ref_matches_golden_fixture() {
    let contract = OutputContract::inline_json_schema(
        OutputSchemaId::new("schema.todo_inline"),
        SchemaVersion::new(1, 2, 0),
        json!({
            "type": "object",
            "required": ["title"],
            "properties": {
                "title": {"type": "string"}
            },
            "additionalProperties": false
        }),
    );

    contract.validate_shape().expect("contract shape");
    assert_eq!(
        normalize(serde_json::to_value(contract).expect("contract JSON")),
        read_fixture("tests/fixtures/output_contract/inline-json-contract.json")
            .expect("output contract fixture")
    );
}

#[test]
fn typed_output_helper_lowers_to_canonical_run_request() {
    let agent = Agent::builder()
        .id(AgentId::new("agent.output.contract"))
        .name("output contract")
        .build()
        .expect("agent builds");
    let source = SourceRef::with_kind(SourceKind::Host, "source.output.contract");

    let request = agent.typed_text_request::<TodoExtraction>(
        RunId::new("run.output.typed"),
        source.clone(),
        "extract a todo",
    );
    let expected = RunRequest::text(
        RunId::new("run.output.typed"),
        agent.id().clone(),
        source,
        "extract a todo",
    )
    .with_output_contract(OutputContract::for_type::<TodoExtraction>());

    assert_eq!(request, expected);
    assert_eq!(
        request.output_contract.as_ref().unwrap().schema_id,
        OutputSchemaId::new("schema.todo_extraction")
    );
}

#[test]
fn runtime_package_normalizes_request_output_contract_into_fingerprint() {
    let agent = Agent::builder()
        .id(AgentId::new("agent.output.package"))
        .name("output package")
        .build()
        .expect("agent builds");
    let package = runtime_package(&agent);
    let baseline = package.fingerprint().expect("baseline fingerprint");
    let runtime = agent_sdk_core::AgentRuntime::builder()
        .default_package(package.clone())
        .build()
        .expect("runtime builds");
    let request = RunRequest::typed_text::<TodoExtraction>(
        RunId::new("run.output.package.1"),
        agent.id().clone(),
        SourceRef::with_kind(SourceKind::Host, "source.output.package"),
        "extract a todo",
    );

    let effective = runtime
        .resolve_effective_package(&request)
        .expect("effective package");
    let expected_snapshot =
        OutputContractSnapshot::from(request.output_contract.as_ref().expect("contract"));

    assert_eq!(effective.package.output_contracts, vec![expected_snapshot]);
    assert_ne!(effective.fingerprint, baseline);

    let second = runtime
        .resolve_effective_package(&RunRequest::typed_text::<TodoExtraction>(
            RunId::new("run.output.package.2"),
            agent.id().clone(),
            SourceRef::with_kind(SourceKind::Host, "source.output.package"),
            "extract a different todo",
        ))
        .expect("second effective package");
    assert_eq!(
        effective.fingerprint, second.fingerprint,
        "run IDs and prompt text must not alter package fingerprint for the same output contract"
    );
}

#[test]
fn output_policy_changes_are_package_fingerprint_inputs() {
    let agent = output_agent("agent.output.fingerprint");
    let package = runtime_package(&agent);
    let baseline_contract = OutputContract::for_type::<TodoExtraction>();
    let baseline = package
        .clone()
        .with_output_contract(&baseline_contract)
        .expect("baseline package")
        .fingerprint()
        .expect("baseline fingerprint");

    let mut validation_changed = baseline_contract.clone();
    validation_changed.validation.max_candidate_bytes += 1;
    assert_ne!(baseline, fingerprint_for(&package, &validation_changed));

    let mut repair_changed = baseline_contract.clone();
    repair_changed.repair.max_repair_attempts = 1;
    repair_changed.retry_budget.max_attempts = 1;
    assert_ne!(baseline, fingerprint_for(&package, &repair_changed));

    let mut retry_changed = baseline_contract.clone();
    retry_changed.retry_budget.max_attempts += 1;
    assert_ne!(baseline, fingerprint_for(&package, &retry_changed));

    let mut projection_changed = baseline_contract.clone();
    projection_changed.projection_hint.include_schema_ref = false;
    assert_ne!(baseline, fingerprint_for(&package, &projection_changed));

    let mut content_policy_changed = baseline_contract.clone();
    content_policy_changed.content_policy.byte_limit = 1;
    assert_ne!(baseline, fingerprint_for(&package, &content_policy_changed));
}

#[test]
fn inline_schema_fingerprint_is_derived_from_canonical_schema_json() {
    let first = OutputContract::inline_json_schema(
        OutputSchemaId::new("schema.inline.hash"),
        SchemaVersion::new(1, 0, 0),
        json!({
            "required": ["title"],
            "type": "object",
            "properties": {
                "title": {"type": "string"}
            }
        }),
    );
    let reordered = OutputContract::inline_json_schema(
        OutputSchemaId::new("schema.inline.hash"),
        SchemaVersion::new(1, 0, 0),
        json!({
            "properties": {
                "title": {"type": "string"}
            },
            "type": "object",
            "required": ["title"]
        }),
    );
    let changed = OutputContract::inline_json_schema(
        OutputSchemaId::new("schema.inline.hash"),
        SchemaVersion::new(1, 0, 0),
        json!({
            "properties": {
                "title": {"type": "string"},
                "priority": {"type": "string"}
            },
            "type": "object",
            "required": ["title"]
        }),
    );

    assert_eq!(first.schema_fingerprint(), reordered.schema_fingerprint());
    assert_ne!(first.schema_fingerprint(), changed.schema_fingerprint());
    assert_eq!(
        first.schema_fingerprint().as_str().len(),
        "sha256:".len() + 64
    );
    assert!(
        first
            .schema_fingerprint()
            .as_str()
            .strip_prefix("sha256:")
            .unwrap()
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    );
}

#[test]
fn provider_assisted_contract_is_hint_only_and_keeps_local_validation_policy() {
    let contract = OutputContract::provider_assisted::<TodoExtraction>();

    assert_eq!(
        contract.projection_hint.provider_hint_policy,
        ProviderHintPolicy::ProviderNativeHint
    );
    assert_eq!(
        contract.validation.validator_ref.as_str(),
        "validator.output.json_schema.local.v1",
        "provider-native schema mode is a hint, not validation authority"
    );
}

#[test]
fn output_contract_phase_does_not_publish_validation_or_repair_results() {
    let source = include_str!("../../src/records/output.rs");
    for forbidden in [
        "StructuredOutputValidated",
        "ValidationErrorReport",
        "pub struct StructuredOutputResult",
        "impl OutputValidator",
        "impl OutputRepairAdapter",
    ] {
        assert!(
            !source.contains(forbidden),
            "{forbidden} belongs to Phase 07/08, not Phase 06"
        );
    }
}

fn runtime_package(agent: &Agent) -> RuntimePackage {
    RuntimePackage::builder(RuntimePackageId::new("package.output.contract"))
        .agent(agent.snapshot())
        .provider_route(ProviderRouteSnapshot::new("provider.fake", "model.fake"))
        .build()
        .expect("package builds")
}

fn output_agent(agent_id: &str) -> Agent {
    Agent::builder()
        .id(AgentId::new(agent_id))
        .name("output package")
        .build()
        .expect("agent builds")
}

fn fingerprint_for(
    package: &RuntimePackage,
    contract: &OutputContract,
) -> agent_sdk_core::RuntimePackageFingerprint {
    package
        .clone()
        .with_output_contract(contract)
        .expect("output package")
        .fingerprint()
        .expect("fingerprint")
}

fn normalize(value: Value) -> Value {
    agent_sdk_core::testing::normalize_json_value(value)
}
