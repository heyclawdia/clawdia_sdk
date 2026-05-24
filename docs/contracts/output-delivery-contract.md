# Output Delivery Contract

This contract defines how final and streaming outputs leave the SDK without making the SDK own product channel UX. Output delivery is a host sink effect: the SDK owns typed refs, policy, privacy, dedupe, events, and journal records; hosts own the concrete destination and transport.

## Primitive Boundary

| Primitive | Owns | Must not own |
| --- | --- | --- |
| `DestinationRef` | Logical destination identity, destination kind, correlation keys, privacy, retention, and host policy refs. | Product UI routing, notification copy, remote credentials, or channel-specific state machines. |
| `OutputSink` | Typed host port for dispatching chunks and finals, returning receipts, and reconciling pending sends. | Model execution, run-loop branching, product retry scheduling, durable channel storage, or UI rendering. |
| `OutputDeliveryPolicy` | Delivery requirement, allowed content modes, sink capability requirements, retry/reconcile bounds, and policy refs for `PolicyStage::Delivery`. | Actual network delivery, channel copy, offline product workflow, or sink credentials. |
| `OutputDeliveryRequest` | One sink call candidate with typed IDs, source refs, destination, delivery kind, content refs, redacted summary, privacy, policy refs, idempotency key, and dedupe key. | Provider transcript mutation, output validation, or channel-specific payload construction rules outside the sink. |
| `OutputDeliveryReceipt` | Host acknowledgement, failure, dedupe, destination cursor, external operation ID, and reconciliation metadata. | Durable run truth, host conversation storage, or product notification policy. |
| `EffectIntent` / `EffectResult` | Shared side-effect spine for output intent/result, idempotency, dedupe, policy, privacy, and recovery. | A private output-dispatch ledger or product retry workflow. |
| `RunJournal` / `AgentEvent` | Durable truth and live observation for output delivery records and events. | Slow sink delivery, global channel archive ownership, or product workflow orchestration. |

## MVP Profile

The MVP slice needs only:

- `DestinationRef` on `RunRequest`;
- an optional fake `OutputSink` in the resolved `RuntimePackage`;
- final-result dispatch after `PolicyStage::Output` publishes a final message or `ValidatedOutput`;
- a deterministic `DedupeKey`;
- `OutputDeliveryIntentRecord` before any externally visible sink call;
- `OutputDeliveryResultRecord` after completion, failure, dedupe, or host-configuration failure.

Remote channels, webhooks, files, desktop widgets, streaming chunk dispatch, offline retry queues, and host reconciliation are feature layers over the same sink contract. They must not introduce channel-specific run paths, event streams, journal records, or policy stages.

## Canonical Lowering

Every output-delivery helper lowers into the same canonical path:

1. `RunRequest.destination(...)` and delivery helpers produce a `DestinationRef` plus an `OutputDeliveryPolicy` selection.
2. `AgentRuntime::start_run` resolves those fields into the effective `RuntimePackage` snapshot. Output sinks and delivery policy are package fields or typed sidecars, not `CapabilitySpec` variants.
3. The agent loop creates an output-delivery candidate only after final output passes `PolicyStage::Output`.
4. `PolicyStage::Delivery` evaluates the candidate with destination, source output refs, content refs, privacy/retention class, desired sink ref, available sink capability metadata, and policy refs.
5. If delivery is required and the required sink is missing, the loop appends `OutputDeliveryIntentRecord` with the desired sink ref, appends `OutputDeliveryResultRecord` with retry classification `HostConfigurationNeeded`, and does not call a sink.
6. If delivery is allowed or modified and the sink is available, the loop appends `OutputDeliveryIntentRecord` with `EffectIntent { kind: OutputDelivery }`.
7. Only after the intent append succeeds may the loop call `OutputSink`.
8. The loop appends `OutputDeliveryResultRecord`, `OutputDeliveryDedupeRecord`, or `OutputDeliveryReconciliationRecord` and emits the corresponding journal-backed `output_delivery` event.

Simple helpers such as `agent.request(input).destination(dest).run(runtime)` and explicit `RunRequest` construction must emit equivalent policy decisions, journal records, events, dedupe keys, and typed failures.

## Typed Shapes

Non-compiling contract sketches:

```rust
pub enum OutputDeliveryRequirement {
    Disabled,
    Optional,
    Required,
}

pub enum OutputDeliveryKind {
    StreamChunk {
        stream_cursor: StreamCursor,
        chunk_index: u64,
    },
    FinalMessage,
    FinalValidatedOutput,
}

pub enum OutputContentMode {
    ContentRefsOnly,
    RedactedSummary,
    RawContentIfPolicyAllows,
}

pub struct OutputDeliveryPolicy {
    pub policy_ref: PolicyRef,
    pub requirement: OutputDeliveryRequirement,
    pub default_content_mode: OutputContentMode,
    pub allowed_content_modes: Vec<OutputContentMode>,
    pub required_sink_ref: Option<OutputSinkRef>,
    pub retry_policy_ref: Option<PolicyRef>,
    pub reconciliation_policy_ref: Option<PolicyRef>,
}

pub struct OutputDeliveryRequest {
    pub delivery_id: OutputDeliveryId,
    pub effect_id: EffectId,
    pub run_id: RunId,
    pub agent_id: AgentId,
    pub turn_id: Option<TurnId>,
    pub attempt_id: Option<AttemptId>,
    pub source_message_id: Option<MessageId>,
    pub validated_output_id: Option<ValidatedOutputId>,
    pub destination: DestinationRef,
    pub sink_ref: OutputSinkRef,
    pub delivery_kind: OutputDeliveryKind,
    pub content_refs: Vec<ContentRef>,
    pub redacted_summary: RedactedSummary,
    pub privacy: PrivacyClass,
    pub retention: RetentionClass,
    pub policy_refs: Vec<PolicyRef>,
    pub idempotency_key: Option<IdempotencyKey>,
    pub dedupe_key: DedupeKey,
}

pub trait OutputSink {
    async fn send_chunk(&self, request: OutputDeliveryRequest) -> Result<OutputDeliveryReceipt, OutputDeliveryError>;
    async fn send_final(&self, request: OutputDeliveryRequest) -> Result<OutputDeliveryReceipt, OutputDeliveryError>;
    async fn reconcile(&self, request: OutputDeliveryReconcileRequest) -> Result<OutputDeliveryReceipt, OutputDeliveryError>;
}
```

`send_chunk` and `send_final` are separate ergonomic entry points only. They receive the same `OutputDeliveryRequest` shape and must use the same policy, privacy, dedupe, journal, event, and recovery semantics. A sink may encode a destination-specific wire payload internally, but that encoding is host-owned and cannot change SDK run behavior.

## Delivery Matrix

| Configuration | SDK behavior | Terminal effect |
| --- | --- | --- |
| No destination or `Disabled` delivery | Do not create an output-delivery candidate. | Run completion depends only on normal terminal records. |
| Optional delivery and no matching sink | Do not call a sink and do not fail the run. Record a bounded redacted diagnostic in the terminal run summary or host-facing status surface. | No external effect attempted. |
| Required delivery and no matching sink | Append delivery intent/result records with the desired sink ref and `dispatch_status: HostConfigurationNeeded`; return `AgentError` with retry classification `HostConfigurationNeeded`. | Run cannot seal as successful required delivery. |
| Delivery policy denies | Do not append a delivery intent and do not call a sink. The policy decision is journaled with `PolicyStage::Delivery`. | Required delivery returns a typed policy failure; optional delivery is skipped observably. |
| Delivery policy modifies content mode | Build the request from the modified content mode, normally `ContentRefsOnly` or `RedactedSummary`. | Sink receives only policy-approved refs/summaries. |
| Dedupe key already completed | Do not call a sink. Append `OutputDeliveryDedupeRecord` and emit `OutputDispatchDeduped`. | Required delivery may complete by dedupe if policy accepts the prior receipt. |
| Sink call succeeds | Append `OutputDeliveryResultRecord` with `EffectResult::Completed` and host receipt refs. | Required delivery gate is satisfied. |
| Sink call fails, times out, or returns unknown | Append `OutputDeliveryResultRecord` with typed error or `OutputDeliveryReconciliationRecord` if the external state is unknown. | Required delivery follows retry/recovery policy; optional delivery does not rewrite the model output. |
| Result append fails after sink call | Enter recovery and block further non-idempotent side effects until reconciliation records the outcome. | Journal remains the source of truth; the sink receipt alone is not run truth. |

## Streaming And Final Delivery

Streaming chunk delivery and final delivery differ only by `OutputDeliveryKind` and source refs. They share:

- the same `DestinationRef`;
- the same `OutputSinkRef` resolution;
- the same `OutputDeliveryPolicy`;
- the same `PolicyStage::Delivery`;
- the same content-mode rules;
- the same `EffectIntent` / `EffectResult` mapping;
- the same `RunJournal` append-before-call rule;
- the same event family and payload minimums;
- the same dedupe-key schema;
- the same privacy, retention, and redaction policy.

Chunk dispatch must not become a provider-stream side channel. A provider stream delta becomes a delivery candidate only after any stream policy/redaction rules have produced the `ContentRef` or redacted summary that the delivery policy allows.

The final delivery dedupe key must be distinct from chunk keys, but it is built by the same key builder. A final delivery must never infer that chunks were delivered by comparing text or by assuming a sink rendered previous deltas.

## Dedupe And Idempotency

`DedupeKey` prevents duplicate sink calls across retries, duplicate subscribers, reconnects, and resume replay. It is typed and deterministic; it must not be derived from raw output text alone.

Minimum dedupe-key fields:

- `run_id`;
- `destination_ref` stable ID and kind;
- `sink_ref`;
- `delivery_kind`;
- source output ref: `message_id`, `validated_output_id`, or stream cursor plus chunk index;
- content ref IDs and versions, or content hashes when refs are immutable by hash;
- delivery policy ref and policy version/fingerprint;
- runtime package fingerprint;
- optional host correlation key when the destination requires source-scoped replies.

`IdempotencyKey` may be sent to a sink or remote transport when the sink supports external idempotency. `DedupeKey` remains SDK-owned and journaled even when the sink also has an external idempotency key.

`OutputDeliveryDedupeRecord` records:

- dedupe key;
- prior delivery ID or prior external operation ID when known;
- prior terminal status;
- whether the current request was skipped, completed by prior receipt, or requires reconciliation;
- redacted summary and policy refs.

## Privacy And Content Modes

Default delivery uses `ContentRef` values or bounded `RedactedSummary` values. Raw content is never sent by default.

Raw or expanded content can be sent only when all of these are true:

- `PolicyStage::Output` allowed the output to be published;
- `PolicyStage::Delivery` allows raw or expanded content for this destination;
- the `OutputSink` declares that it may receive that content mode;
- content-capture policy names retention, redaction policy, and allowed sink;
- the journal/event record can still be useful with raw content elided.

Privacy matrix:

| Requested content mode | Destination policy | Sink capability | SDK result |
| --- | --- | --- | --- |
| default | any allowed destination | refs or summaries supported | send refs or redacted summary only |
| raw requested, policy denies | denied | any | no raw send; required delivery fails or optional delivery downgrades/skips by policy |
| raw requested, policy allows, sink denies | allowed | raw unsupported | `HostConfigurationNeeded` or policy-modified refs, depending on requirement |
| content ref requested, sink cannot resolve refs | allowed | refs unsupported | required delivery fails with `HostConfigurationNeeded`; optional delivery skips observably |
| redacted summary requested | allowed | summaries supported | send bounded summary with content refs for audit |

Events, journals, telemetry, and receipts must use IDs, hashes, sizes, MIME/part kinds, policy refs, destination refs, delivery status, and bounded summaries by default. They must not copy raw user text, model output, hidden reasoning, tool output, memory bodies, files, credentials, or auth headers unless an explicit content-capture policy allows it.

## Events And Journal Records

Events may use the reserved `output_delivery` family only when the implementing workstream emits those kinds and provides per-kind golden fixtures:

- `OutputDispatchRequested`
- `OutputDispatchCompleted`
- `OutputDispatchFailed`
- `OutputDispatchDeduped`

Every `output_delivery` event uses the standard envelope:

- `subject_ref = EntityRef::OutputDelivery(delivery_id)`;
- `related_refs` include the source message or validated output, content refs, and effect refs; `destination` is also present on the event envelope;
- `causal_refs` link to the model/structured-output/policy decision that produced the candidate;
- `destination = Some(DestinationRef)`;
- `policy_refs` include `PolicyStage::Output` and `PolicyStage::Delivery` decisions when applicable;
- `delivery_semantics = journal_backed` after the journal append succeeds;
- payload fields include the minimum event-schema fields: destination, dedupe key, source message ID, dispatch status, ack ref, and reconciliation status.

Journal records:

- `OutputDeliveryIntentRecord`: wraps or maps one-to-one to `EffectIntent { kind: OutputDelivery }` and includes destination, sink ref or desired sink ref, delivery kind, content refs, redacted summary, policy refs, privacy/retention, idempotency key, dedupe key, and runtime package fingerprint.
- `OutputDeliveryResultRecord`: wraps or maps one-to-one to `EffectResult` and includes terminal status, typed error/ref, receipt refs, external operation ID, content refs, redacted summary, and retry classification.
- `OutputDeliveryDedupeRecord`: records skipped duplicate sends and the prior receipt/reconciliation state.
- `OutputDeliveryReconciliationRecord`: records unknown external outcomes, host ack lookup refs, repair/retry decision, and unsafe pending reason when terminal result could not be confirmed.

`OutputDispatchRequested` is emitted only after `OutputDeliveryIntentRecord` is durably appended. `OutputDispatchCompleted`, `OutputDispatchFailed`, and `OutputDispatchDeduped` are emitted only after the matching terminal journal record exists. If journal append fails before a required delivery side effect, the SDK fails closed and does not call `OutputSink`.

## Completion And Recovery

Required delivery is terminal bookkeeping. `RunHandle::wait()` cannot resolve successful completion until required output delivery has completed, deduped, failed according to policy, or entered a typed recovery state and the run journal has sealed the resulting terminal status.

Rules:

- Optional delivery failures are observable but do not rewrite the final message, `ValidatedOutput`, or typed result.
- Required delivery failures do not mutate output content; they affect terminal delivery status and may make the run return a typed delivery error.
- Resume replay never re-executes a delivery unless the dedupe key, idempotency key, and reconciliation policy classify the retry as safe.
- Repair replay can rebuild delivery indexes and telemetry projections but must not call `OutputSink`.
- Anti-entropy may compare journaled delivery state against host receipt metadata and emit host action needed, but it cannot send, unsend, or compensate product messages itself.

## Host-Owned Boundaries

SDK owns:

- `DestinationRef`, output delivery IDs, effect IDs, dedupe keys, idempotency keys, policy refs, privacy/retention refs, and content refs;
- `OutputDeliveryPolicy` and canonical lowering from helpers;
- delivery intent/result/dedupe/reconciliation journal records;
- output-delivery event family/kind usage and payload minimums;
- typed failures such as `HostConfigurationNeeded`;
- replay and recovery rules that prevent duplicate side effects.

Host owns:

- channel routing, UI rendering, notification copy, credentials, webhooks, remote-channel APIs, file handles, terminal output devices, and desktop surfaces;
- sink capability declarations and concrete wire payload encoding;
- durable channel stores, ack lookup, offline retry scheduler, and product compensation workflows;
- user-facing copy for delivery failures and approvals;
- product policy deciding whether a delivery failure is surfaced as blocking, warning, or follow-up.

## Integration Handoff Values

Destination kinds should reuse the stable `DestinationRef.kind` vocabulary from the event schema. Output delivery should prefer `output_sink` as the SDK-facing destination when a host sink is the immediate target, with `ui`, `remote_channel`, `cli` via `other:<namespace>/<kind>`, `session`, or `external_runtime` used only as destination metadata under the same `DestinationRef` rules.

Output delivery event names:

- `OutputDispatchRequested`
- `OutputDispatchCompleted`
- `OutputDispatchFailed`
- `OutputDispatchDeduped`

Journal record names:

- `OutputDeliveryIntentRecord`
- `OutputDeliveryResultRecord`
- `OutputDeliveryDedupeRecord`
- `OutputDeliveryReconciliationRecord`

Required future fixture list:

- `events/output_delivery_requested_v1.json`
- `events/output_delivery_completed_v1.json`
- `events/output_delivery_failed_v1.json`
- `events/output_delivery_deduped_v1.json`
- `journal/output_delivery_intent_v1.json`
- `journal/output_delivery_result_v1.json`
- `journal/output_delivery_dedupe_v1.json`
- `journal/output_delivery_reconciliation_v1.json`

## Acceptance Tests

Future tests and audits:

- `run_request_destination_does_not_require_product_channel_import`
- `run_level_destination_helper_lowers_to_destination_ref_and_delivery_policy`
- `request_builder_and_explicit_delivery_request_emit_equivalent_records`
- `missing_optional_output_sink_does_not_fail_run`
- `required_output_sink_absence_returns_host_configuration_needed`
- `output_delivery_intent_precedes_sink_call`
- `journal_append_failure_prevents_output_sink_call`
- `output_delivery_failure_records_terminal_result`
- `output_delivery_dedupes_by_key_not_text`
- `dedupe_record_prevents_duplicate_sink_call`
- `raw_output_dispatch_requires_destination_policy`
- `raw_output_dispatch_requires_allowed_sink_capability`
- `default_output_delivery_uses_content_refs_or_redacted_summary`
- `streaming_and_final_delivery_share_dedupe_and_privacy_rules`
- `stream_chunk_delivery_uses_same_effect_intent_shape_as_final_delivery`
- `output_delivery_failure_does_not_mutate_run_result`
- `policy_stage_delivery_allows_denies_or_modifies_before_sink_call`
- `required_delivery_gates_run_completion_until_terminal_record`
- `resume_replay_uses_dedupe_before_resending_output`
- `repair_replay_does_not_call_output_sink`
- `output_delivery_event_golden_payload_exists_for_each_emitted_kind`
- `output_delivery_journal_fixture_exists_for_each_record_kind`
- `destination_privacy_matrix_denies_raw_content_by_default`
- `host_owned_channel_boundary_audit_has_no_product_specific_run_path`

## Complete Example

Typed shape:

```rust
// Non-compiling contract sketch.
let destination = DestinationRef::new(DestinationKind::OutputSink, DestinationId::new("sink.reply"))
    .with_policy(PolicyRef::new("policy.output.redacted_reply"))
    .with_correlation(CorrelationKey::new("thread.example"));

let delivery_policy = OutputDeliveryPolicy {
    policy_ref: PolicyRef::new("policy.output.redacted_reply"),
    requirement: OutputDeliveryRequirement::Required,
    default_content_mode: OutputContentMode::ContentRefsOnly,
    allowed_content_modes: vec![OutputContentMode::ContentRefsOnly, OutputContentMode::RedactedSummary],
    required_sink_ref: Some(OutputSinkRef::new("host.remote_reply")),
    retry_policy_ref: Some(PolicyRef::new("policy.output.retry_once_with_dedupe")),
    reconciliation_policy_ref: Some(PolicyRef::new("policy.output.lookup_ack_by_dedupe")),
};

let delivery = OutputDeliveryRequest {
    delivery_id: OutputDeliveryId::new(),
    effect_id: EffectId::new(),
    run_id,
    agent_id,
    turn_id: Some(turn_id),
    attempt_id: Some(attempt_id),
    source_message_id: Some(final_message_id),
    validated_output_id: None,
    destination,
    sink_ref: OutputSinkRef::new("host.remote_reply"),
    delivery_kind: OutputDeliveryKind::FinalMessage,
    content_refs: vec![ContentRef::message(final_message_id)],
    redacted_summary: RedactedSummary::new("final assistant reply"),
    privacy: PrivacyClass::RedactedSummary,
    retention: RetentionClass::Conversation,
    policy_refs: vec![delivery_policy.policy_ref.clone()],
    idempotency_key: Some(IdempotencyKey::from_dedupe_key(&dedupe_key)),
    dedupe_key: DedupeKey::from_fields(OutputDeliveryDedupeFields {
        run_id,
        destination_ref: destination.stable_ref(),
        sink_ref: OutputSinkRef::new("host.remote_reply"),
        delivery_kind: OutputDeliveryKindDiscriminant::FinalMessage,
        source_ref: OutputSourceRef::Message(final_message_id),
        content_refs: vec![ContentRef::message(final_message_id)],
        policy_ref: delivery_policy.policy_ref.clone(),
        runtime_package_fingerprint,
    }),
};
```

Wiring:

1. `RunRequest` names a destination and whether external delivery is optional or required.
2. Runtime package resolution captures output sink IDs, sink capability versions, delivery policy refs, and dedupe policy in the package fingerprint.
3. SDK validates and publishes output through `PolicyStage::Output`.
4. SDK runs `PolicyStage::Delivery` for the delivery candidate.
5. SDK appends `OutputDeliveryIntentRecord`.
6. SDK emits `OutputDispatchRequested` with a journal cursor.
7. SDK calls the host-provided `OutputSink`.
8. SDK appends result, failure, dedupe, or reconciliation records and emits the matching event.

SDK owns / Host owns:

- SDK owns destination refs, delivery policy lowering, intent/result records, dedupe semantics, privacy checks, events, journal records, recovery rules, and typed failures.
- Host owns channel routing, UI copy, transport credentials, durable channel storage, offline retry scheduling, ack lookup implementation, and product notification policy.
