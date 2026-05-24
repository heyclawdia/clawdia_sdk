use agent_sdk_core::{
    AdapterRef, AgentId, CorrelationEntry, CorrelationKey, CorrelationValue, DestinationKind,
    DestinationRef, EntityKind, EntityRef, EventCursorId, EventId, JournalCursor, PolicyKind,
    PolicyRef, PrivacyClass, RetentionClass, RunId, SourceKind, SourceRef, TraceId, TrustClass,
};
use serde_json::json;

fn fixture(path: &str) -> serde_json::Value {
    serde_json::from_str(path).expect("fixture parses")
}

#[test]
fn typed_ids_serialize_as_stable_strings_and_redact_logs() {
    let value = json!({
        "agent_id": AgentId::new("agent.contract"),
        "run_id": RunId::new("run.contract"),
        "event_id": EventId::new("event.contract"),
        "trace_id": TraceId::new("trace.contract"),
        "event_cursor": EventCursorId::new("event.cursor.contract"),
        "journal_cursor": JournalCursor::new("journal.cursor.contract"),
    });

    let expected = fixture(include_str!("../fixtures/ids/typed-ids.json"));
    assert_eq!(value, expected);

    assert_eq!(
        format!("{:?}", AgentId::new("agent.contract")),
        "AgentId(redacted)"
    );
    assert_eq!(format!("{}", RunId::new("run.contract")), "RunId(redacted)");
    assert_eq!(
        format!("{:?}", EventCursorId::new("event.cursor.contract")),
        "EventCursorId(redacted)"
    );
}

#[test]
fn refs_serialize_with_product_neutral_lineage_fields() {
    let mut source = SourceRef::with_kind(SourceKind::RemoteChannel, "source.remote.1");
    source.trust = TrustClass::UserProvided;
    source.privacy = PrivacyClass::ContentRefsOnly;
    source.correlation.push(CorrelationEntry {
        key: CorrelationKey::new("thread"),
        value: CorrelationValue::new("thread.redacted"),
    });
    source.redacted_summary = Some("remote user request".to_string());

    let mut destination =
        DestinationRef::with_kind(DestinationKind::Provider, "destination.provider.1");
    destination.privacy = PrivacyClass::ContentRefsOnly;
    destination.retention = RetentionClass::RunScoped;
    destination.redacted_summary = Some("provider projection".to_string());

    let mut entity = EntityRef::run(RunId::new("run.contract"));
    entity.source = Some(source.clone());
    entity.redacted_summary = Some("run subject".to_string());

    let mut policy = PolicyRef::with_kind(PolicyKind::Privacy, "policy.privacy.default");
    policy.version = Some("v1".to_string());

    let value = json!({
        "source": source,
        "destination": destination,
        "entity": entity,
        "policy": policy,
    });

    let expected = fixture(include_str!("../fixtures/ids/refs.json"));
    assert_eq!(value, expected);
}

#[test]
fn privacy_retention_and_trust_classes_use_snake_case_wire_names() {
    let value = json!({
        "privacy": [
            PrivacyClass::Public,
            PrivacyClass::Internal,
            PrivacyClass::ContentRefsOnly,
            PrivacyClass::Sensitive,
            PrivacyClass::Secret,
        ],
        "retention": [
            RetentionClass::Ephemeral,
            RetentionClass::RunScoped,
            RetentionClass::SessionScoped,
            RetentionClass::Durable,
            RetentionClass::Persistent,
            RetentionClass::HostPolicy,
        ],
        "trust": [
            TrustClass::Trusted,
            TrustClass::SdkGenerated,
            TrustClass::HostProvided,
            TrustClass::UserProvided,
            TrustClass::External,
            TrustClass::Untrusted,
        ],
    });

    let expected = fixture(include_str!("../fixtures/ids/privacy-retention-trust.json"));
    assert_eq!(value, expected);
    assert!(!PrivacyClass::ContentRefsOnly.allows_raw_content_by_default());
}

#[test]
fn ref_debug_and_display_do_not_expose_raw_ids() {
    let source = SourceRef::with_kind(SourceKind::Extension, "source.secret.extension");
    let destination =
        DestinationRef::with_kind(DestinationKind::OutputSink, "destination.secret.sink");
    let policy = PolicyRef::with_kind(PolicyKind::Redaction, "policy.secret.redaction");
    let entity = EntityRef::new(EntityKind::ExtensionAction, "extension.action.secret");

    let rendered = format!("{source:?}\n{destination:?}\n{policy:?}\n{entity:?}");
    assert!(!rendered.contains("secret"));
    assert!(rendered.contains("redacted"));

    assert_eq!(source.to_string(), "Extension:redacted");
    assert_eq!(destination.to_string(), "OutputSink:redacted");
    assert_eq!(policy.to_string(), "Redaction:redacted");
    assert_eq!(entity.to_string(), "ExtensionAction:redacted");
}

#[test]
fn ids_and_refs_expose_reusable_validation_for_hostile_cases() {
    assert!(AgentId::try_new("agent.valid").is_ok());
    assert!(AgentId::try_new("").is_err());
    assert!(RunId::try_new("run.bad\nid").is_err());
    assert!(TraceId::try_new("x".repeat(agent_sdk_core::MAX_ID_LEN + 1)).is_err());
    assert!(EventCursorId::try_new("cursor.valid").is_ok());
    assert!(JournalCursor::try_new("journal.bad\tcursor").is_err());

    assert!(agent_sdk_core::refs::EntityId::try_new("entity.valid").is_ok());
    assert!(agent_sdk_core::refs::SourceId::try_new("").is_err());
    assert!(agent_sdk_core::refs::DestinationId::try_new("destination.bad\u{0007}").is_err());
    assert!(agent_sdk_core::refs::PolicyId::try_new("policy.valid").is_ok());
    assert!(AdapterRef::try_new("adapter.valid").is_ok());
}

#[test]
fn durable_id_deserialization_rejects_invalid_wire_values() {
    assert!(serde_json::from_str::<AgentId>("\"agent.valid\"").is_ok());
    assert!(serde_json::from_str::<AgentId>("\"\"").is_err());
    assert!(serde_json::from_str::<RunId>("\"run.bad\\nid\"").is_err());
    assert!(serde_json::from_str::<EventCursorId>("\"event.cursor\\tbad\"").is_err());
    assert!(serde_json::from_str::<agent_sdk_core::refs::SourceId>("\"\"").is_err());
    assert!(serde_json::from_str::<AdapterRef>("\"adapter.bad\\nbreak\"").is_err());
}

#[test]
fn public_new_constructors_enforce_identifier_validation() {
    assert!(std::panic::catch_unwind(|| AgentId::new("")).is_err());
    assert!(std::panic::catch_unwind(|| EventCursorId::new("bad\ncursor")).is_err());
    assert!(std::panic::catch_unwind(|| agent_sdk_core::refs::SourceId::new("")).is_err());
    assert!(std::panic::catch_unwind(|| AdapterRef::new("bad\tadapter")).is_err());
}
