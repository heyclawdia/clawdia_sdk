use agent_sdk_core::{
    AgentError, AgentEventStream, ArchiveCursor, CompiledEventFilter, EventArchive,
    EventArchiveReader, EventFrame,
};

use crate::{client::SupabaseClient, transport::supabase_error};

#[derive(Clone)]
/// Supabase-backed event archive.
pub struct SupabaseEventArchive {
    client: SupabaseClient,
}

impl SupabaseEventArchive {
    pub fn new(client: SupabaseClient) -> Self {
        Self { client }
    }
}

impl EventArchiveReader for SupabaseEventArchive {
    fn frames_after(&self, cursor: Option<ArchiveCursor>) -> Result<Vec<EventFrame>, AgentError> {
        let after = cursor
            .map(|cursor| cursor.position)
            .unwrap_or_else(|| "archive.0".to_string());
        let query = format!(
            "store_scope=eq.{}&position=gt.{}&select=frame&order=position.asc",
            self.client.config().store_scope(),
            after
        );
        let response = self.client.select("agent_sdk_event_archive", &query)?;
        if !(200..300).contains(&response.status) {
            return Err(supabase_error(format!(
                "supabase event archive read failed with status {}",
                response.status
            )));
        }
        let rows = serde_json::from_slice::<Vec<serde_json::Value>>(&response.body)
            .map_err(|error| supabase_error(error.to_string()))?;
        rows.into_iter()
            .map(|row| {
                serde_json::from_value::<EventFrame>(row["frame"].clone())
                    .map_err(|error| supabase_error(error.to_string()))
            })
            .collect()
    }
}

impl EventArchive for SupabaseEventArchive {
    fn replay_filtered_from_cursor(
        &self,
        filter: CompiledEventFilter,
        cursor: ArchiveCursor,
    ) -> Result<AgentEventStream, AgentError> {
        Ok(AgentEventStream::new(
            self.frames_after(Some(cursor))?
                .into_iter()
                .filter(|frame| filter.matches_envelope(&frame.event.envelope)),
        ))
    }
}
