use std::collections::BTreeMap;

use agent_sdk_core::{
    AgentError, AgentPoolId, AgentPoolMember, AgentPoolSnapshot, AgentPoolStore,
    AgentPoolStoreConfig, AgentPoolStoreCursor, AgentPoolStoreRecord, AgentPoolStoreRecordPayload,
    AgentPoolStoreStream, AgentPoolStoredMessage, AgentPoolStoredWake, IdempotencyKey,
    MessageReceipt, RunId, RunMessage, WakeCondition, WakeConditionId, WakeRegistration,
    agent_pool::AgentPoolStoredWake as CoreStoredWake, event::CompiledEventFilter,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{client::SupabaseClient, transport::supabase_error};

#[derive(Clone)]
/// Supabase-backed agent-pool coordination store.
pub struct SupabaseAgentPoolStore {
    client: SupabaseClient,
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

impl SupabaseAgentPoolStore {
    pub fn new(client: SupabaseClient) -> Self {
        Self { client }
    }

    fn load_state(&self, pool_id: &AgentPoolId) -> Result<Option<PoolState>, AgentError> {
        let query = format!(
            "store_scope=eq.{}&pool_id=eq.{}&select=state&limit=1",
            self.client.config().store_scope(),
            pool_id.as_str()
        );
        let response = self.client.select("agent_sdk_agent_pools", &query)?;
        if !(200..300).contains(&response.status) {
            return Err(supabase_error(format!(
                "supabase agent pool read failed with status {}",
                response.status
            )));
        }
        let rows = serde_json::from_slice::<Vec<serde_json::Value>>(&response.body)
            .map_err(|error| supabase_error(error.to_string()))?;
        rows.into_iter()
            .next()
            .map(|row| {
                serde_json::from_value(row["state"].clone()).map_err(|error| {
                    AgentError::contract_violation(format!(
                        "supabase agent pool state decode failed: {error}"
                    ))
                })
            })
            .transpose()
    }

    fn save_state(&self, pool_id: &AgentPoolId, state: &PoolState) -> Result<(), AgentError> {
        let response = self.client.rpc(
            "agent_sdk_upsert_agent_pool_state",
            &json!({
                "p_store_scope": self.client.config().store_scope(),
                "p_pool_id": pool_id.as_str(),
                "p_state": state,
            }),
        )?;
        if !(200..300).contains(&response.status) {
            return Err(supabase_error(format!(
                "supabase agent pool save failed with status {}",
                response.status
            )));
        }
        Ok(())
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

impl AgentPoolStore for SupabaseAgentPoolStore {
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
        let response = self.client.rpc(
            "agent_sdk_next_agent_pool_event_sequence",
            &json!({
                "p_store_scope": self.client.config().store_scope(),
                "p_pool_id": pool_id.as_str(),
            }),
        )?;
        if !(200..300).contains(&response.status) {
            return Err(supabase_error(format!(
                "supabase agent pool sequence allocation failed with status {}",
                response.status
            )));
        }
        let value = serde_json::from_slice::<serde_json::Value>(&response.body)
            .map_err(|error| supabase_error(error.to_string()))?;
        parse_sequence(value)
            .ok_or_else(|| supabase_error("supabase agent pool sequence response missing value"))
    }
}

fn parse_sequence(value: serde_json::Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_array()?.first()?.as_u64())
        .or_else(|| value.as_array()?.first()?.get("next_sequence")?.as_u64())
        .or_else(|| value.get("next_sequence")?.as_u64())
}
