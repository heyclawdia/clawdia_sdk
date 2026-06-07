use std::path::PathBuf;

use agent_sdk_core::{
    AgentError, AgentEventStream, ArchiveCursor, CompiledEventFilter, EventArchive,
    EventArchiveReader, EventFrame, domain::ArchiveCursorId,
};

use crate::util::{append_json_line, parse_cursor_seq, read_json_lines, root_join};

#[derive(Clone, Debug)]
/// Filesystem-backed event archive.
pub struct FileEventArchive {
    root: PathBuf,
    archive_id: ArchiveCursorId,
}

impl FileEventArchive {
    /// Creates an event archive rooted under the provided directory.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            archive_id: ArchiveCursorId::new("archive.file.default"),
        }
    }

    /// Creates an event archive with an explicit archive id.
    pub fn with_archive_id(root: impl Into<PathBuf>, archive_id: ArchiveCursorId) -> Self {
        Self {
            root: root.into(),
            archive_id,
        }
    }

    /// Appends one frame to the archive, assigning an archive cursor when absent.
    pub fn append_frame(&self, mut frame: EventFrame) -> Result<ArchiveCursor, AgentError> {
        let next_seq = self.frames_after(None)?.len() as u64 + 1;
        let cursor = frame
            .archive_cursor
            .clone()
            .unwrap_or_else(|| ArchiveCursor {
                archive_id: self.archive_id.clone(),
                position: format!("archive.{next_seq}"),
                event_id: Some(frame.event.envelope.event_id.clone()),
                watermark: None,
            });
        frame.archive_cursor = Some(cursor.clone());
        append_json_line(&self.archive_path(), &frame)?;
        Ok(cursor)
    }

    fn archive_path(&self) -> PathBuf {
        root_join(
            &self.root,
            &["events".to_string(), "archive.ndjson".to_string()],
        )
    }
}

impl EventArchiveReader for FileEventArchive {
    fn frames_after(&self, cursor: Option<ArchiveCursor>) -> Result<Vec<EventFrame>, AgentError> {
        let start_after = cursor
            .as_ref()
            .and_then(|cursor| parse_cursor_seq(&cursor.position, "archive."));
        Ok(read_json_lines::<EventFrame>(&self.archive_path())?
            .into_iter()
            .filter(|frame| {
                frame
                    .archive_cursor
                    .as_ref()
                    .and_then(|cursor| parse_cursor_seq(&cursor.position, "archive."))
                    .is_some_and(|seq| start_after.is_none_or(|after| seq > after))
            })
            .collect())
    }
}

impl EventArchive for FileEventArchive {
    fn replay_filtered_from_cursor(
        &self,
        filter: CompiledEventFilter,
        cursor: ArchiveCursor,
    ) -> Result<AgentEventStream, AgentError> {
        let frames = self
            .frames_after(Some(cursor))?
            .into_iter()
            .filter(|frame| filter.matches_envelope(&frame.event.envelope))
            .collect::<Vec<_>>();
        Ok(AgentEventStream::new(frames))
    }
}
