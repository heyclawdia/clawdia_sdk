use std::path::{Path, PathBuf};

use agent_sdk_core::{
    AgentError, AgentEventStream, ArchiveCursor, CompiledEventFilter, EventArchive,
    EventArchiveReader, EventFrame, domain::ArchiveCursorId,
};
use rusqlite::params;

use crate::util::{decode, encode, open, sqlite_error};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS event_archive_frames (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id TEXT NOT NULL,
    frame_json TEXT NOT NULL
);
";

#[derive(Clone, Debug)]
/// SQLite-backed event archive.
pub struct SqliteEventArchive {
    path: PathBuf,
    archive_id: ArchiveCursorId,
}

impl SqliteEventArchive {
    /// Opens or creates a SQLite event archive.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, AgentError> {
        crate::util::init(path.as_ref(), SCHEMA)?;
        Ok(Self {
            path: path.as_ref().to_path_buf(),
            archive_id: ArchiveCursorId::new("archive.sqlite.default"),
        })
    }

    /// Appends one event frame and assigns an archive cursor when absent.
    pub fn append_frame(&self, mut frame: EventFrame) -> Result<ArchiveCursor, AgentError> {
        let connection = open(&self.path)?;
        let next_seq = connection
            .query_row(
                "SELECT COALESCE(MAX(seq), 0) + 1 FROM event_archive_frames",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map_err(sqlite_error)?;
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
        connection
            .execute(
                "INSERT INTO event_archive_frames (seq, event_id, frame_json)
                 VALUES (?1, ?2, ?3)",
                params![
                    next_seq,
                    frame.event.envelope.event_id.as_str(),
                    encode(&frame)?,
                ],
            )
            .map_err(sqlite_error)?;
        Ok(cursor)
    }
}

impl EventArchiveReader for SqliteEventArchive {
    fn frames_after(&self, cursor: Option<ArchiveCursor>) -> Result<Vec<EventFrame>, AgentError> {
        let start_after = cursor
            .and_then(|cursor| {
                cursor
                    .position
                    .strip_prefix("archive.")
                    .unwrap_or(&cursor.position)
                    .parse::<i64>()
                    .ok()
            })
            .unwrap_or(0);
        let connection = open(&self.path)?;
        let mut statement = connection
            .prepare(
                "SELECT frame_json FROM event_archive_frames
                 WHERE seq > ?1 ORDER BY seq ASC",
            )
            .map_err(sqlite_error)?;
        let rows = statement
            .query_map(params![start_after], |row| row.get::<_, String>(0))
            .map_err(sqlite_error)?;
        let mut frames = Vec::new();
        for row in rows {
            frames.push(decode(&row.map_err(sqlite_error)?)?);
        }
        Ok(frames)
    }
}

impl EventArchive for SqliteEventArchive {
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
