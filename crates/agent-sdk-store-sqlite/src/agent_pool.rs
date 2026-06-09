use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use agent_sdk_core::{
    AgentError, AgentErrorKind, AgentPoolId, AgentPoolMember, AgentPoolSnapshot, AgentPoolStore,
    AgentPoolStoreConfig, AgentPoolStoreCursor, AgentPoolStoreRecord, AgentPoolStoreRecordPayload,
    AgentPoolStoreStream, AgentPoolStoredMessage, AgentPoolStoredWake, CompiledEventFilter,
    IdempotencyKey, MessageId, MessageReceipt, RetryClassification, RunId, RunMessage, TopicId,
    WakeCondition, WakeConditionId, WakeRegistration,
};
use rusqlite::{Connection, params};

/// SQLite-backed implementation of `AgentPoolStore`.
///
/// Two independent `SqliteAgentPoolStore` values opened against the same
/// database file share pool membership, messages, wake registrations, dedupe,
/// rehydration, and watch cursors. The adapter stores core pool records as JSON
/// and replays them into snapshots; it does not own workflow scheduling or a
/// second event bus.
#[derive(Clone)]
pub struct SqliteAgentPoolStore {
    path: PathBuf,
    connection: Arc<Mutex<Connection>>,
}

impl SqliteAgentPoolStore {
    /// Opens or creates a SQLite-backed agent-pool store.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, AgentError> {
        let path = path.as_ref().to_path_buf();
        let connection = Connection::open(&path).map_err(sqlite_error)?;
        connection
            .execute_batch(
                "PRAGMA journal_mode = WAL;
                 CREATE TABLE IF NOT EXISTS agent_pool_records (
                     pool_id TEXT NOT NULL,
                     seq INTEGER NOT NULL,
                     kind TEXT NOT NULL,
                     payload_json TEXT NOT NULL,
                     PRIMARY KEY (pool_id, seq)
                 );
                 CREATE TABLE IF NOT EXISTS agent_pool_event_seq (
                     pool_id TEXT PRIMARY KEY,
                     seq INTEGER NOT NULL
                 );",
            )
            .map_err(sqlite_error)?;
        Ok(Self {
            path,
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    /// Returns the backing database path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn append_record(
        &self,
        pool_id: &AgentPoolId,
        payload: AgentPoolStoreRecordPayload,
    ) -> Result<AgentPoolStoreCursor, AgentError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(sqlite_error)?;
        let next_seq = transaction
            .query_row(
                "SELECT COALESCE(MAX(seq), 0) + 1 FROM agent_pool_records WHERE pool_id = ?1",
                params![pool_id.as_str()],
                |row| row.get::<_, i64>(0),
            )
            .map_err(sqlite_error)?;
        let payload_json = serde_json::to_string(&payload).map_err(serde_error)?;
        transaction
            .execute(
                "INSERT INTO agent_pool_records (pool_id, seq, kind, payload_json)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    pool_id.as_str(),
                    next_seq,
                    payload_kind(&payload),
                    payload_json
                ],
            )
            .map_err(sqlite_error)?;
        transaction.commit().map_err(sqlite_error)?;
        Ok(AgentPoolStoreCursor::new(next_seq as u64))
    }

    fn records_after(
        &self,
        pool_id: &AgentPoolId,
        cursor: Option<AgentPoolStoreCursor>,
    ) -> Result<Vec<AgentPoolStoreRecord>, AgentError> {
        let start_after = cursor.map(|cursor| cursor.sequence).unwrap_or(0);
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT seq, payload_json
                 FROM agent_pool_records
                 WHERE pool_id = ?1 AND seq > ?2
                 ORDER BY seq ASC",
            )
            .map_err(sqlite_error)?;
        let rows = statement
            .query_map(params![pool_id.as_str(), start_after as i64], |row| {
                let seq: i64 = row.get(0)?;
                let payload_json: String = row.get(1)?;
                Ok((seq, payload_json))
            })
            .map_err(sqlite_error)?;

        let mut records = Vec::new();
        for row in rows {
            let (seq, payload_json) = row.map_err(sqlite_error)?;
            let payload = serde_json::from_str::<AgentPoolStoreRecordPayload>(&payload_json)
                .map_err(serde_error)?;
            records.push(AgentPoolStoreRecord {
                pool_id: pool_id.clone(),
                cursor: AgentPoolStoreCursor::new(seq as u64),
                payload,
            });
        }
        Ok(records)
    }

    fn replay(&self, pool_id: &AgentPoolId) -> Result<PoolReplay, AgentError> {
        let mut replay = PoolReplay::default();
        for record in self.records_after(pool_id, Some(AgentPoolStoreCursor::start()))? {
            replay.cursor = Some(record.cursor.clone());
            replay.apply(record.payload)?;
        }
        Ok(replay)
    }

    fn connection(&self) -> Result<std::sync::MutexGuard<'_, Connection>, AgentError> {
        self.connection
            .lock()
            .map_err(|_| AgentError::contract_violation("sqlite agent pool store lock poisoned"))
    }
}

impl AgentPoolStore for SqliteAgentPoolStore {
    fn open_pool(
        &self,
        pool_id: AgentPoolId,
        config: AgentPoolStoreConfig,
    ) -> Result<AgentPoolSnapshot, AgentError> {
        let replay = self.replay(&pool_id)?;
        if let Some(existing) = replay.config.as_ref() {
            if existing != &config {
                return Err(AgentError::new(
                    AgentErrorKind::InvalidStateTransition,
                    RetryClassification::RepairNeeded,
                    "sqlite agent pool store config conflicts with existing pool",
                ));
            }
        } else {
            self.append_record(&pool_id, AgentPoolStoreRecordPayload::PoolOpened { config })?;
        }
        self.snapshot(&pool_id)
    }

    fn snapshot(&self, pool_id: &AgentPoolId) -> Result<AgentPoolSnapshot, AgentError> {
        self.replay(pool_id)?.snapshot(pool_id.clone())
    }

    fn record_pool_created(
        &self,
        pool_id: &AgentPoolId,
    ) -> Result<AgentPoolStoreCursor, AgentError> {
        self.append_record(pool_id, AgentPoolStoreRecordPayload::PoolCreated)
    }

    fn join_member(
        &self,
        pool_id: &AgentPoolId,
        member: AgentPoolMember,
    ) -> Result<AgentPoolStoreCursor, AgentError> {
        self.snapshot(pool_id)?;
        self.append_record(
            pool_id,
            AgentPoolStoreRecordPayload::MemberJoined { member },
        )
    }

    fn leave_member(
        &self,
        pool_id: &AgentPoolId,
        run_id: &RunId,
    ) -> Result<(AgentPoolMember, AgentPoolStoreCursor), AgentError> {
        let replay = self.replay(pool_id)?;
        let member = replay.members.get(run_id).cloned().ok_or_else(|| {
            AgentError::new(
                AgentErrorKind::InvalidStateTransition,
                RetryClassification::NotRetryable,
                "run is not a member of this agent pool",
            )
        })?;
        let cursor = self.append_record(
            pool_id,
            AgentPoolStoreRecordPayload::MemberLeft {
                member: member.clone(),
            },
        )?;
        Ok((member, cursor))
    }

    fn message_receipt(
        &self,
        pool_id: &AgentPoolId,
        idempotency_key: &IdempotencyKey,
    ) -> Result<Option<MessageReceipt>, AgentError> {
        Ok(self
            .replay(pool_id)?
            .message_dedupe
            .get(idempotency_key)
            .cloned())
    }

    fn record_message(
        &self,
        pool_id: &AgentPoolId,
        message: RunMessage,
        receipt: MessageReceipt,
    ) -> Result<AgentPoolStoreCursor, AgentError> {
        self.snapshot(pool_id)?;
        self.append_record(
            pool_id,
            AgentPoolStoreRecordPayload::RunMessage {
                stored: AgentPoolStoredMessage { message, receipt },
            },
        )
    }

    fn wake_registration(
        &self,
        pool_id: &AgentPoolId,
        idempotency_key: &IdempotencyKey,
    ) -> Result<Option<WakeRegistration>, AgentError> {
        Ok(self
            .replay(pool_id)?
            .wake_dedupe
            .get(idempotency_key)
            .cloned())
    }

    fn wake(
        &self,
        pool_id: &AgentPoolId,
        condition_id: &WakeConditionId,
    ) -> Result<Option<AgentPoolStoredWake>, AgentError> {
        Ok(self.replay(pool_id)?.wakes.get(condition_id).cloned())
    }

    fn record_wake(
        &self,
        pool_id: &AgentPoolId,
        condition: WakeCondition,
        compiled_filter: CompiledEventFilter,
        registration: WakeRegistration,
    ) -> Result<AgentPoolStoreCursor, AgentError> {
        self.snapshot(pool_id)?;
        self.append_record(
            pool_id,
            AgentPoolStoreRecordPayload::Wake {
                stored: AgentPoolStoredWake {
                    condition,
                    compiled_filter,
                    registration,
                },
            },
        )
    }

    fn watch(
        &self,
        pool_id: &AgentPoolId,
        cursor: Option<AgentPoolStoreCursor>,
    ) -> Result<AgentPoolStoreStream, AgentError> {
        Ok(AgentPoolStoreStream::new(VecDeque::from(
            self.records_after(pool_id, cursor)?,
        )))
    }

    fn next_event_sequence(&self, pool_id: &AgentPoolId) -> Result<u64, AgentError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(sqlite_error)?;
        transaction
            .execute(
                "INSERT OR IGNORE INTO agent_pool_event_seq (pool_id, seq) VALUES (?1, 0)",
                params![pool_id.as_str()],
            )
            .map_err(sqlite_error)?;
        transaction
            .execute(
                "UPDATE agent_pool_event_seq SET seq = seq + 1 WHERE pool_id = ?1",
                params![pool_id.as_str()],
            )
            .map_err(sqlite_error)?;
        let seq = transaction
            .query_row(
                "SELECT seq FROM agent_pool_event_seq WHERE pool_id = ?1",
                params![pool_id.as_str()],
                |row| row.get::<_, i64>(0),
            )
            .map_err(sqlite_error)?;
        transaction.commit().map_err(sqlite_error)?;
        Ok(seq as u64)
    }
}

#[derive(Default)]
struct PoolReplay {
    config: Option<AgentPoolStoreConfig>,
    created: bool,
    members: BTreeMap<RunId, AgentPoolMember>,
    messages: BTreeMap<MessageId, AgentPoolStoredMessage>,
    message_dedupe: BTreeMap<IdempotencyKey, MessageReceipt>,
    wakes: BTreeMap<WakeConditionId, AgentPoolStoredWake>,
    wake_dedupe: BTreeMap<IdempotencyKey, WakeRegistration>,
    cursor: Option<AgentPoolStoreCursor>,
}

impl PoolReplay {
    fn apply(&mut self, payload: AgentPoolStoreRecordPayload) -> Result<(), AgentError> {
        match payload {
            AgentPoolStoreRecordPayload::PoolOpened { config } => {
                if self
                    .config
                    .as_ref()
                    .is_some_and(|existing| existing != &config)
                {
                    return Err(AgentError::new(
                        AgentErrorKind::InvalidStateTransition,
                        RetryClassification::RepairNeeded,
                        "sqlite agent pool store contains conflicting pool open records",
                    ));
                }
                self.config = Some(config);
            }
            AgentPoolStoreRecordPayload::PoolCreated => {
                self.created = true;
            }
            AgentPoolStoreRecordPayload::MemberJoined { member } => {
                self.members.insert(member.run_id.clone(), member);
            }
            AgentPoolStoreRecordPayload::MemberLeft { member } => {
                self.members.remove(&member.run_id);
            }
            AgentPoolStoreRecordPayload::RunMessage { stored } => {
                self.message_dedupe.insert(
                    stored.message.idempotency_key.clone(),
                    stored.receipt.clone(),
                );
                self.messages
                    .insert(stored.message.message_id.clone(), stored);
            }
            AgentPoolStoreRecordPayload::Wake { stored } => {
                self.wake_dedupe.insert(
                    stored.condition.idempotency_key.clone(),
                    stored.registration.clone(),
                );
                self.wakes
                    .insert(stored.condition.condition_id.clone(), stored);
            }
        }
        Ok(())
    }

    fn snapshot(self, pool_id: AgentPoolId) -> Result<AgentPoolSnapshot, AgentError> {
        let config = self.config.ok_or_else(|| {
            AgentError::new(
                AgentErrorKind::HostConfigurationNeeded,
                RetryClassification::HostConfigurationNeeded,
                "sqlite agent pool store has not opened this pool",
            )
        })?;
        let topics = topics_from_members(self.members.values());
        Ok(AgentPoolSnapshot {
            pool_id,
            created: self.created,
            members: self.members.into_values().collect(),
            topics,
            message_policy: config.message_policy,
            wake_policy: config.wake_policy,
            policy_refs: config.policy_refs,
            messages: self.messages.into_values().collect(),
            wakes: self.wakes.into_values().collect(),
            cursor: self.cursor,
        })
    }
}

fn topics_from_members<'a>(members: impl IntoIterator<Item = &'a AgentPoolMember>) -> Vec<TopicId> {
    let mut topics = BTreeSet::new();
    for member in members {
        topics.extend(member.topics.iter().cloned());
    }
    topics.into_iter().collect()
}

fn payload_kind(payload: &AgentPoolStoreRecordPayload) -> &'static str {
    match payload {
        AgentPoolStoreRecordPayload::PoolOpened { .. } => "pool_opened",
        AgentPoolStoreRecordPayload::PoolCreated => "pool_created",
        AgentPoolStoreRecordPayload::MemberJoined { .. } => "member_joined",
        AgentPoolStoreRecordPayload::MemberLeft { .. } => "member_left",
        AgentPoolStoreRecordPayload::RunMessage { .. } => "run_message",
        AgentPoolStoreRecordPayload::Wake { .. } => "wake",
    }
}

fn sqlite_error(error: rusqlite::Error) -> AgentError {
    AgentError::new(
        AgentErrorKind::InvalidStateTransition,
        RetryClassification::RepairNeeded,
        format!("sqlite agent pool store failure: {error}"),
    )
}

fn serde_error(error: serde_json::Error) -> AgentError {
    AgentError::new(
        AgentErrorKind::InvalidStateTransition,
        RetryClassification::RepairNeeded,
        format!("sqlite agent pool store serialization failure: {error}"),
    )
}
