use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use agent_sdk_core::{
    AgentId, AgentPool, AgentPoolId, AgentPoolMember, AgentPoolStoreRecordPayload, IdempotencyKey,
    InMemoryAgentEventBus, MessageId, MessageStatus, RunAddress, RunId, RunMessage, WakeCondition,
    WakeConditionId, WakeRegistrationStatus,
    domain::ContentRef as ContentRefId,
    event::{EventFamily, EventFilter, EventFilterSet, EventKind},
    testing::FakeJournalStore,
};
use agent_sdk_toolkit::SqliteAgentPoolStore;

#[test]
fn sqlite_agent_pool_store_coordinates_independent_handles() {
    let db_path = temp_db("agent-pool-store");
    let store_a = SqliteAgentPoolStore::open(&db_path).expect("store a opens");
    let store_b = SqliteAgentPoolStore::open(&db_path).expect("store b opens");
    let pool_a = pool_with_sqlite_store("pool.sqlite.shared", store_a);
    let pool_b = pool_with_sqlite_store("pool.sqlite.shared", store_b);

    pool_a
        .join_run(AgentPoolMember::new(
            RunId::new("run.sqlite.a"),
            AgentId::new("agent.sqlite.a"),
        ))
        .expect("run a joins");
    assert_eq!(pool_b.members().unwrap().len(), 1);

    pool_b
        .join_run(AgentPoolMember::new(
            RunId::new("run.sqlite.b"),
            AgentId::new("agent.sqlite.b"),
        ))
        .expect("run b joins");
    assert_eq!(pool_a.members().unwrap().len(), 2);

    let condition = WakeCondition::new(
        WakeConditionId::new("wake.sqlite.message"),
        RunId::new("run.sqlite.b"),
        EventFilter {
            run_ids: EventFilterSet::Include(vec![RunId::new("run.sqlite.a")]),
            families: EventFilterSet::Include(vec![EventFamily::AgentPool]),
            kinds: EventFilterSet::Include(vec![EventKind::RunMessageDelivered]),
            ..EventFilter::default()
        },
        IdempotencyKey::new("idem.wake.sqlite.message"),
    );
    assert_eq!(
        pool_b
            .suspend_until(RunId::new("run.sqlite.b"), condition)
            .expect("wake registered")
            .status,
        WakeRegistrationStatus::Registered
    );

    let message = run_message(
        "message.sqlite.shared",
        "run.sqlite.a",
        RunAddress::run(RunId::new("run.sqlite.b")),
    );
    let receipt = pool_a.send(message.clone()).expect("message delivered");
    assert_eq!(receipt.status, MessageStatus::Delivered);
    assert_eq!(receipt.delivered_to, vec![RunId::new("run.sqlite.b")]);
    assert_eq!(
        pool_b.send(message).expect("deduped receipt"),
        receipt,
        "idempotency must dedupe across independent SQLite handles"
    );
    assert_eq!(
        pool_b
            .poll_wake(&WakeConditionId::new("wake.sqlite.message"))
            .expect("wake state visible")
            .status,
        WakeRegistrationStatus::Triggered
    );

    drop(pool_a);
    drop(pool_b);

    let rehydrated_store = SqliteAgentPoolStore::open(&db_path).expect("store reopens");
    let rehydrated = pool_with_sqlite_store("pool.sqlite.shared", rehydrated_store);
    let snapshot = rehydrated.snapshot().expect("snapshot rehydrates");
    assert_eq!(snapshot.members.len(), 2);
    assert_eq!(snapshot.messages.len(), 1);
    assert_eq!(
        snapshot.messages[0].receipt.status,
        MessageStatus::Delivered
    );
    assert_eq!(snapshot.wakes.len(), 1);
    assert_eq!(
        snapshot.wakes[0].registration.status,
        WakeRegistrationStatus::Triggered
    );
    assert!(
        rehydrated
            .watch_pool(None)
            .expect("watch records")
            .any(|record| matches!(
                record.payload,
                AgentPoolStoreRecordPayload::RunMessage { stored }
                    if stored.receipt.status == MessageStatus::Delivered
            ))
    );
}

fn pool_with_sqlite_store(pool_id: &str, store: SqliteAgentPoolStore) -> AgentPool {
    AgentPool::builder(AgentPoolId::new(pool_id))
        .runtime(
            agent_sdk_core::AgentRuntime::builder()
                .journal(FakeJournalStore::default())
                .event_bus(InMemoryAgentEventBus::default())
                .build()
                .expect("runtime builds"),
        )
        .store(store)
        .build()
        .expect("pool builds")
}

fn run_message(message_id: &str, from: &str, to: RunAddress) -> RunMessage {
    RunMessage::new(
        MessageId::new(message_id),
        RunId::new(from),
        to,
        ContentRefId::new(format!("content.{message_id}")),
        IdempotencyKey::new(format!("idem.{message_id}")),
    )
}

fn temp_db(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("agent-sdk-toolkit-{label}-{nonce}"));
    fs::create_dir_all(&root).unwrap();
    root.join("agent-pool.sqlite")
}
