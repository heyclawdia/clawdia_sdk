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

pub const FIXTURE_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Debug)]
pub struct FakeFixtureHarness {
    pub deterministic_seed: u64,
    pub ids: DeterministicIdGenerator,
    pub clock: DeterministicClock,
    pub content_store: FakeContentStore,
    pub journal_store: FakeJournalStore,
    pub event_sink: FakeEventSink,
    pub provider: FakeProvider,
}

impl FakeFixtureHarness {
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
pub struct DeterministicIdGenerator {
    seed: u64,
    next: Cell<u64>,
}

impl DeterministicIdGenerator {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            next: Cell::new(0),
        }
    }

    pub fn next_raw(&self, prefix: &str) -> String {
        let seq = self.next.get();
        self.next.set(seq + 1);
        format!("{prefix}.{:04}.{:04}", self.seed, seq)
    }

    pub fn next_content_ref(&self) -> ContentId {
        ContentId::new(self.next_raw("content"))
    }
}

#[derive(Clone, Debug)]
pub struct DeterministicClock {
    start_millis: u64,
    step_millis: u64,
    ticks: Cell<u64>,
}

impl DeterministicClock {
    pub fn new(seed: u64) -> Self {
        Self {
            start_millis: seed,
            step_millis: 1,
            ticks: Cell::new(0),
        }
    }

    pub fn next_millis(&self) -> u64 {
        let ticks = self.ticks.get();
        self.ticks.set(ticks + 1);
        self.start_millis + ticks * self.step_millis
    }
}

#[derive(Clone, Debug, Default)]
pub struct FakeContentStore {
    entries: Arc<Mutex<BTreeMap<ContentId, StoredContent>>>,
}

impl FakeContentStore {
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

    pub fn get(&self, content_ref: &ContentId) -> Option<StoredContent> {
        self.entries
            .lock()
            .expect("content store lock")
            .get(content_ref)
            .cloned()
    }

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
pub struct StoredContent {
    pub media_type: String,
    pub bytes: Vec<u8>,
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StoredContentManifestEntry {
    pub content_ref: String,
    pub media_type: String,
    pub byte_len: usize,
    pub redacted_summary: String,
}

#[derive(Clone, Debug, Default)]
pub struct FakeJournalStore {
    records: Arc<Mutex<Vec<JournalRecord>>>,
    fail_next_append: Arc<Mutex<Option<String>>>,
}

impl FakeJournalStore {
    pub fn records(&self) -> Vec<JournalRecord> {
        self.records.lock().expect("journal store lock").clone()
    }

    pub fn normalized_records(&self) -> Vec<Value> {
        self.records()
            .into_iter()
            .map(|record| normalize_json_value(serde_json::to_value(record).expect("record JSON")))
            .collect()
    }

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
pub struct FakeEventSink {
    frames: Arc<Mutex<Vec<EventFrame>>>,
}

impl FakeEventSink {
    pub fn emit(&self, frame: EventFrame) {
        self.frames.lock().expect("event sink lock").push(frame);
    }

    pub fn frames(&self) -> Vec<EventFrame> {
        self.frames.lock().expect("event sink lock").clone()
    }

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
pub struct FakeProvider {
    responses: Arc<Mutex<Vec<String>>>,
    requests: Arc<Mutex<Vec<ProviderRequest>>>,
}

impl FakeProvider {
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
            usage: Some(ProviderUsage {
                input_tokens: Some(input_tokens),
                output_tokens: Some(output_tokens),
                total_tokens: Some(input_tokens + output_tokens),
            }),
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FixtureManifest {
    pub schema_version: u16,
    pub fixture_name: String,
    pub redaction: String,
    pub entries: Vec<FixtureManifestEntry>,
}

impl FixtureManifest {
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
pub struct FixtureManifestEntry {
    pub path: String,
    pub contract: String,
    pub schema_version: u16,
}

pub fn write_fixture(path: impl AsRef<Path>, value: &Value) -> Result<(), AgentError> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(io_error)?;
    }
    let json =
        serde_json::to_string_pretty(&normalize_json_value(value.clone())).map_err(serde_error)?;
    fs::write(path, format!("{json}\n")).map_err(io_error)
}

pub fn read_fixture(path: impl AsRef<Path>) -> Result<Value, AgentError> {
    let json = fs::read_to_string(path).map_err(io_error)?;
    serde_json::from_str::<Value>(&json)
        .map(normalize_json_value)
        .map_err(serde_error)
}

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
