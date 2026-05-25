//! Concrete workspace tool helpers layered over core tool/effect contracts. Use these
//! modules for bounded read, search, edit, write, and format-aware extraction
//! behavior under a host-selected workspace policy. Reads search local files;
//! edit/write helpers may mutate files only through explicit executor calls. This
//! file contains the sqlite portion of that contract.
//!
use std::path::Path;

use rusqlite::{Connection, OpenFlags, types::ValueRef};

use super::{RenderedRead, add_truncation_guidance};
use crate::workspace::{
    read_pipeline::{WorkspaceReaderStep, WorkspaceSqliteMetadata, WorkspaceSqliteTableMetadata},
    util::truncate_bytes,
};

const MAX_SQLITE_OBJECTS: usize = 20;
const MAX_SAMPLE_ROWS: usize = 3;
const MAX_SAMPLE_COLUMNS: usize = 12;
const MAX_CELL_BYTES: usize = 120;

/// Render sqlite.
/// This parses caller-provided bytes into a bounded rendered read response and does not write
/// workspace files.
pub(super) fn render_sqlite(
    path: &Path,
    max_output_bytes: u64,
) -> Result<RenderedRead, agent_sdk_core::AgentError> {
    let mut warnings = Vec::new();
    let connection = match Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) {
        Ok(connection) => connection,
        Err(error) => {
            warnings.push(format!("SQLite open failed: {error}"));
            return Ok(sqlite_summary_read(
                "SQLite database detected, but it could not be opened read-only.".to_string(),
                None,
                warnings,
                max_output_bytes,
            ));
        }
    };
    if let Err(error) =
        connection.execute_batch("PRAGMA query_only = ON; PRAGMA trusted_schema = OFF;")
    {
        warnings.push(format!("SQLite query-only setup warning: {error}"));
    }
    let mut statement = match connection.prepare(
        "SELECT name, type, sql FROM sqlite_schema \
         WHERE type IN ('table', 'view') AND name NOT LIKE 'sqlite_%' \
         ORDER BY type, name LIMIT ?1",
    ) {
        Ok(statement) => statement,
        Err(error) => {
            warnings.push(format!("SQLite schema query failed: {error}"));
            return Ok(sqlite_summary_read(
                "SQLite database detected, but schema inspection failed.".to_string(),
                None,
                warnings,
                max_output_bytes,
            ));
        }
    };
    let object_rows = match statement.query_map([MAX_SQLITE_OBJECTS as i64], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?.unwrap_or_default(),
        ))
    }) {
        Ok(rows) => rows,
        Err(error) => {
            warnings.push(format!("SQLite schema rows failed: {error}"));
            return Ok(sqlite_summary_read(
                "SQLite database detected, but schema row inspection failed.".to_string(),
                None,
                warnings,
                max_output_bytes,
            ));
        }
    };
    let mut tables = Vec::new();
    for object in object_rows {
        let (name, kind, sql) = match object {
            Ok(object) => object,
            Err(error) => {
                warnings.push(format!("SQLite schema row failed: {error}"));
                continue;
            }
        };
        let columns = columns_for(&connection, &name, &mut warnings)?;
        let sample_rows = sample_rows_for(&connection, &name, columns.len(), &mut warnings)?;
        let mut table = WorkspaceSqliteTableMetadata {
            name,
            kind,
            columns,
            sample_rows,
        };
        if table.columns.is_empty() && !sql.is_empty() {
            table.columns.push(truncate_bytes(&sql, MAX_CELL_BYTES));
        }
        tables.push(table);
    }
    let truncated_objects = tables.len() == MAX_SQLITE_OBJECTS;
    let mut summary = format!("SQLite database: {} object(s)\n", tables.len());
    for table in &tables {
        summary.push_str(&format!("- {} `{}`", table.kind, table.name));
        if !table.columns.is_empty() {
            summary.push_str(&format!(" columns=[{}]", table.columns.join(", ")));
        }
        summary.push('\n');
        for row in &table.sample_rows {
            summary.push_str("  row: ");
            summary.push_str(&row.join(" | "));
            summary.push('\n');
        }
    }
    if truncated_objects {
        warnings.push("SQLite object listing hit the reader object limit".to_string());
    }
    Ok(sqlite_summary_read(
        summary,
        Some(WorkspaceSqliteMetadata {
            parser: "rusqlite:0.39.0".to_string(),
            table_count: tables.len(),
            tables,
            truncated: truncated_objects,
            warnings: warnings.clone(),
        }),
        warnings,
        max_output_bytes,
    ))
}

fn sqlite_summary_read(
    summary: String,
    sqlite: Option<WorkspaceSqliteMetadata>,
    warnings: Vec<String>,
    max_output_bytes: u64,
) -> RenderedRead {
    let truncated = summary.len() as u64 > max_output_bytes;
    let mut rendered = RenderedRead {
        content: if truncated {
            truncate_bytes(&summary, max_output_bytes as usize)
        } else {
            summary.clone()
        },
        content_summary: Some(summary),
        truncated,
        binary: true,
        anchors: Vec::new(),
        reader_pipeline: vec![
            WorkspaceReaderStep::DetectFileType,
            WorkspaceReaderStep::InspectSqliteDatabase,
            WorkspaceReaderStep::SummarizeBinary,
        ],
        media: None,
        document: None,
        archive: None,
        sqlite,
        resource: None,
        warnings,
    };
    add_truncation_guidance(&mut rendered);
    rendered
}

fn columns_for(
    connection: &Connection,
    table: &str,
    warnings: &mut Vec<String>,
) -> Result<Vec<String>, agent_sdk_core::AgentError> {
    let pragma = format!("PRAGMA table_info({})", quote_identifier(table));
    let mut statement = connection
        .prepare(&pragma)
        .map_err(|error| super::extraction_error("sqlite", error))?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| super::extraction_error("sqlite", error))?;
    let mut columns = Vec::new();
    for row in rows.take(MAX_SAMPLE_COLUMNS) {
        match row {
            Ok(column) => columns.push(column),
            Err(error) => warnings.push(format!("SQLite column read failed: {error}")),
        }
    }
    Ok(columns)
}

fn sample_rows_for(
    connection: &Connection,
    table: &str,
    column_count: usize,
    warnings: &mut Vec<String>,
) -> Result<Vec<Vec<String>>, agent_sdk_core::AgentError> {
    if column_count == 0 {
        return Ok(Vec::new());
    }
    let query = format!(
        "SELECT * FROM {} LIMIT {MAX_SAMPLE_ROWS}",
        quote_identifier(table)
    );
    let mut statement = match connection.prepare(&query) {
        Ok(statement) => statement,
        Err(error) => {
            warnings.push(format!(
                "SQLite sample query skipped for `{table}`: {error}"
            ));
            return Ok(Vec::new());
        }
    };
    let mut rows = statement
        .query([])
        .map_err(|error| super::extraction_error("sqlite", error))?;
    let mut samples = Vec::new();
    while let Some(row) = rows
        .next()
        .map_err(|error| super::extraction_error("sqlite", error))?
    {
        let mut values = Vec::new();
        for index in 0..column_count.min(MAX_SAMPLE_COLUMNS) {
            values.push(format_value(
                row.get_ref(index)
                    .map_err(|error| super::extraction_error("sqlite", error))?,
            ));
        }
        samples.push(values);
    }
    Ok(samples)
}

fn format_value(value: ValueRef<'_>) -> String {
    match value {
        ValueRef::Null => "NULL".to_string(),
        ValueRef::Integer(value) => value.to_string(),
        ValueRef::Real(value) => value.to_string(),
        ValueRef::Text(value) => truncate_bytes(&String::from_utf8_lossy(value), MAX_CELL_BYTES),
        ValueRef::Blob(value) => format!("<blob:{} bytes>", value.len()),
    }
}

fn quote_identifier(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}
