use std::{collections::BTreeMap, path::PathBuf};

use agent_sdk_core::{
    AgentError, AgentPoolId, AgentPoolMember, AgentPoolSnapshot, AgentPoolStore,
    AgentPoolStoreConfig, AgentPoolStoreCursor, AgentPoolStoreRecord, AgentPoolStoreRecordPayload,
    AgentPoolStoreStream, AgentPoolStoredMessage, AgentPoolStoredWake, IdempotencyKey,
    MessageReceipt, RunId, RunMessage, WakeCondition, WakeConditionId, WakeRegistration,
    agent_pool::AgentPoolStoredWake as CoreStoredWake, event::CompiledEventFilter,
};
use serde::{Deserialize, Serialize};

use crate::util::{read_json, root_join, safe_segment, write_json};

#[derive(Clone, Debug)]
/// Filesystem-backed agent-pool coordination store.
pub struct FileAgentPoolStore {
    root: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PoolState {
    config: AgentPoolStoreConfig,
    created: bool,
    members: BTreeMap<RunId, AgentPoolMember>,
    messages: BTreeMap<String, AgentPoolStoredMessage>,
    message_dedupe: BTreeMap<IdempotencyKey, MessageReceipt>,
    wakes: BTreeMap<WakeConditionId, AgentPoolStoredWake>,
    wake_dedupe: BTreeMap<IdempotencyKey, WakeRegistration>,
    records: Vec<AgentPoolStoreRecord>,
    next_event_counter: u64,
}

impl PoolState {
    fn new(config: AgentPoolStoreConfig) -> Self {
        Self {
            config,
            created: false,
            members: BTreeMap::new(),
            messages: BTreeMap::new(),
            message_dedupe: BTreeMap::new(),
            wakes: BTreeMap::new(),
            wake_dedupe: BTreeMap::new(),
            records: Vec::new(),
            next_event_counter: 0,
        }
    }

    fn snapshot(&self, pool_id: AgentPoolId) -> AgentPoolSnapshot {
        AgentPoolSnapshot {
            pool_id,
            created: self.created,
            topics: self
                .members
                .values()
                .flat_map(|member| member.topics.clone())
                .collect(),
            members: self.members.values().cloned().collect(),
            message_policy: self.config.message_policy.clone(),
            wake_policy: self.config.wake_policy.clone(),
            policy_refs: self.config.policy_refs.clone(),
            messages: self.messages.values().cloned().collect(),
            wakes: self.wakes.values().cloned().collect(),
            cursor: self.records.last().map(|record| record.cursor.clone()),
        }
    }
}

impl FileAgentPoolStore {
    /// Creates an agent-pool store rooted under the provided directory.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn state_path(&self, pool_id: &AgentPoolId) -> PathBuf {
        root_join(
            &self.root,
            &[
                "agent_pools".to_string(),
                safe_segment(pool_id.as_str()),
                "state.json".to_string(),
            ],
        )
    }

    fn load_state(&self, pool_id: &AgentPoolId) -> Result<Option<PoolState>, AgentError> {
        read_json(&self.state_path(pool_id))
    }

    fn save_state(&self, pool_id: &AgentPoolId, state: &PoolState) -> Result<(), AgentError> {
        write_json(&self.state_path(pool_id), state)
    }

    fn with_state<T>(
        &self,
        pool_id: &AgentPoolId,
        f: impl FnOnce(&mut PoolState) -> Result<T, AgentError>,
    ) -> Result<T, AgentError> {
        let mut state = self
            .load_state(pool_id)?
            .ok_or_else(|| AgentError::host_configuration_needed("agent pool is not open"))?;
        let output = f(&mut state)?;
        self.save_state(pool_id, &state)?;
        Ok(output)
    }

    fn append_record(
        pool_id: &AgentPoolId,
        state: &mut PoolState,
        payload: AgentPoolStoreRecordPayload,
    ) -> AgentPoolStoreCursor {
        let cursor = AgentPoolStoreCursor::new(state.records.len() as u64 + 1);
        state.records.push(AgentPoolStoreRecord {
            pool_id: pool_id.clone(),
            cursor: cursor.clone(),
            payload,
        });
        cursor
    }
}

impl AgentPoolStore for FileAgentPoolStore {
    fn open_pool(
        &self,
        pool_id: AgentPoolId,
        config: AgentPoolStoreConfig,
    ) -> Result<AgentPoolSnapshot, AgentError> {
        let state = if let Some(existing) = self.load_state(&pool_id)? {
            if existing.config != config {
                return Err(AgentError::contract_violation(
                    "agent pool store config conflicts with existing pool",
                ));
            }
            existing
        } else {
            let mut state = PoolState::new(config.clone());
            Self::append_record(
                &pool_id,
                &mut state,
                AgentPoolStoreRecordPayload::PoolOpened { config },
            );
            state
        };
        let snapshot = state.snapshot(pool_id.clone());
        self.save_state(&pool_id, &state)?;
        Ok(snapshot)
    }

    fn snapshot(&self, pool_id: &AgentPoolId) -> Result<AgentPoolSnapshot, AgentError> {
        self.load_state(pool_id)?
            .map(|state| state.snapshot(pool_id.clone()))
            .ok_or_else(|| AgentError::host_configuration_needed("agent pool is not open"))
    }

    fn record_pool_created(
        &self,
        pool_id: &AgentPoolId,
    ) -> Result<AgentPoolStoreCursor, AgentError> {
        self.with_state(pool_id, |state| {
            state.created = true;
            Ok(Self::append_record(
                pool_id,
                state,
                AgentPoolStoreRecordPayload::PoolCreated,
            ))
        })
    }

    fn join_member(
        &self,
        pool_id: &AgentPoolId,
        member: AgentPoolMember,
    ) -> Result<AgentPoolStoreCursor, AgentError> {
        self.with_state(pool_id, |state| {
            state.members.insert(member.run_id.clone(), member.clone());
            Ok(Self::append_record(
                pool_id,
                state,
                AgentPoolStoreRecordPayload::MemberJoined { member },
            ))
        })
    }

    fn leave_member(
        &self,
        pool_id: &AgentPoolId,
        run_id: &RunId,
    ) -> Result<(AgentPoolMember, AgentPoolStoreCursor), AgentError> {
        self.with_state(pool_id, |state| {
            let member = state.members.remove(run_id).ok_or_else(|| {
                AgentError::contract_violation("run is not a member of this agent pool")
            })?;
            let cursor = Self::append_record(
                pool_id,
                state,
                AgentPoolStoreRecordPayload::MemberLeft {
                    member: member.clone(),
                },
            );
            Ok((member, cursor))
        })
    }

    fn message_receipt(
        &self,
        pool_id: &AgentPoolId,
        idempotency_key: &IdempotencyKey,
    ) -> Result<Option<MessageReceipt>, AgentError> {
        Ok(self
            .load_state(pool_id)?
            .and_then(|state| state.message_dedupe.get(idempotency_key).cloned()))
    }

    fn record_message(
        &self,
        pool_id: &AgentPoolId,
        message: RunMessage,
        receipt: MessageReceipt,
    ) -> Result<AgentPoolStoreCursor, AgentError> {
        self.with_state(pool_id, |state| {
            let stored = AgentPoolStoredMessage {
                message: message.clone(),
                receipt: receipt.clone(),
            };
            state
                .messages
                .insert(message.message_id.as_str().to_string(), stored.clone());
            state
                .message_dedupe
                .insert(message.idempotency_key.clone(), receipt);
            Ok(Self::append_record(
                pool_id,
                state,
                AgentPoolStoreRecordPayload::RunMessage { stored },
            ))
        })
    }

    fn wake_registration(
        &self,
        pool_id: &AgentPoolId,
        idempotency_key: &IdempotencyKey,
    ) -> Result<Option<WakeRegistration>, AgentError> {
        Ok(self
            .load_state(pool_id)?
            .and_then(|state| state.wake_dedupe.get(idempotency_key).cloned()))
    }

    fn wake(
        &self,
        pool_id: &AgentPoolId,
        condition_id: &WakeConditionId,
    ) -> Result<Option<CoreStoredWake>, AgentError> {
        Ok(self
            .load_state(pool_id)?
            .and_then(|state| state.wakes.get(condition_id).cloned()))
    }

    fn record_wake(
        &self,
        pool_id: &AgentPoolId,
        condition: WakeCondition,
        compiled_filter: CompiledEventFilter,
        registration: WakeRegistration,
    ) -> Result<AgentPoolStoreCursor, AgentError> {
        self.with_state(pool_id, |state| {
            let stored = AgentPoolStoredWake {
                condition: condition.clone(),
                compiled_filter,
                registration: registration.clone(),
            };
            state
                .wakes
                .insert(condition.condition_id.clone(), stored.clone());
            state
                .wake_dedupe
                .insert(condition.idempotency_key.clone(), registration);
            Ok(Self::append_record(
                pool_id,
                state,
                AgentPoolStoreRecordPayload::Wake { stored },
            ))
        })
    }

    fn watch(
        &self,
        pool_id: &AgentPoolId,
        cursor: Option<AgentPoolStoreCursor>,
    ) -> Result<AgentPoolStoreStream, AgentError> {
        let after = cursor.map(|cursor| cursor.sequence).unwrap_or(0);
        let records = self
            .load_state(pool_id)?
            .map(|state| {
                state
                    .records
                    .into_iter()
                    .filter(|record| record.cursor.sequence > after)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        Ok(AgentPoolStoreStream::new(records))
    }

    fn next_event_sequence(&self, pool_id: &AgentPoolId) -> Result<u64, AgentError> {
        self.with_state(pool_id, |state| {
            state.next_event_counter += 1;
            Ok(state.next_event_counter)
        })
    }
}
