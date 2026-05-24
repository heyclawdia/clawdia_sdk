# Output Delivery Contract

This contract defines how final and streaming outputs leave the SDK without making the SDK own product channel UX.

## Primitive Boundary

| Primitive | Owns | Must not own |
| --- | --- | --- |
| `DestinationRef` | Logical destination identity, destination kind, correlation, privacy, retention, and host policy refs. | Product UI routing or remote transport implementation. |
| `OutputSink` | Typed port for dispatching chunks, finals, acknowledgements, failures, and reconciliation metadata. | Model execution, channel UI, or retry scheduling outside policy. |
| `OutputDeliveryRequest` | One dispatch attempt with source message/output refs, destination, dedupe key, content refs, and policy refs. | Provider transcript mutation. |
| `OutputDeliveryReceipt` | Ack/failure/dedupe result, destination cursor, and host correlation refs. | Durable channel storage policy. |
| `OutputDeliveryPolicy` | Which outputs may be dispatched, when approval is required, retry bounds, and content-capture limits. | Actual network delivery or UI copy. |
| `EffectIntent` / `EffectResult` | Shared side-effect spine for delivery intent/result, idempotency, dedupe, policy, reconciliation, and privacy. | Product retry workflow or channel storage. |

## MVP Profile

The MVP slice needs only:

- `DestinationRef` on `RunRequest`;
- an optional fake `OutputSink`;
- final-result dispatch after `RunResult` is validated;
- a deterministic dedupe key;
- journaled intent before dispatch and receipt after dispatch.

Remote channels, webhooks, files, desktop widgets, streaming chunk dispatch, offline retry queues, and host reconciliation are feature layers over the same sink contract.

## Contract Rules

- The SDK may produce output, but the host decides whether and where product channels are available.
- Side-effecting delivery appends an intent record before calling `OutputSink`.
- `OutputDeliveryIntentRecord` contains or maps one-to-one to `EffectIntent { kind: OutputDelivery }`.
- `OutputDeliveryResultRecord` contains or maps one-to-one to `EffectResult`.
- `OutputSink` absence means no external dispatch; it does not fail the run unless the request required delivery.
- Streaming chunk dispatch uses the same destination, dedupe, privacy, and journal rules as final dispatch.
- Deduplication is by typed key, not by comparing output text.
- Output delivery cannot send raw content unless the destination policy allows it.
- Output delivery failures are typed and observable; they do not rewrite the model output.
- Host channel acknowledgements are receipts, not run truth. The run journal remains the durable source for SDK behavior.
- Delivery passes through `PolicyStage::Delivery` before any externally visible sink call. The stage decision is journaled with destination, subject/related refs, policy refs, privacy/retention class, and redacted summary.

## Events And Journal Records

Events may use the reserved `output_delivery` family only when the implementing workstream emits those kinds and provides fixtures:

- `OutputDispatchRequested`
- `OutputDispatchCompleted`
- `OutputDispatchFailed`
- `OutputDispatchDeduped`

Journal records:

- `OutputDeliveryIntentRecord`
- `OutputDeliveryResultRecord`
- `OutputDeliveryDedupeRecord`
- `OutputDeliveryReconciliationRecord`

## Acceptance Tests

- `run_request_destination_does_not_require_product_channel_import`
- `missing_optional_output_sink_does_not_fail_run`
- `required_output_sink_absence_returns_host_configuration_needed`
- `output_delivery_intent_precedes_sink_call`
- `output_delivery_dedupes_by_key_not_text`
- `raw_output_dispatch_requires_destination_policy`
- `streaming_and_final_delivery_share_dedupe_and_privacy_rules`
- `output_delivery_failure_does_not_mutate_run_result`
- `policy_stage_delivery_allows_denies_or_modifies_before_sink_call`

## Complete Example

Typed shape:

```rust
// Non-compiling contract sketch.
let destination = DestinationRef::new(DestinationKind::RemoteChannel, DestinationId::new("channel.support"))
    .with_policy(PolicyRef::new("policy.output.redacted_reply"))
    .with_correlation(CorrelationKey::new("thread.example"));

let delivery = OutputDeliveryRequest {
    delivery_id: OutputDeliveryId::new(),
    run_id,
    source_message_id: final_message_id,
    destination,
    dedupe_key: DedupeKey::from_run_final(run_id, final_message_id),
    content_ref: ContentRef::message(final_message_id),
    privacy: PrivacyClass::RedactedSummary,
    policy_ref: PolicyRef::new("policy.output.redacted_reply"),
};
```

Wiring:

1. `RunRequest` names a destination and whether external delivery is optional or required.
2. SDK validates output and builds an `OutputDeliveryRequest`.
3. SDK appends `OutputDeliveryIntentRecord`.
4. SDK calls the host-provided `OutputSink`.
5. SDK appends a result, failure, or dedupe record and emits the matching event.

SDK owns / Host owns:

- SDK owns destination refs, delivery intent/result records, dedupe semantics, privacy checks, and typed failures.
- Host owns channel routing, UI copy, transport credentials, durable channel storage, offline retry scheduler, and product notification policy.
