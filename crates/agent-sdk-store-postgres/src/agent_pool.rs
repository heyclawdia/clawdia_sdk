use agent_sdk_core::{
    AgentError, AgentPoolId, AgentPoolMember, AgentPoolMessagePolicy, AgentPoolSnapshot,
    AgentPoolStore, AgentPoolStoreConfig, AgentPoolStoreCursor, AgentPoolStoreRecord,
    AgentPoolStoreRecordPayload, AgentPoolStoreStream, AgentPoolStoredWake, AgentPoolWakePolicy,
    CompiledEventFilter, IdempotencyKey, MessageReceipt, RunId, RunMessage, WakeCondition,
    WakeConditionId, WakeRegistration,
};
use serde_json::{Value, json};

use crate::{PostgresStoreClient, util::json_value};

#[derive(Clone)]
pub struct PostgresAgentPoolStore {
    client: PostgresStoreClient,
}

impl PostgresAgentPoolStore {
    pub fn new(client: PostgresStoreClient) -> Self {
        Self { client }
    }
}

impl AgentPoolStore for PostgresAgentPoolStore {
    fn open_pool(
        &self,
        pool_id: AgentPoolId,
        config: AgentPoolStoreConfig,
    ) -> Result<AgentPoolSnapshot, AgentError> {
        self.client.execute(
            format!(
                "select state_json from {} where store_scope = $1 and pool_id = $2",
                self.client.table("agent_sdk_agent_pools")
            ),
            vec![
                self.client.scope(),
                Value::String(pool_id.as_str().to_string()),
            ],
        )?;
        Ok(AgentPoolSnapshot {
            pool_id,
            created: false,
            members: Vec::new(),
            topics: Vec::new(),
            message_policy: config.message_policy,
            wake_policy: config.wake_policy,
            policy_refs: config.policy_refs,
            messages: Vec::new(),
            wakes: Vec::new(),
            cursor: None,
        })
    }

    fn snapshot(&self, pool_id: &AgentPoolId) -> Result<AgentPoolSnapshot, AgentError> {
        Ok(AgentPoolSnapshot {
            pool_id: pool_id.clone(),
            created: false,
            members: Vec::new(),
            topics: Vec::new(),
            message_policy: AgentPoolMessagePolicy::bounded_defaults(),
            wake_policy: AgentPoolWakePolicy::safe_defaults(),
            policy_refs: Vec::new(),
            messages: Vec::new(),
            wakes: Vec::new(),
            cursor: None,
        })
    }

    fn record_pool_created(
        &self,
        pool_id: &AgentPoolId,
    ) -> Result<AgentPoolStoreCursor, AgentError> {
        self.append_pool_record(pool_id, json!({"kind": "pool_created"}))
    }

    fn join_member(
        &self,
        pool_id: &AgentPoolId,
        member: AgentPoolMember,
    ) -> Result<AgentPoolStoreCursor, AgentError> {
        self.append_pool_record(
            pool_id,
            json_value(&AgentPoolStoreRecordPayload::MemberJoined { member })?,
        )
    }

    fn leave_member(
        &self,
        _pool_id: &AgentPoolId,
        _run_id: &RunId,
    ) -> Result<(AgentPoolMember, AgentPoolStoreCursor), AgentError> {
        Err(AgentError::contract_violation(
            "scripted Postgres agent pool leave_member requires caller-provided row fixture",
        ))
    }

    fn message_receipt(
        &self,
        _pool_id: &AgentPoolId,
        _idempotency_key: &IdempotencyKey,
    ) -> Result<Option<MessageReceipt>, AgentError> {
        Ok(None)
    }

    fn record_message(
        &self,
        pool_id: &AgentPoolId,
        message: RunMessage,
        receipt: MessageReceipt,
    ) -> Result<AgentPoolStoreCursor, AgentError> {
        self.append_pool_record(
            pool_id,
            json_value(&AgentPoolStoreRecordPayload::RunMessage {
                stored: agent_sdk_core::AgentPoolStoredMessage { message, receipt },
            })?,
        )
    }

    fn wake_registration(
        &self,
        _pool_id: &AgentPoolId,
        _idempotency_key: &IdempotencyKey,
    ) -> Result<Option<WakeRegistration>, AgentError> {
        Ok(None)
    }

    fn wake(
        &self,
        _pool_id: &AgentPoolId,
        _condition_id: &WakeConditionId,
    ) -> Result<Option<AgentPoolStoredWake>, AgentError> {
        Ok(None)
    }

    fn record_wake(
        &self,
        pool_id: &AgentPoolId,
        condition: WakeCondition,
        compiled_filter: CompiledEventFilter,
        registration: WakeRegistration,
    ) -> Result<AgentPoolStoreCursor, AgentError> {
        let wake = AgentPoolStoredWake {
            condition,
            compiled_filter,
            registration,
        };
        self.append_pool_record(
            pool_id,
            json_value(&AgentPoolStoreRecordPayload::Wake { stored: wake })?,
        )
    }

    fn watch(
        &self,
        _pool_id: &AgentPoolId,
        _cursor: Option<AgentPoolStoreCursor>,
    ) -> Result<AgentPoolStoreStream, AgentError> {
        Ok(AgentPoolStoreStream::new(Vec::<AgentPoolStoreRecord>::new()))
    }

    fn next_event_sequence(&self, pool_id: &AgentPoolId) -> Result<u64, AgentError> {
        let response = self.client.execute(
            format!(
                "select {}.next_agent_pool_event_sequence($1, $2) as next_sequence",
                self.client.config.schema
            ),
            vec![
                self.client.scope(),
                Value::String(pool_id.as_str().to_string()),
            ],
        )?;
        Ok(response
            .rows
            .first()
            .and_then(|row| row.get("next_sequence"))
            .and_then(Value::as_u64)
            .unwrap_or(1))
    }
}

impl PostgresAgentPoolStore {
    fn append_pool_record(
        &self,
        pool_id: &AgentPoolId,
        payload: Value,
    ) -> Result<AgentPoolStoreCursor, AgentError> {
        let response = self.client.execute(
            format!("insert into {} (store_scope, pool_id, payload_json) values ($1, $2, $3) returning seq", self.client.table("agent_sdk_agent_pool_records")),
            vec![self.client.scope(), Value::String(pool_id.as_str().to_string()), payload],
        )?;
        let seq = response
            .rows
            .first()
            .and_then(|row| row.get("seq"))
            .and_then(Value::as_u64)
            .unwrap_or(1);
        Ok(AgentPoolStoreCursor::new(seq))
    }
}
