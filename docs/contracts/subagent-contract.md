# Subagent Contract

Subagents are parent-owned child runs. They lower into `RunRequest`, stripped child
`RuntimePackage` snapshots, `RunJournal`, `AgentEvent`, `PolicyRef`, `ContentRef`,
usage/cost records, and typed refs. They are not user-chatable conversations unless
a host explicitly promotes them outside the core SDK contract.

This is a feature-layer primitive over the kernel. It must not create a second run
loop, package registry, event stream, journal, policy path, context pipeline,
runtime ledger, recursive agent societies, or product UI.

## External Lessons

- Strands has multi-agent lifecycle and node stream events. The SDK should adopt explicit lineage and events.
- Cursor names subagents with prompts/models, but product routing stays outside the core run.
- The SDK requires stricter safety than free-form agent societies: no direct user chat, no recursive subagent tools by default, route validation, parent-owned cancellation, and child usage rollup.

## Public Shape And Canonical Lowering

```rust
// Non-compiling contract sketch.
pub struct SubagentSupervisor {
    topology: AgentTopology,
    depth_budget: DepthBudget,
    route_policy: SubagentRoutePolicy,
    default_handoff_policy: ContextHandoffPolicy,
    child_package_policy: ChildRuntimePackagePolicy,
    mailbox_policy: SubagentMailboxPolicy,
    lifecycle_policy: RunChildLifecyclePolicy,
}

impl SubagentSupervisor {
    pub async fn start_child(
        &self,
        parent: &RunContext,
        request: SubagentRequest,
    ) -> Result<ChildRunHandle, AgentError>;

    pub async fn cancel_child(&self, child_run_id: RunId) -> Result<(), AgentError>;
    pub async fn send_parent_message(&self, message: SubagentParentMessage) -> Result<ParentMessageReceipt, AgentError>;
    pub async fn ask_parent_for_clarification(&self, request: SubagentClarificationRequest) -> Result<SubagentClarificationResponse, AgentError>;
    pub async fn answer_child_clarification(&self, reply: SubagentClarificationReply) -> Result<(), AgentError>;
    pub async fn read_parent_messages(&self, fetch: SubagentParentMessageFetch) -> Result<Vec<SubagentParentMessage>, AgentError>;
    pub fn wrap_child_event(&self, event: AgentEvent) -> Result<AgentEvent, AgentError>;
}

pub struct SubagentRequest {
    pub request_id: SubagentRequestId,
    pub parent_run_id: RunId,
    pub parent_tool_call_id: ToolCallId,
    pub child_run_id: RunId,
    pub child_agent_id: AgentId,
    pub child_source: SourceRef,
    pub child_destination: DestinationRef,
    pub route_policy: SubagentRoutePolicy,
    pub context_handoff: ContextHandoffPolicy,
    pub child_package_policy: ChildRuntimePackagePolicy,
    pub child_tool_policy: SubagentToolPolicy,
    pub mailbox_policy: SubagentMailboxPolicy,
    pub lifecycle_policy_ref: Option<RunChildLifecyclePolicyRef>,
    pub depth_budget: DepthBudget,
    pub idempotency_key: IdempotencyKey,
}

pub struct ChildRunHandle {
    pub child_run_id: RunId,
    pub child_agent_id: AgentId,
    pub parent_run_id: RunId,
    pub child_package_fingerprint: RuntimePackageFingerprint,
    pub child_journal_ref: RunJournalRef,
    pub wrapped_event_filter: EventFilter,
}

pub enum ContextHandoffPolicy {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "summary_only")]
    SummaryOnly {
        summary_ref: Option<ContentRef>,
        max_tokens: u32,
        policy_ref: PolicyRef,
    },
    #[serde(rename = "selected_refs")]
    SelectedRefs {
        refs: Vec<ContentRef>,
        policy_ref: PolicyRef,
    },
    #[serde(rename = "full_history_with_policy")]
    FullHistoryWithPolicy {
        policy_ref: PolicyRef,
        projection_audit_required: bool,
    },
}

pub struct ChildRuntimePackagePolicy {
    pub source_parent_package: RuntimePackageFingerprint,
    pub inherit_provider_route: RouteInheritanceMode,
    pub allowed_route_overrides: Vec<ProviderRouteRef>,
    pub strip_recursive_subagents: bool,
    pub strip_disallowed_tools: bool,
    pub child_lifecycle_bounds: RunChildLifecyclePolicyRef,
    pub redaction_policy_ref: PolicyRef,
}

pub struct SubagentMailboxPolicy {
    pub max_parent_messages: u32,
    pub read_scope: ParentMailboxReadScope,
    pub clarification_timeout_ms: u64,
    pub parent_reply_policy_ref: PolicyRef,
}
```

`ContextHandoffPolicy::None` is the default. It passes no parent transcript,
messages, summaries, or selected refs into the child. `summary_only`,
`selected_refs`, and `full_history_with_policy` are explicit opt-ins that still
flow through `ContextContribution`, `ContextAssembler`, `ContextItem`, and
`ContextProjection`; they do not inject raw history or bypass policy.

`SubagentSupervisor::start_child` is the canonical lowering point:

1. Validate topology, depth, max child count, cycle prevention, route policy,
   child lifecycle policy, and mailbox policy.
2. Build a child `RuntimePackage` snapshot from the parent package by stripping
   recursive subagent capabilities and disallowed tools, then validating every
   provider-visible capability has an executor and policy ref.
3. Convert allowed handoff content into `ContextContribution` candidates. Only
   policy-admitted items become child `ContextItem` values.
4. Create a child `RunRequest` with `source = SourceRef::subagent(parent_run_id)`,
   `destination = DestinationRef::child_agent(child_run_id)`, and the stripped
   child package ref.
5. Append the child-start `EffectIntent` and subagent journal records before the
   child run starts.
6. Start the child through `AgentRuntime::start_run` or a host-provided child-run
   adapter that honors the same `RunRequest`, package, journal, event, policy,
   and cancellation contracts.

Helpers such as `Agent::as_tool` and `RunContext::spawn_child` are only thin
lowering layers into `CapabilityKind::AgentAsTool` sidecars and
`SubagentRequest`. They must emit the same events, journal records, policy
checks, telemetry projections, failures, package fingerprints, and lifecycle
records as an explicit advanced request.

The SDK parent-control tool shape:

- `subagent_send_message`
- `subagent_reply_to_clarification`
- `subagent_ask_parent`
- `subagent_read_parent_messages`

The SDK contract must preserve the capability while making the authority boundary explicit. A child can ask the parent for clarification and read scoped parent messages, but it does not become a user-chatable session and does not gain ambient access to the parent transcript.

## Parent Message And Clarification Primitives

```rust
// Non-compiling contract sketch.
pub struct SubagentParentMessage {
    pub parent_message_id: ParentMessageId,
    pub parent_run_id: RunId,
    pub child_run_id: RunId,
    pub parent_tool_call_id: ToolCallId,
    pub source: SourceRef,
    pub destination: DestinationRef,
    pub content_ref: ContentRef,
    pub visibility: ParentMessageVisibility,
    pub policy_refs: Vec<PolicyRef>,
    pub idempotency_key: IdempotencyKey,
    pub created_at: Timestamp,
}

pub struct SubagentClarificationRequest {
    pub clarification_id: ClarificationId,
    pub parent_run_id: RunId,
    pub child_run_id: RunId,
    pub parent_tool_call_id: ToolCallId,
    pub agent_name: String,
    pub question_ref: ContentRef,
    pub policy_refs: Vec<PolicyRef>,
    pub idempotency_key: IdempotencyKey,
    pub timeout_ms: u64,
}

pub struct SubagentClarificationResponse {
    pub clarification_id: ClarificationId,
    pub parent_run_id: RunId,
    pub child_run_id: RunId,
    pub response_status: ClarificationResponseStatus,
    pub answer_ref: Option<ContentRef>,
    pub actor: Option<ActorRef>,
    pub policy_refs: Vec<PolicyRef>,
}

pub struct SubagentClarificationReply {
    pub clarification_id: ClarificationId,
    pub parent_run_id: RunId,
    pub child_run_id: RunId,
    pub parent_tool_call_id: ToolCallId,
    pub answer_ref: ContentRef,
    pub actor: ActorRef,
    pub policy_refs: Vec<PolicyRef>,
    pub idempotency_key: IdempotencyKey,
}
```

Rules:

- Parent messages are source-scoped to a parent tool call or explicit host action.
- Child reads require an explicit parent mailbox policy. Selected refs are handoff
  content, not mailbox authorization. The default handoff is isolated child
  context with `ContextHandoffPolicy::None`.
- A child clarification request pauses only the child step unless parent policy escalates it.
- Parent answers are delivered as child context items with lineage, not as direct user messages.
- Clarification IDs are unique per parent run and stable across resume.
- Duplicate parent messages, reads, clarification requests, and replies with the
  same idempotency key are deduped.
- Parent-message content uses content refs; raw bodies are omitted from events unless content-capture policy explicitly allows them.
- Clarification question and answer bodies use `ContentRef`; live events and OTel
  projections carry refs, bounded summaries, policy refs, privacy, retention, and
  redaction policy IDs by default.
- The parent mailbox is not a user-chat transport. It is a parent-owned control
  surface addressed by typed parent/child refs and replayed from `RunJournal`.
- A parent may deny, narrow, summarize, or redact a child handoff or clarification
  reply. Denial is journaled as a typed policy outcome and returns a typed child
  result, not provider narrative promotion.

## Child Package Rules

Child packages are stripped `RuntimePackage` snapshots. They are not mutable views
of the parent runtime and they are not a separate package registry.

Rules:

- Start from the parent effective `RuntimePackage` snapshot, not ambient runtime
  state.
- Strip `CapabilityKind::AgentAsTool`, `subagent_*` parent-control tools, child
  subagent definitions, and any other recursive subagent-spawning route by
  default.
- Keep `strip_recursive_subagents = true` as the default. Setting it to false
  is outside the core contract; a future optional orchestration layer would need
  a separate proposal with explicit policy refs, max-depth limits, cycle
  prevention, fixtures, and stitching approval before core validation may accept
  such a package.
- Validate provider/model route against package-declared and host-configured
  routes. Unknown or disallowed child routes fail closed before journaled start.
- Inherit the parent route unless override is explicit, valid, and recorded in
  the child package fingerprint inputs.
- Apply child handoff policy: `none`, `summary_only`, `selected_refs`, or
  `full_history_with_policy`.
- Full parent history is never the default. It requires
  `ContextHandoffPolicy::FullHistoryWithPolicy`, a policy ref that survives
  package validation, projection audit records, redaction policy, retention class,
  and content-capture policy review.
- Apply child tool policy: inherited allowlist, read-only, no tools, or custom
  allowlist. A child tool policy can only narrow or explicitly policy-select from
  parent package capabilities; it cannot broaden ambient power.
- Preserve required policies, content/ref resolvers, output contracts, isolation
  requirements, telemetry policy, redaction refs, and child lifecycle bounds that
  the child run needs.
- Child package gets its own fingerprint linked to the parent package
  fingerprint.

Child package fingerprint inputs:

| Input | Requirement |
| --- | --- |
| parent package fingerprint | always included as lineage, not as a substitute for child canonical fields |
| child agent ID/version | included |
| route policy and selected provider route | included |
| `ContextHandoffPolicy` variant and policy refs | included; selected ref IDs are included, raw content is not |
| child tool policy and retained capability IDs | included |
| recursive subagent strip manifest | included |
| mailbox policy | included when mailbox tools are available |
| lifecycle policy ref and detach bounds | included |
| isolation requirements | included when child execution requests isolation |
| redaction/content-capture/retention refs | included |

Package-diff fixture names for future implementation:

- `packages/subagent_child_package_diff_strips_recursive_tools_v1.json`
- `packages/subagent_child_package_diff_none_handoff_v1.json`
- `packages/subagent_child_package_diff_selected_refs_v1.json`
- `packages/subagent_child_package_diff_full_history_policy_v1.json`

## Child Lifecycle Rules

- Subagent runs are parent-owned child artifacts by default.
- Child lifecycle policy is `RunChildLifecyclePolicy` selected or tightened by
  `RunRequest` within the bounds of the effective `RuntimePackage`.
- Parent manual cancel appends child shutdown intent before cancelling the child run.
- Parent normal completion requires each child run to be terminal, rolled up, or explicitly detached under policy.
- A child cannot outlive its parent by accident. Detachment requires explicit policy, user/host intent when configured, host acknowledgement, and `ChildLifecycleRecord` entries.
- Detached child runs remain linked to the parent run through lineage and reclaim policy, but the host or optional orchestration layer owns continued supervision.
- Child lifecycle policy cannot grant recursive subagent tools or broader tool access than the child package policy allows.
- `BeforeSubagentStart` hooks can deny or narrow a child request through typed response only; they cannot silently create untracked child processes.
- Parent completion may not seal successful terminal state while a non-detached
  child is running, unreconciled, or missing usage rollup.
- Child terminal state is recorded in both the child run journal and the parent
  subagent record. The child journal is an SDK `RunJournal` partition/ref linked
  by parent/child IDs, not a separate runtime ledger.
- Detached child runs move lifecycle ownership through explicit detach records.
  Core records the transfer and reclaim policy; host or optional workflow layers
  own any detached inspector, scheduler, or continued supervision UX.
- Cancellation and detach must preserve idempotency keys so replay can reconcile
  crash windows without double-cancelling, double-detaching, or double-counting
  usage.

## Event And Journal Rules

- Parent appends `EffectIntent { kind: ChildAgentStart }` and `SubagentStartedRecord` before child starts.
- Child events are wrapped as `SubagentEvent` with parent run ID, child run ID,
  child agent ID, child package fingerprint, handoff policy, policy refs,
  source/destination refs, and the original child event kind.
- Parent appends `SubagentParentMessageSent` before delivering a parent message to a child.
- Child reads append `SubagentParentMessageRead` with message IDs and read cursor, not raw bodies.
- Child clarification requests append `SubagentClarificationRequested` before waiting.
- Parent replies append `SubagentClarificationResponded` before delivery to the child.
- Child journal is a normal SDK `RunJournal` for the child run, linked to the
  parent by typed refs. It is not a separate runtime ledger and cannot introduce
  facts absent from SDK journal records.
- Parent appends `SubagentUsageRolledUp` exactly once per child terminal usage
  unit. Duplicate subscribers must not duplicate child runs, usage records, or
  cost records.
- Child terminal completion appends `EffectResult` for the child start effect before parent completion can seal.
- Parent cancellation appends child shutdown intent and cancels child by default.
- Parent completion appends either child terminal/rollup records or explicit detach records before sealing.
- Child failure returns a typed tool result or parent failure according to policy.
- Parent and child events are filterable by envelope/index fields plus
  `EntityRef::SubagentRun`; raw payload parsing is not required for routing.
- Default subagent events use `ContentCaptureMode::Off`,
  `EventPrivacy::DefaultRedacted`, `ContentRef`, redacted summaries, policy refs,
  privacy/retention classes, and redaction policy IDs. Raw parent or child
  transcript content is opt-in only under telemetry/privacy policy.
- OTel projections derive from these events, child/parent journal records, and
  usage/cost records. OTel exporter state cannot decide child terminal status or
  replace child/parent journal truth.

Journal records:

| Record | Required fields |
| --- | --- |
| `SubagentStartedRecord` | parent run ID, child run ID, parent tool call ID, child package fingerprint, handoff policy, tool policy |
| `SubagentHandoffRecord` | handoff policy variant, contribution IDs, selected content refs, projection audit ref, policy refs, redaction policy ID |
| `SubagentWrappedEventRecord` | parent run ID, child run ID, child agent ID, original child event ID/kind, wrapped event ID, child journal cursor, privacy |
| `SubagentParentMessageRecord` | parent message ID, parent run ID, child run ID, content ref, visibility, delivery status, idempotency key |
| `SubagentParentMessageReadRecord` | child run ID, read cursor, message IDs returned, policy ref |
| `SubagentClarificationRequestedRecord` | clarification ID, parent/child run IDs, parent tool call ID, question ref, timeout |
| `SubagentClarificationRespondedRecord` | clarification ID, actor ref, answer ref, delivery status, idempotency key |
| `SubagentUsageRolledUpRecord` | child usage ref, parent usage ref, cost/currency, terminal status |
| `SubagentCompletedRecord` | child terminal status, result ref, error ref, policy outcome |
| `ChildLifecycleRecord` | child shutdown intent/result or detach intent/ack/reclaim policy |

`SubagentStartedRecord`, `SubagentCompletedRecord`, and `ChildLifecycleRecord` must embed or map one-to-one to the shared effect fields. Child start uses `EffectKind::ChildAgentStart`; parent-driven shutdown uses `EffectKind::ChildArtifactShutdown`; detach uses `EffectKind::DetachTransfer`. Feature-specific records can add child package, mailbox, clarification, and usage fields, but they cannot bypass intent-before-effect or terminal `EffectResult`.

## Phase 05 Emitted-Kind And Redaction Matrix

Phase 05c may activate only the subagent event kinds that have per-kind payload
fixtures and redaction cases. These names are the handoff to stitching for
closing the Phase 04 OTel subagent deferral.

| Workstream adapter | Emitted event kind | Future fixture name | Default redaction case |
| --- | --- | --- | --- |
| fake subagent runner | `SubagentStarted` | `events/subagent_started_v1.json` | child package fingerprint, policy refs, and summary only |
| fake subagent runner | `SubagentHandoff` | `events/subagent_handoff_none_v1.json` | proves `none` includes no parent content refs |
| fake subagent runner | `SubagentHandoff` | `events/subagent_handoff_summary_only_v1.json` | summary `ContentRef`, counts, and policy refs only |
| fake subagent runner | `SubagentHandoff` | `events/subagent_handoff_selected_refs_v1.json` | selected `ContentRef` IDs only, no raw content |
| fake subagent runner | `SubagentHandoff` | `events/subagent_handoff_full_history_policy_v1.json` | projection audit ref, redaction policy ID, no raw transcript |
| fake subagent runner | `SubagentEvent` | `events/subagent_event_wrapped_child_event_v1.json` | child event envelope refs and redacted child summary only |
| fake subagent runner | `SubagentParentMessageSent` | `events/subagent_parent_message_sent_v1.json` | parent message `ContentRef`, visibility, policy refs only |
| fake subagent runner | `SubagentParentMessageRead` | `events/subagent_parent_message_read_v1.json` | read cursor and message IDs only |
| fake subagent runner | `SubagentClarificationRequested` | `events/subagent_clarification_requested_v1.json` | question `ContentRef`, timeout, policy refs only |
| fake subagent runner | `SubagentClarificationResponded` | `events/subagent_clarification_responded_v1.json` | answer `ContentRef`, actor ref, policy refs only |
| fake subagent runner | `SubagentCompleted` | `events/subagent_completed_v1.json` | result/error refs and terminal status only |
| fake subagent runner | `SubagentFailed` | `events/subagent_failed_v1.json` | typed error ref and retry classification only |
| fake subagent runner | `SubagentCancelled` | `events/subagent_cancelled_v1.json` | cancellation reason, lifecycle refs, no transcript |
| fake subagent runner | `SubagentUsageRolledUp` | `events/subagent_usage_rolled_up_v1.json` | usage/cost refs, child run ID, no raw content |

Child lifecycle event kinds used by subagent cancellation and detach remain owned
by the child-lifecycle/isolation stitching path. Subagents reference them through
`ChildLifecycleRecord` and `EntityRef::ChildArtifact`; they do not rename those
kinds.

OTel projection inputs for stitching:

- `SubagentStarted` opens or links a child run span under the parent run span.
- `SubagentHandoff` adds span events with handoff policy variant, counts,
  selected content-ref count, projection audit ref, and redaction policy ID.
- `SubagentEvent` links child event IDs/kinds and child journal cursors without
  exporting raw child payload content by default.
- `SubagentCompleted`, `SubagentFailed`, and `SubagentCancelled` close the child
  span from journal-backed terminal state.
- `SubagentUsageRolledUp` emits usage/cost metrics linked to child and parent
  usage refs; rollup must be idempotent.
- Default attributes use existing `agent_sdk.*` fields plus
  `agent_sdk.subagent.child_run_id`. Any additional parent/child attribute names
  should be accepted by stitching before the OTel contract changes.

## Promotion Rule

Child transcript is not a top-level conversation by default. Host promotion requires:

- explicit host policy
- new conversation ID
- audit link to parent/child run IDs
- privacy review of child context
- host-owned storage and routing outside `agent-sdk-core`
- no mutation of the child `RunJournal` to make it look like a root user chat

## Compatibility Boundary

Provider narrative text about "subagents" is not an SDK child run. External runtime compatibility notifications can be observed as external runtime events but cannot bypass `SubagentSupervisor`.

The core SDK does not own:

- direct user-chat promotion for child agents
- product subagent inspector UI
- a separate child runtime ledger
- recursive agent societies
- provider narrative promotion
- detached-child schedulers or dashboards

## Acceptance Tests

- `child_package_strips_subagent_tools`
- `child_package_diff_records_recursive_strip_manifest`
- `child_package_fingerprint_includes_handoff_tool_lifecycle_and_redaction_policy`
- `child_cannot_be_addressed_as_normal_chat`
- `parent_cancel_cancels_child`
- `child_usage_rollup_preserves_child_run_id`
- `child_usage_rollup_is_idempotent_across_duplicate_subscribers`
- `unknown_child_provider_route_fails_closed`
- `extension_subagent_name_is_namespaced`
- `provider_narrative_subagent_text_is_ignored`
- `child_transcript_promotion_requires_host_policy`
- `default_context_handoff_is_none`
- `none_handoff_passes_no_parent_context`
- `summary_only_handoff_requires_policy_and_content_ref`
- `selected_refs_handoff_requires_policy_and_content_refs`
- `full_history_handoff_requires_policy_projection_audit_and_redaction`
- `child_can_ask_parent_without_becoming_user_chat`
- `clarification_round_trip_is_parent_owned`
- `duplicate_clarification_reply_is_deduped`
- `child_read_parent_messages_is_policy_scoped`
- `parent_message_events_use_content_refs_by_default`
- `subagent_parent_message_events_are_wrapped`
- `subagent_clarification_records_replay_after_resume`
- `agent_as_tool_lowers_to_subagent_request`
- `simple_spawn_and_explicit_subagent_request_emit_equivalent_events`
- `subagent_request_lowers_to_child_run_request`
- `wrapped_child_event_uses_subagent_entity_refs`
- `child_journal_is_linked_run_journal_not_separate_ledger`
- `parent_manual_cancel_cascades_to_child_processes`
- `child_run_cannot_outlive_parent_without_detach_policy`
- `detached_child_run_records_parent_detach_intent`
- `before_subagent_start_hook_can_deny_or_narrow_child_request`
- `recursive_subagent_tools_are_stripped_by_default`
- `subagent_otel_projection_uses_journal_and_usage_refs`
- `subagent_event_payloads_are_redacted_by_default`

Future fixture groups:

- topology matrix: `subagents/topology_depth_cycle_matrix_v1.json`
- package diffs: `packages/subagent_child_package_diff_strips_recursive_tools_v1.json`
- handoff policy matrix: `subagents/context_handoff_policy_matrix_v1.json`
- mailbox flow: `subagents/mailbox_parent_message_flow_v1.json`
- clarification flow: `subagents/clarification_round_trip_v1.json`
- event wrapping: `events/subagent_event_wrapped_child_event_v1.json`
- usage rollup: `telemetry/subagent_usage_rollup_v1.json`
- OTel span: `otel/subagent_child_run_span_v1.json`

## Ergonomics

Simple API:

```rust
// Non-compiling contract sketch.
let reviewer_tool = reviewer_agent
    .as_tool("reviewer")
    .read_only()
    .inherit_parent_model()
    .max_depth(1)
    .build()?;

let child = parent_context
    .spawn_child("reviewer")
    .with_context_handoff(ContextHandoffPolicy::None)
    .read_only()
    .await?;

let child_with_refs = parent_context
    .spawn_child("reviewer")
    .with_context_handoff(ContextHandoffPolicy::SelectedRefs {
        refs: vec![context_ref],
        policy_ref: PolicyRef::new("policy.subagent.selected_refs"),
    })
    .read_only()
    .await?;
```

Advanced API:

```rust
// Non-compiling contract sketch.
let request = SubagentRequestBuilder::new(AgentId::new("reviewer"))
    .parent_run(parent_run_id)
    .parent_tool_call(parent_tool_call_id)
    .handoff_context(ContextHandoffPolicy::None)
    .route_policy(SubagentRoutePolicy::InheritParentUnlessAllowed)
    .mailbox_policy(SubagentMailboxPolicy::bounded(20))
    .tool_policy(SubagentToolPolicy::ReadOnly)
    .depth_budget(DepthBudget::max_depth(1))
    .build()?;
```

Canonical lowering:

- `Agent::as_tool` lowers into a subagent capability in `RuntimePackage`.
- `spawn_child("reviewer")` lowers into `SubagentRequest`.
- `SubagentRequest` lowers into a child `RunRequest` with a stripped
  `RuntimePackage` snapshot.
- Child package construction still strips recursive subagent tools and validates route/model policy.
- Default handoff remains `ContextHandoffPolicy::None` unless the caller
  explicitly chooses a broader policy.

Equivalence:

- Simple and advanced spawn paths emit the same subagent events and journal records.
- Parent mailbox, clarification, cancellation, and usage rollup behavior are identical.

SDK owns / Host owns:

- SDK owns helper lowering, child package stripping, parent-owned supervision, mailbox/clarification records, and event wrapping.
- Host owns inspector UI, promotion to conversation, concrete child runner, and user-facing names/descriptions.
- SDK owns child usage/cost rollup records and derived telemetry inputs.
- Host owns rate tables, billing UI, detached-child dashboards, and product
  workflows over subagent events.

Tests:

- `agent_as_tool_lowers_to_subagent_request`
- `simple_spawn_and_explicit_subagent_request_emit_equivalent_events`
- `child_package_strips_subagent_tools`

## Complete Example

Typed shape:

```rust
// Non-compiling contract sketch.
let child_run_id = RunId::new();

let request = SubagentRequest {
    request_id: SubagentRequestId::new(),
    child_run_id,
    child_agent_id: AgentId::new("reviewer"),
    parent_run_id,
    parent_tool_call_id,
    child_source: SourceRef::subagent(parent_run_id),
    child_destination: DestinationRef::child_agent(child_run_id),
    route_policy: SubagentRoutePolicy::InheritParentUnlessAllowed,
    context_handoff: ContextHandoffPolicy::SelectedRefs {
        refs: vec![context_ref],
        policy_ref: PolicyRef::new("policy.subagent.selected_refs"),
    },
    child_package_policy: ChildRuntimePackagePolicy::strip_recursive_defaults(parent_package_fingerprint),
    child_tool_policy: SubagentToolPolicy::ReadOnly,
    mailbox_policy: SubagentMailboxPolicy::bounded(20),
    lifecycle_policy_ref: Some(RunChildLifecyclePolicyRef::new("policy.child.parent_owned")),
    depth_budget: DepthBudget::max_depth(1),
    idempotency_key: IdempotencyKey::new("subagent-start-reviewer-1"),
};

let child = supervisor.start_child(&parent_context, request).await?;

supervisor.send_parent_message(SubagentParentMessage {
    parent_message_id: ParentMessageId::new(),
    parent_run_id,
    child_run_id: child.child_run_id,
    parent_tool_call_id,
    source: SourceRef::parent_agent(parent_run_id),
    destination: DestinationRef::child_agent(child.child_run_id),
    content_ref: ContentRef::new("parent_message/ref_1"),
    visibility: ParentMessageVisibility::ChildOnly,
    policy_refs: vec![PolicyRef::new("policy.subagent.mailbox")],
    idempotency_key: IdempotencyKey::new("parent-message-reviewer-1"),
    created_at: Timestamp::now(),
}).await?;
```

Replaceable ports:

- `SubagentSupervisor` can dispatch to an in-process runner, external agent adapter, or host child-run manager.
- Child package construction uses runtime package rules and strips recursive subagent tools by default.
- Parent mailbox and clarification channels are ports, not user chat sessions.

Wiring:

1. Parent tool request asks for a child run.
2. Supervisor builds child runtime package with inherited/limited policy.
3. Parent appends child-start `EffectIntent`, `SubagentStartedRecord`, and `SubagentStarted`; child begins only after durable append succeeds.
4. Handoff content, if any, moves through `SubagentHandoff`, `ContextContribution`, and `ContextProjection` with content refs and redaction policy.
5. Child events are wrapped into the parent stream as `SubagentEvent` while the child journal remains the durable child-run truth.
6. Child can ask parent for clarification through `SubagentClarificationRequested`.
7. Parent rolls up child usage and terminal status once.

Events:

- `SubagentStarted`
- `SubagentHandoff`
- wrapped `SubagentEvent`
- `SubagentParentMessageSent`
- `SubagentParentMessageRead`
- `SubagentClarificationRequested`
- `SubagentClarificationResponded`
- `SubagentCompleted`, `SubagentFailed`, or `SubagentCancelled`
- `SubagentUsageRolledUp`

Journal:

- `SubagentStartedRecord`
- `SubagentHandoffRecord`
- `SubagentWrappedEventRecord`
- `SubagentParentMessageRecord`
- `SubagentParentMessageReadRecord`
- `SubagentClarificationRequestedRecord`
- `SubagentClarificationRespondedRecord`
- `SubagentUsageRolledUpRecord`
- `SubagentCompletedRecord`
- `ChildLifecycleRecord`

Policies and failures:

- Unknown child provider route fails closed.
- Child cannot be addressed as a normal user chat.
- Parent cancellation cancels child.
- Parent completion requires terminal child state, usage rollup, or explicit detach records.
- Duplicate clarification replies are deduped by idempotency key.
- Recursive subagent tools are stripped by default.
- Raw child or parent transcript content is not emitted by default.

SDK owns / Host owns:

- SDK owns parent/child IDs, `SubagentRequest` to child `RunRequest` lowering, stripped package snapshots, supervision rules, lifecycle policy, mailbox/clarification records, event wrapping, and usage rollup contract.
- Host owns subagent inspector UI, promotion to conversation, concrete child runner adapter/process management, rate tables/billing UI, and continued supervision of explicitly detached child runs.

Tests:

- `child_package_strips_subagent_tools`
- `child_can_ask_parent_without_becoming_user_chat`
- `subagent_clarification_records_replay_after_resume`
- `child_run_cannot_outlive_parent_without_detach_policy`
