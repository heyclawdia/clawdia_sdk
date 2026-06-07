use std::collections::BTreeSet;

use agent_sdk_core::{
    AgentId, AgentSnapshot, CapabilitySpec, ExecutorRef, PackageDelta, PackageSidecarRef,
    PackageSidecarSnapshot, PolicyKind, PolicyRef, ProviderRouteSnapshot, RuntimePackage,
    RuntimePackageCanonicalV1, RuntimePackageId, SourceKind, SourceRef, testing::read_fixture,
};
use serde_json::Value;

const MANIFEST_PATH: &str = "tests/fixtures/golden/manifest.json";

#[test]
fn manifest_enumerates_current_event_kind_surface() {
    let manifest = manifest();
    let source_kinds = enum_variants(include_str!("../../src/records/event.rs"), "EventKind");
    let manifest_kinds = status_kinds(&manifest["event_kinds"]);

    assert_eq!(
        manifest_kinds, source_kinds,
        "EventKind changed; update tests/fixtures/golden/manifest.json with covered/reserved/unimplemented status"
    );

    for entry in covered_entries(&manifest["event_kinds"]) {
        assert_nonempty_fixture_set(entry, "event covered entry");
        assert_nonempty_array(entry, "redaction_cases");
    }
}

#[test]
fn emitted_event_kinds_are_covered_not_unimplemented() {
    let manifest = manifest();
    let covered = covered_kind_set(&manifest["event_kinds"]);
    let unimplemented = status_array_kind_set(&manifest["event_kinds"]["unimplemented"]);
    let emitted = emitted_event_kinds_from_current_sources();

    let missing_coverage = emitted
        .difference(&covered)
        .cloned()
        .collect::<BTreeSet<_>>();
    let misclassified = emitted
        .intersection(&unimplemented)
        .cloned()
        .collect::<BTreeSet<_>>();

    assert!(
        missing_coverage.is_empty(),
        "emitted event kinds need golden fixture coverage: {missing_coverage:?}"
    );
    assert!(
        misclassified.is_empty(),
        "emitted event kinds cannot remain unimplemented in the manifest: {misclassified:?}"
    );
}

#[test]
fn manifest_enumerates_current_journal_record_kind_surface() {
    let manifest = manifest();
    let source_kinds = enum_variants(
        include_str!("../../src/records/journal.rs"),
        "JournalRecordKind",
    );
    let manifest_kinds = status_kinds(&manifest["journal_record_kinds"]);

    assert_eq!(
        manifest_kinds, source_kinds,
        "JournalRecordKind changed; update tests/fixtures/golden/manifest.json with covered/reserved/unimplemented status"
    );

    for entry in covered_entries(&manifest["journal_record_kinds"]) {
        assert_nonempty_fixture_set(entry, "journal covered entry");
        assert_nonempty_array(entry, "redaction_cases");
    }
}

#[test]
fn manifest_fixture_paths_load_and_core_schemas_validate() {
    let manifest = manifest();
    for path in fixture_paths(&manifest) {
        let value = read_fixture(&path).unwrap_or_else(|error| {
            panic!("fixture path from manifest must load: {path}: {error:?}")
        });
        assert_fixture_has_schema_or_shape(&path, &value);
    }

    let snapshot =
        read_fixture("tests/fixtures/package/runtime-package-canonical-v1.json").unwrap();
    serde_json::from_value::<RuntimePackageCanonicalV1>(snapshot)
        .expect("runtime package snapshot schema still deserializes");

    let delta_value =
        read_fixture("tests/fixtures/golden/runtime-package-delta-activation-v1.json").unwrap();
    let delta =
        serde_json::from_value::<PackageDelta>(delta_value).expect("package delta schema parses");
    let package = package_with_tool();
    assert_eq!(
        delta.previous_fingerprint,
        package.fingerprint().expect("fingerprint")
    );
    let next = package.apply_delta(delta).expect("golden delta applies");
    assert_eq!(next.capabilities.len(), 2);
    assert_eq!(next.catalogs.len(), 1);
}

#[test]
fn covered_event_fixtures_contain_declared_event_kind() {
    let manifest = manifest();

    for entry in covered_entries(&manifest["event_kinds"]) {
        let kind = entry["kind"].as_str().expect("covered event kind string");
        let mut found = false;
        for path in strings_from_array(&entry["fixtures"]) {
            let value = read_fixture(&path).expect("event fixture loads");
            found |= json_contains_string(&value, kind);
            found |= json_contains_string(&value, &snake_to_pascal(kind));
        }
        assert!(
            found,
            "covered event kind {kind} must appear in at least one declared fixture"
        );
    }
}

#[test]
fn redaction_cases_do_not_capture_raw_content_by_default() {
    let manifest = manifest();
    let mut checked = BTreeSet::new();

    for case in manifest["redaction_cases"]
        .as_array()
        .expect("redaction_cases array")
    {
        for path in strings_from_array(&case["paths"]) {
            if !checked.insert(path.clone()) {
                continue;
            }
            let value = read_fixture(&path).expect("redaction case fixture loads");
            assert_no_forbidden_raw_markers(&path, &value);
            assert!(
                has_redaction_evidence(&value),
                "redaction case {path} must include privacy, content-capture, refs-only, raw-content-false, or redacted-summary evidence"
            );
        }
    }
}

#[test]
fn otel_golden_projections_keep_otel_as_derived_projection() {
    let manifest = manifest();

    for entry in manifest["otel_projections"]
        .as_array()
        .expect("otel projections array")
    {
        let path = entry["fixture"].as_str().expect("otel fixture path");
        let value = read_fixture(path).expect("otel fixture loads");
        assert_eq!(
            value["schema_url"],
            "https://opentelemetry.io/schemas/1.41.0"
        );
        assert_eq!(value["stability_opt_in"], "gen_ai_latest_experimental");
        assert_eq!(value["raw_content_attributes_present"], false);

        let attributes = collect_attribute_keys(&value);
        assert!(!attributes.is_empty(), "otel fixture {path} has attributes");
        for key in &attributes {
            assert!(
                key.starts_with("agent_sdk.")
                    || key.starts_with("gen_ai.")
                    || key.starts_with("mcp.")
                    || key.starts_with("jsonrpc.")
                    || key.starts_with("network.")
                    || key == "error.type",
                "unexpected OTel attribute namespace {key} in {path}"
            );
        }
        for forbidden in [
            "gen_ai.system_instructions",
            "gen_ai.tool.call.arguments",
            "gen_ai.tool.call.result",
        ] {
            assert!(
                !attributes.contains(forbidden),
                "{path} must not export opt-in raw content attribute {forbidden}"
            );
        }
    }
}

fn manifest() -> Value {
    read_fixture(MANIFEST_PATH).expect("golden manifest loads")
}

fn status_kinds(section: &Value) -> BTreeSet<String> {
    let mut kinds = BTreeSet::new();
    for entry in covered_entries(section) {
        kinds.insert(
            entry["kind"]
                .as_str()
                .expect("covered entry kind")
                .to_string(),
        );
    }
    for status in ["reserved", "unimplemented"] {
        for value in section[status]
            .as_array()
            .unwrap_or_else(|| panic!("{status} status array"))
        {
            let kind = value
                .as_str()
                .or_else(|| value["kind"].as_str())
                .unwrap_or_else(|| panic!("{status} status entries must name kind"));
            assert!(
                kinds.insert(kind.to_string()),
                "duplicate manifest status for {kind}"
            );
        }
    }
    kinds
}

fn covered_kind_set(section: &Value) -> BTreeSet<String> {
    covered_entries(section)
        .iter()
        .map(|entry| {
            entry["kind"]
                .as_str()
                .expect("covered entry kind")
                .to_string()
        })
        .collect()
}

fn status_array_kind_set(section: &Value) -> BTreeSet<String> {
    section
        .as_array()
        .expect("status array")
        .iter()
        .map(|value| {
            value
                .as_str()
                .or_else(|| value["kind"].as_str())
                .expect("status value names kind")
                .to_string()
        })
        .collect()
}

fn covered_entries(section: &Value) -> &[Value] {
    section["covered"]
        .as_array()
        .expect("covered status array")
        .as_slice()
}

fn assert_nonempty_fixture_set(entry: &Value, label: &str) {
    assert_nonempty_array(entry, "fixtures");
    for path in strings_from_array(&entry["fixtures"]) {
        assert!(
            path.starts_with("tests/fixtures/"),
            "{label} fixture path must stay under tests/fixtures: {path}"
        );
    }
}

fn assert_nonempty_array(entry: &Value, field: &str) {
    assert!(
        entry[field]
            .as_array()
            .is_some_and(|items| !items.is_empty()),
        "manifest entry {entry:?} must include non-empty {field}"
    );
}

fn fixture_paths(value: &Value) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    collect_fixture_paths(value, &mut paths);
    paths
}

fn collect_fixture_paths(value: &Value, paths: &mut BTreeSet<String>) {
    match value {
        Value::String(path) if path.starts_with("tests/fixtures/") => {
            paths.insert(path.clone());
        }
        Value::Array(items) => {
            for item in items {
                collect_fixture_paths(item, paths);
            }
        }
        Value::Object(fields) => {
            for value in fields.values() {
                collect_fixture_paths(value, paths);
            }
        }
        _ => {}
    }
}

fn emitted_event_kinds_from_current_sources() -> BTreeSet<String> {
    [
        include_str!("../../src/application/loop_driver.rs"),
        include_str!("../../src/application/agent_pool.rs"),
        include_str!("../../src/ports/subscription.rs"),
    ]
    .into_iter()
    .flat_map(event_kind_refs)
    .collect()
}

fn event_kind_refs(source: &str) -> BTreeSet<String> {
    let mut kinds = BTreeSet::new();
    let mut remaining = source;
    while let Some(index) = remaining.find("EventKind::") {
        let after = &remaining[index + "EventKind::".len()..];
        let variant = after
            .chars()
            .take_while(|ch| ch.is_ascii_alphanumeric())
            .collect::<String>();
        if !variant.is_empty() {
            kinds.insert(rust_variant_to_snake(&variant));
        }
        remaining = &after[variant.len()..];
    }
    kinds
}

fn assert_fixture_has_schema_or_shape(path: &str, value: &Value) {
    assert!(
        value.is_object(),
        "golden fixture {path} must be a JSON object for reviewable schema drift"
    );

    let has_known_shape = value.get("schema_version").is_some()
        || value.get("journal_schema_version").is_some()
        || value.get("record_schema_version").is_some()
        || value.get("kind").is_some()
        || value.get("type").is_some()
        || value.get("effect_kind").is_some()
        || value.get("records").is_some()
        || value.get("events").is_some()
        || value.get("spans").is_some()
        || value.get("schema_url").is_some()
        || value.get("payload").is_some()
        || value.get("activated_capabilities").is_some()
        || value.get("fingerprint").is_some()
        || value.get("allowed").is_some()
        || value.get("routes").is_some()
        || value.get("frames").is_some()
        || value.get("interventions").is_some()
        || value.get("budget").is_some()
        || value.get("messages").is_some()
        || value.get("output_id").is_some()
        || value.get("delivery_id").is_some()
        || value.get("journal").is_some()
        || value.get("subagent_records").is_some()
        || value.get("lifecycle").is_some();
    assert!(
        has_known_shape,
        "fixture {path} has no recognized golden shape"
    );
}

fn enum_variants(source: &str, enum_name: &str) -> BTreeSet<String> {
    let marker = format!("pub enum {enum_name} {{");
    let start = source.find(&marker).expect("enum marker exists") + marker.len();
    let body = source[start..].split_once('}').expect("enum closes").0;

    body.lines()
        .filter_map(|line| {
            let line = line.split("//").next().unwrap_or("").trim();
            if line.is_empty() || line.starts_with("#[") {
                return None;
            }
            let variant = line.trim_end_matches(',').trim();
            if variant.is_empty() {
                None
            } else {
                Some(rust_variant_to_snake(variant))
            }
        })
        .collect()
}

fn rust_variant_to_snake(variant: &str) -> String {
    let mut out = String::new();
    for (index, ch) in variant.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if index > 0 {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

fn snake_to_pascal(kind: &str) -> String {
    kind.split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let mut word = String::new();
                    word.push(first.to_ascii_uppercase());
                    word.extend(chars);
                    word
                }
                None => String::new(),
            }
        })
        .collect::<String>()
}

fn strings_from_array(value: &Value) -> Vec<String> {
    value
        .as_array()
        .expect("array of strings")
        .iter()
        .map(|value| value.as_str().expect("string value").to_string())
        .collect()
}

fn json_contains_string(value: &Value, needle: &str) -> bool {
    match value {
        Value::String(candidate) => candidate == needle,
        Value::Array(items) => items.iter().any(|item| json_contains_string(item, needle)),
        Value::Object(fields) => fields
            .values()
            .any(|value| json_contains_string(value, needle)),
        _ => false,
    }
}

fn assert_no_forbidden_raw_markers(path: &str, value: &Value) {
    let rendered = serde_json::to_string(value)
        .expect("redaction fixture renders")
        .to_ascii_lowercase();
    for forbidden in [
        "do not export this raw text",
        "unredacted secret",
        "api_key=",
        "password=",
        "authorization:",
        "bearer ",
        "private_key",
    ] {
        assert!(
            !rendered.contains(forbidden),
            "redaction fixture {path} contains forbidden raw marker {forbidden}"
        );
    }
}

fn has_redaction_evidence(value: &Value) -> bool {
    match value {
        Value::String(text) => {
            matches!(
                text.as_str(),
                "content_refs_only" | "off" | "redacted_summary" | "redacted"
            ) || text.contains("redacted")
        }
        Value::Bool(false) => true,
        Value::Array(items) => items.iter().any(has_redaction_evidence),
        Value::Object(fields) => fields.iter().any(|(key, value)| {
            key.contains("redacted")
                || key.contains("privacy")
                || key.contains("content_ref")
                || key.contains("content_capture")
                || key.contains("delivery_semantics")
                || key.contains("journal_cursor")
                || key.contains("raw_content")
                || key.contains("raw_match")
                || has_redaction_evidence(value)
        }),
        _ => false,
    }
}

fn collect_attribute_keys(value: &Value) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    collect_attribute_keys_inner(value, &mut keys);
    keys
}

fn collect_attribute_keys_inner(value: &Value, keys: &mut BTreeSet<String>) {
    match value {
        Value::Object(fields) => {
            if let Some(attributes) = fields.get("attributes").and_then(Value::as_object) {
                keys.extend(attributes.keys().cloned());
            }
            for value in fields.values() {
                collect_attribute_keys_inner(value, keys);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_attribute_keys_inner(item, keys);
            }
        }
        _ => {}
    }
}

fn package_with_tool() -> RuntimePackage {
    RuntimePackage::builder(RuntimePackageId::new("package.contract"))
        .agent(AgentSnapshot {
            agent_id: AgentId::new("agent.contract"),
            name: "contract agent".to_string(),
            default_behavior_refs: vec![package_policy("policy.agent.default")],
        })
        .provider_route(ProviderRouteSnapshot::new("provider.fake", "model.fake.p0"))
        .sidecar(PackageSidecarSnapshot {
            sidecar_id: "sidecar.schema.workspace_read".to_string(),
            kind: "tool_schema".to_string(),
            version: "v1".to_string(),
            refs: vec![schema_ref("v1")],
            policy_refs: vec![approval_policy("policy.approval.workspace_read")],
            content_hash: "sha256:schema.v1".to_string(),
            redacted_payload: None,
        })
        .capability(workspace_read_tool("v1", "executor.workspace_read.v1"))
        .policy(package_policy("policy.package.default"))
        .build()
        .expect("package builds")
}

fn workspace_read_tool(schema_version: &str, executor_ref: &str) -> CapabilitySpec {
    CapabilitySpec::fake_tool(
        "cap.workspace_read",
        "workspace_read",
        schema_ref(schema_version),
        ExecutorRef::new(executor_ref),
        approval_policy("policy.approval.workspace_read"),
        source("source.sdk.toolpack"),
    )
}

fn schema_ref(version: &str) -> PackageSidecarRef {
    let mut sidecar =
        PackageSidecarRef::new("sidecar.schema.workspace_read", "tool_schema", version);
    sidecar.content_hash = Some(format!("sha256:schema.{version}"));
    sidecar
}

fn source(id: &str) -> SourceRef {
    SourceRef::with_kind(SourceKind::Sdk, id)
}

fn package_policy(id: &str) -> PolicyRef {
    PolicyRef::with_kind(PolicyKind::RuntimePackage, id)
}

fn approval_policy(id: &str) -> PolicyRef {
    PolicyRef::with_kind(PolicyKind::Approval, id)
}
