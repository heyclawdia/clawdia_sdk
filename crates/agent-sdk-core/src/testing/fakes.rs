//! Reusable deterministic fakes for SDK consumers. Use this module in tests to
//! exercise public ports without live providers, real stores, network telemetry, or
//! product UI. Fakes may mutate only their in-memory state.
//!
use std::{
    cell::Cell,
    collections::BTreeMap,
    fs,
    path::Path,
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{
    domain::{AgentError, ContentId},
    error::{AgentErrorKind, RetryClassification},
    events::EventFrame,
    journal::{JOURNAL_SCHEMA_VERSION, JournalCursor, JournalRecord},
    journal_ports::RunJournal,
    provider::{
        ProviderAdapter, ProviderCapabilities, ProviderRequest, ProviderResponse,
        ProviderStopReason, ProviderUsage,
    },
};

/// Constant value for the testing::fakes contract. Use it to keep SDK
/// records and tests aligned on the same stable value.
pub const FIXTURE_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Debug)]
/// In-memory fake fixture harness fixture for SDK conformance tests.
/// Use it to script deterministic behavior in memory; any transcript or endpoint mutation is documented on the method that performs it.
pub struct FakeFixtureHarness {
    /// Deterministic seed used by this record or request.
    pub deterministic_seed: u64,
    /// Identifiers used to select or correlate ids values.
    /// Use them for typed lookup, filtering, or lineage instead of stringly typed matching.
    pub ids: DeterministicIdGenerator,
    /// Clock used by this record or request.
    pub clock: DeterministicClock,
    /// Content store used by this record or request.
    pub content_store: FakeContentStore,
    /// Journal store used by this record or request.
    pub journal_store: FakeJournalStore,
    /// Event sink used by this record or request.
    pub event_sink: FakeEventSink,
    /// Provider used by this record or request.
    pub provider: FakeProvider,
}

impl FakeFixtureHarness {
    /// Returns this value with its seed setting replaced. The method
    /// follows builder-style data construction and does not execute
    /// external work.
    pub fn with_seed(deterministic_seed: u64) -> Self {
        Self {
            deterministic_seed,
            ids: DeterministicIdGenerator::new(deterministic_seed),
            clock: DeterministicClock::new(deterministic_seed),
            content_store: FakeContentStore::default(),
            journal_store: FakeJournalStore::default(),
            event_sink: FakeEventSink::default(),
            provider: FakeProvider::default(),
        }
    }
}

impl Default for FakeFixtureHarness {
    fn default() -> Self {
        Self::with_seed(0)
    }
}

#[derive(Clone, Debug)]
/// In-memory deterministic id generator fixture for SDK conformance tests.
/// Use it to script deterministic behavior in memory; any transcript or endpoint mutation is documented on the method that performs it.
pub struct DeterministicIdGenerator {
    seed: u64,
    next: Cell<u64>,
}

impl DeterministicIdGenerator {
    /// Creates a new testing::fakes value with explicit caller-provided
    /// inputs. This constructor is data-only and performs no I/O or
    /// external side effects.
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            next: Cell::new(0),
        }
    }

    /// Builds the next raw value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn next_raw(&self, prefix: &str) -> String {
        let seq = self.next.get();
        self.next.set(seq + 1);
        format!("{prefix}.{:04}.{:04}", self.seed, seq)
    }

    /// Returns the next content ref currently held by this value.
    /// This reads deterministic in-memory test state and performs no external I/O.
    pub fn next_content_ref(&self) -> ContentId {
        ContentId::new(self.next_raw("content"))
    }
}

#[derive(Clone, Debug)]
/// In-memory deterministic clock fixture for SDK conformance tests.
/// Use it to script deterministic behavior in memory; any transcript or endpoint mutation is documented on the method that performs it.
pub struct DeterministicClock {
    start_millis: u64,
    step_millis: u64,
    ticks: Cell<u64>,
}

impl DeterministicClock {
    /// Creates a new testing::fakes value with explicit caller-provided
    /// inputs. This constructor is data-only and performs no I/O or
    /// external side effects.
    pub fn new(seed: u64) -> Self {
        Self {
            start_millis: seed,
            step_millis: 1,
            ticks: Cell::new(0),
        }
    }

    /// Builds the next millis value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn next_millis(&self) -> u64 {
        let ticks = self.ticks.get();
        self.ticks.set(ticks + 1);
        self.start_millis + ticks * self.step_millis
    }
}

#[derive(Clone, Debug, Default)]
/// In-memory fake content store fixture for SDK conformance tests.
/// Use it to script deterministic behavior in memory; any transcript or endpoint mutation is documented on the method that performs it.
pub struct FakeContentStore {
    entries: Arc<Mutex<BTreeMap<ContentId, StoredContent>>>,
}

impl FakeContentStore {
    /// Builds the put text value.
    /// This is data construction and performs no I/O, journal append, event publication, or
    /// process work.
    pub fn put_text(&self, content_ref: ContentId, text: impl Into<String>) {
        self.entries.lock().expect("content store lock").insert(
            content_ref,
            StoredContent {
                media_type: "text/plain; charset=utf-8".to_string(),
                bytes: text.into().into_bytes(),
                redacted_summary: "text content".to_string(),
            },
        );
    }

    /// Looks up an entry in this local store without registry or runtime work.
    /// This reads deterministic in-memory test store state and performs no external I/O.
    pub fn get(&self, content_ref: &ContentId) -> Option<StoredContent> {
        self.entries
            .lock()
            .expect("content store lock")
            .get(content_ref)
            .cloned()
    }

    /// Returns the manifest currently held by this value.
    /// This configures deterministic in-memory test state only.
    pub fn manifest(&self) -> Vec<StoredContentManifestEntry> {
        self.entries
            .lock()
            .expect("content store lock")
            .iter()
            .map(|(content_ref, content)| StoredContentManifestEntry {
                content_ref: content_ref.as_str().to_string(),
                media_type: content.media_type.clone(),
                byte_len: content.bytes.len(),
                redacted_summary: content.redacted_summary.clone(),
            })
            .collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// In-memory stored content fixture for SDK conformance tests.
/// Use it to script deterministic behavior in memory; any transcript or endpoint mutation is documented on the method that performs it.
pub struct StoredContent {
    /// Media type used by this record or request.
    pub media_type: String,
    /// Byte size or byte limit for bytes.
    /// Use it to enforce bounded reads, writes, summaries, or parser output.
    pub bytes: Vec<u8>,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// In-memory stored content manifest entry fixture for SDK conformance tests.
/// Use it to script deterministic behavior in memory; any transcript or endpoint mutation is documented on the method that performs it.
pub struct StoredContentManifestEntry {
    /// Content reference where payload bytes or structured tool output are
    /// stored.
    pub content_ref: String,
    /// Media type used by this record or request.
    pub media_type: String,
    /// Observed byte length for the source, sidecar, or extracted record.
    pub byte_len: usize,
    /// Redacted human-readable summary safe for events, telemetry, and logs.
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Default)]
/// In-memory fake journal store fixture for SDK conformance tests.
/// Use it to script deterministic behavior in memory; any transcript or endpoint mutation is documented on the method that performs it.
pub struct FakeJournalStore {
    records: Arc<Mutex<Vec<JournalRecord>>>,
    fail_next_append: Arc<Mutex<Option<String>>>,
}

impl FakeJournalStore {
    /// Returns the records currently held by this value.
    /// This configures deterministic in-memory test state only.
    pub fn records(&self) -> Vec<JournalRecord> {
        self.records.lock().expect("journal store lock").clone()
    }

    /// Returns the normalized records currently held by this value.
    /// This reads deterministic in-memory test state and performs no external I/O.
    pub fn normalized_records(&self) -> Vec<Value> {
        self.records()
            .into_iter()
            .map(|record| normalize_json_value(serde_json::to_value(record).expect("record JSON")))
            .collect()
    }

    /// Fail next append.
    /// This reads or mutates deterministic in-memory test state unless the method explicitly
    /// names a fixture file.
    pub fn fail_next_append(&self, message: impl Into<String>) {
        *self.fail_next_append.lock().expect("journal fail lock") = Some(message.into());
    }
}

impl RunJournal for FakeJournalStore {
    fn append(&self, record: JournalRecord) -> Result<JournalCursor, AgentError> {
        if let Some(message) = self
            .fail_next_append
            .lock()
            .expect("journal fail lock")
            .take()
        {
            return Err(AgentError::new(
                AgentErrorKind::JournalFailure,
                RetryClassification::RepairNeeded,
                message,
            ));
        }
        let mut records = self.records.lock().expect("journal store lock");
        if record.journal_schema_version != JOURNAL_SCHEMA_VERSION {
            return Err(AgentError::contract_violation(
                "journal record schema version mismatch",
            ));
        }
        if record.journal_seq != records.len() as u64 + 1 {
            return Err(AgentError::contract_violation(
                "journal_seq must be monotonic within fake journal",
            ));
        }
        records.push(record);
        Ok(JournalCursor::new(format!("journal.{}", records.len())))
    }
}

#[derive(Clone, Debug, Default)]
/// In-memory fake event sink fixture for SDK conformance tests.
/// Use it to script deterministic behavior in memory; any transcript or endpoint mutation is documented on the method that performs it.
pub struct FakeEventSink {
    frames: Arc<Mutex<Vec<EventFrame>>>,
}

impl FakeEventSink {
    /// Emit.
    /// This reads or mutates deterministic in-memory test state unless the method explicitly
    /// names a fixture file.
    pub fn emit(&self, frame: EventFrame) {
        self.frames.lock().expect("event sink lock").push(frame);
    }

    /// Returns the frames currently held by this value.
    /// This configures deterministic in-memory test state only.
    pub fn frames(&self) -> Vec<EventFrame> {
        self.frames.lock().expect("event sink lock").clone()
    }

    /// Returns the normalized events currently held by this value.
    /// This reads deterministic in-memory test state and performs no external I/O.
    pub fn normalized_events(&self) -> Vec<Value> {
        self.frames()
            .into_iter()
            .enumerate()
            .map(|(index, frame)| normalized_event_frame(index + 1, frame))
            .collect()
    }
}

fn normalized_event_frame(seq: usize, frame: EventFrame) -> Value {
    let event = frame.event;
    let envelope = event.envelope;
    normalize_json_value(serde_json::json!({
        "schema_version": FIXTURE_SCHEMA_VERSION,
        "event_seq": seq,
        "event": {
            "event_id": envelope.event_id.as_str(),
            "run_id": envelope.run_id.as_str(),
            "agent_id": envelope.agent_id.as_str(),
            "family": format!("{:?}", envelope.event_family),
            "kind": format!("{:?}", envelope.event_kind),
            "privacy": format!("{:?}", envelope.privacy),
        },
    }))
}

#[derive(Clone, Debug)]
/// In-memory fake provider fixture for SDK conformance tests.
/// Use it to script deterministic behavior in memory; any transcript or endpoint mutation is documented on the method that performs it.
pub struct FakeProvider {
    responses: Arc<Mutex<Vec<String>>>,
    requests: Arc<Mutex<Vec<ProviderRequest>>>,
}

impl FakeProvider {
    /// Returns this value with its responses setting replaced. The
    /// method follows builder-style data construction and does not
    /// execute external work.
    pub fn with_responses(responses: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let mut responses = responses
            .into_iter()
            .map(Into::into)
            .collect::<Vec<String>>();
        responses.reverse();
        Self {
            responses: Arc::new(Mutex::new(responses)),
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Returns the requests currently held by this value.
    /// This configures deterministic in-memory test state only.
    pub fn requests(&self) -> Vec<ProviderRequest> {
        self.requests
            .lock()
            .expect("provider requests lock")
            .clone()
    }

    fn pop_response(&self) -> Result<String, AgentError> {
        self.responses
            .lock()
            .expect("provider responses lock")
            .pop()
            .ok_or_else(|| {
                AgentError::contract_violation("fake provider exhausted deterministic responses")
            })
    }
}

impl Default for FakeProvider {
    fn default() -> Self {
        Self::with_responses(["fake provider response"])
    }
}

impl ProviderAdapter for FakeProvider {
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::text_only("provider.fake")
    }

    fn complete(&self, request: &ProviderRequest) -> Result<ProviderResponse, AgentError> {
        self.requests
            .lock()
            .expect("provider requests lock")
            .push(request.clone());

        let output_text = self.pop_response()?;
        let input_tokens = request
            .messages
            .iter()
            .map(|message| message.content.split_whitespace().count() as u32)
            .sum::<u32>();
        let output_tokens = output_text.split_whitespace().count() as u32;

        Ok(ProviderResponse {
            schema_version: ProviderResponse::SCHEMA_VERSION,
            output_text,
            stop_reason: ProviderStopReason::EndTurn,
            tool_calls: Vec::new(),
            usage: Some(ProviderUsage {
                input_tokens: Some(input_tokens),
                output_tokens: Some(output_tokens),
                total_tokens: Some(input_tokens + output_tokens),
            }),
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// In-memory fixture manifest fixture for SDK conformance tests.
/// Use it to script deterministic behavior in memory; any transcript or endpoint mutation is documented on the method that performs it.
pub struct FixtureManifest {
    /// Wire schema version used for compatibility checks.
    pub schema_version: u16,
    /// Fixture name used by this record or request.
    pub fixture_name: String,
    /// Redaction used by this record or request.
    pub redaction: String,
    /// Bounded entries included in this record. Limits and truncation are
    /// represented by companion metadata when applicable.
    pub entries: Vec<FixtureManifestEntry>,
}

impl FixtureManifest {
    /// Creates a new testing::fakes value with explicit caller-provided
    /// inputs. This constructor is data-only and performs no I/O or
    /// external side effects.
    pub fn new(fixture_name: impl Into<String>) -> Self {
        Self {
            schema_version: FIXTURE_SCHEMA_VERSION,
            fixture_name: fixture_name.into(),
            redaction: "golden fixtures contain redacted summaries or metadata unless a later contract explicitly opts into raw content".to_string(),
            entries: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// In-memory fixture manifest entry fixture for SDK conformance tests.
/// Use it to script deterministic behavior in memory; any transcript or endpoint mutation is documented on the method that performs it.
pub struct FixtureManifestEntry {
    /// Workspace-relative or resource path selected by the request or result.
    pub path: String,
    /// Contract used by this record or request.
    pub contract: String,
    /// Wire schema version used for compatibility checks.
    pub schema_version: u16,
}

/// Write fixture.
/// This writes normalized JSON to the caller-provided fixture path on disk.
pub fn write_fixture(path: impl AsRef<Path>, value: &Value) -> Result<(), AgentError> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(io_error)?;
    }
    let json =
        serde_json::to_string_pretty(&normalize_json_value(value.clone())).map_err(serde_error)?;
    fs::write(path, format!("{json}\n")).map_err(io_error)
}

/// Read fixture.
/// This reads and parses normalized JSON from the caller-provided fixture path on disk.
pub fn read_fixture(path: impl AsRef<Path>) -> Result<Value, AgentError> {
    let json = fs::read_to_string(path).map_err(io_error)?;
    serde_json::from_str::<Value>(&json)
        .map(normalize_json_value)
        .map_err(serde_error)
}

/// Returns normalize json value for the current value.
/// This is a read-only or data-construction helper unless the method body explicitly calls a
/// port or store.
pub fn normalize_json_value(value: Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.into_iter().map(normalize_json_value).collect()),
        Value::Object(fields) => {
            let mut normalized = Map::new();
            let mut sorted = BTreeMap::new();
            for (key, value) in fields {
                sorted.insert(key, value);
            }
            for (key, value) in sorted {
                normalized.insert(key, normalize_json_value(value));
            }
            Value::Object(normalized)
        }
        scalar => scalar,
    }
}

fn io_error(error: std::io::Error) -> AgentError {
    AgentError::contract_violation(format!("fixture I/O failed: {error}"))
}

fn serde_error(error: serde_json::Error) -> AgentError {
    AgentError::contract_violation(format!("fixture JSON failed: {error}"))
}
