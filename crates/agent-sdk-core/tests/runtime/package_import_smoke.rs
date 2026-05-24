use agent_sdk_core::{
    Agent, AgentId, ContextProjection, EventFilter, RuntimePackage,
    testing::{FakeFixtureHarness, FakeJournalStore},
};

#[test]
fn core_package_imports_without_optional_crates() {
    let agent = Agent::builder()
        .id(AgentId::new("agent.smoke"))
        .name("smoke")
        .build()
        .expect("agent builds");

    assert_eq!(agent.name(), "smoke");
    assert_eq!(ContextProjection::default().items.len(), 0);
    assert!(EventFilter::default().families.is_any());
    assert_eq!(FakeFixtureHarness::default().deterministic_seed, 0);
    assert!(FakeJournalStore::default().records().is_empty());
    let _package_type_name = std::any::type_name::<RuntimePackage>();
}
