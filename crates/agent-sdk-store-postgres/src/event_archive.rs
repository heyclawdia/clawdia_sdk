use agent_sdk_core::{
    AgentError, ArchiveCursor, CompiledEventFilter, EventArchive, EventArchiveReader, EventFrame,
    domain::ArchiveCursorId,
};
use serde_json::Value;

use crate::{
    PostgresStoreClient,
    util::{decode_row, json_value},
};

#[derive(Clone)]
pub struct PostgresEventArchive {
    client: PostgresStoreClient,
}

impl PostgresEventArchive {
    pub fn new(client: PostgresStoreClient) -> Self {
        Self { client }
    }

    pub fn append_frame(&self, frame: EventFrame) -> Result<ArchiveCursor, AgentError> {
        let response = self.client.execute(
            format!("insert into {} (store_scope, event_id, frame_json) values ($1, $2, $3) returning archive_seq", self.client.table("agent_sdk_event_archive")),
                vec![self.client.scope(), Value::String(frame.event.envelope.event_id.as_str().to_string()), json_value(&frame)?],
        )?;
        let seq = response
            .rows
            .first()
            .and_then(|row| row.get("archive_seq"))
            .and_then(Value::as_u64)
            .unwrap_or(1);
        Ok(ArchiveCursor {
            archive_id: ArchiveCursorId::new("archive.postgres.default"),
            position: format!("archive.{seq}"),
            event_id: Some(frame.event.envelope.event_id),
            watermark: None,
        })
    }
}

impl EventArchiveReader for PostgresEventArchive {
    fn frames_after(&self, cursor: Option<ArchiveCursor>) -> Result<Vec<EventFrame>, AgentError> {
        let after = cursor
            .and_then(|cursor| {
                cursor
                    .position
                    .strip_prefix("archive.")
                    .unwrap_or(&cursor.position)
                    .parse::<u64>()
                    .ok()
            })
            .unwrap_or(0);
        let response = self.client.execute(
            format!("select frame_json from {} where store_scope = $1 and archive_seq > $2 order by archive_seq asc", self.client.table("agent_sdk_event_archive")),
            vec![self.client.scope(), Value::from(after)],
        )?;
        response
            .rows
            .into_iter()
            .map(|row| decode_row(row, "frame_json"))
            .collect()
    }
}

impl EventArchive for PostgresEventArchive {
    fn replay_filtered_from_cursor(
        &self,
        filter: CompiledEventFilter,
        cursor: ArchiveCursor,
    ) -> Result<agent_sdk_core::AgentEventStream, AgentError> {
        let frames = self
            .frames_after(Some(cursor))?
            .into_iter()
            .filter(|frame| filter.matches_envelope(&frame.event.envelope))
            .collect::<Vec<_>>();
        Ok(agent_sdk_core::AgentEventStream::new(frames))
    }
}
