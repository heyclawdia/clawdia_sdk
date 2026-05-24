# Subagent Contract

Subagents are parent-owned child runs. They are not user-chatable conversations unless a host explicitly promotes them.

## External Lessons

- Strands has multi-agent lifecycle and node stream events. The SDK should adopt explicit lineage and events.
- Cursor names subagents with prompts/models, but product routing stays outside the core run.
- The SDK requires stricter safety than free-form agent societies: no direct user chat, no recursive subagent tools by default, route validation, parent-owned cancellation, and child usage rollup.

## Public Shape

```rust
// Non-compiling contract sketch.
pub struct SubagentSupervisor {
    topology: AgentTopology,
    depth_budget: DepthBudget,
    route_policy: SubagentRoutePolicy,
    handoff_policy: ContextHandoffPolicy,
    tool_policy: SubagentToolPolicy,
    lifecycle_policy: SubagentLifecyclePolicy,
}

impl SubagentSupervisor {
    pub async fn start_child(&self, parent: &RunContext, request: SubagentRequest) -> Result<ChildRunHandle, AgentError>;
    pub async fn cancel_child(&self, child_run_id: RunId) -> Result<(), AgentError>;
    pub async fn send_parent_message(&self, message: SubagentParentMessage) -> Result<ParentMessageReceipt, AgentError>;
    pub async fn ask_parent_for_clarification(&self, request: SubagentClarificationRequest) -> Result<SubagentClarificationResponse, AgentError>;
    pub async fn answer_child_clarification(&self, reply: SubagentClarificationReply) -> Result<(), AgentError>;
    pub async fn read_parent_messages(&self, fetch: SubagentParentMessageFetch) -> Result<Vec<SubagentParentMessage>, AgentError>;
    pub fn wrap_child_event(&self, event: AgentEvent) -> Result<AgentEvent, AgentError>;
}

pub struct SubagentLifecyclePolicy {
    pub on_parent_cancel: ChildShutdownBehavior,
    pub on_parent_complete: ChildShutdownBehavior,
    pub detach_policy: DetachPolicy,
    pub require_terminal_before_parent_complete: bool,
}

pub enum ContextHandoffPolicy {
    None,
    SummaryOnly { max_tokens: u32 },
    SelectedRefs { refs: Vec<ContentRef> },
    FullHistoryWithPolicy { policy_ref: PolicyRef },
}
```

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
    pub created_at: Timestamp,
}

pub struct SubagentClarificationRequest {
    pub clarification_id: ClarificationId,
    pub parent_run_id: RunId,
    pub child_run_id: RunId,
    pub parent_tool_call_id: ToolCallId,
    pub agent_name: String,
    pub question_ref: ContentRef,
    pub timeout_ms: u64,
}

pub struct SubagentClarificationReply {
    pub clarification_id: ClarificationId,
    pub parent_run_id: RunId,
    pub child_run_id: RunId,
    pub parent_tool_call_id: ToolCallId,
    pub answer_ref: ContentRef,
    pub actor: ActorRef,
}
```

Rules:

- Parent messages are source-scoped to a parent tool call or explicit host action.
- Child reads require an explicit parent mailbox or selected-ref policy. The default handoff is isolated child context with `ContextHandoffPolicy::None`.
- A child clarification request pauses only the child step unless parent policy escalates it.
- Parent answers are delivered as child context items with lineage, not as direct user messages.
- Clarification IDs are unique per parent run and stable across resume.
- Duplicate replies with the same idempotency key are deduped.
- Parent-message content uses content refs; raw bodies are omitted from events unless content-capture policy explicitly allows them.

## Child Package Rules

- Strip subagent-spawning tools and subagent definitions by default.
- Validate provider/model route against backend configured routes.
- Inherit parent route unless override is explicit and valid.
- Apply child handoff policy: none, summary-only, selected refs, or full history with explicit policy.
- Full parent history is never the default. It requires `ContextHandoffPolicy::FullHistoryWithPolicy` and a policy ref that survives package validation.
- Apply child tool policy: inherited allowlist, read-only, no tools, or custom.
- Child package gets its own fingerprint linked to parent package fingerprint.

## Child Lifecycle Rules

- Subagent runs are parent-owned child artifacts by default.
- Parent manual cancel appends child shutdown intent before cancelling the child run.
- Parent normal completion requires each child run to be terminal, rolled up, or explicitly detached under policy.
- A child cannot outlive its parent by accident. Detachment requires explicit policy, user/host intent when configured, host acknowledgement, and `ChildLifecycleRecord` entries.
- Detached child runs remain linked to the parent run through lineage and reclaim policy, but the host or optional orchestration layer owns continued supervision.
- Child lifecycle policy cannot grant recursive subagent tools or broader tool access than the child package policy allows.
- `BeforeSubagentStart` hooks can deny or narrow a child request through typed response only; they cannot silently create untracked child processes.

## Event And Journal Rules

- Parent appends `EffectIntent { kind: ChildAgentStart }` and `SubagentStartedRecord` before child starts.
- Child events are wrapped as `SubagentEvent` with parent run ID and child run ID.
- Parent appends `SubagentParentMessageSent` before delivering a parent message to a child.
- Child reads append `SubagentParentMessageRead` with message IDs and read cursor, not raw bodies.
- Child clarification requests append `SubagentClarificationRequested` before waiting.
- Parent replies append `SubagentClarificationResponded` before delivery to the child.
- Child journal is a child trace/session artifact.
- Parent appends `SubagentUsageRolledUp`.
- Child terminal completion appends `EffectResult` for the child start effect before parent completion can seal.
- Parent cancellation appends child shutdown intent and cancels child by default.
- Parent completion appends either child terminal/rollup records or explicit detach records before sealing.
- Child failure returns a typed tool result or parent failure according to policy.

Journal records:

| Record | Required fields |
| --- | --- |
| `SubagentStartedRecord` | parent run ID, child run ID, parent tool call ID, child package fingerprint, handoff policy, tool policy |
| `SubagentParentMessageRecord` | parent message ID, parent run ID, child run ID, content ref, visibility, delivery status |
| `SubagentParentMessageReadRecord` | child run ID, read cursor, message IDs returned, policy ref |
| `SubagentClarificationRequestedRecord` | clarification ID, parent/child run IDs, parent tool call ID, question ref, timeout |
| `SubagentClarificationRespondedRecord` | clarification ID, actor ref, answer ref, delivery status, idempotency key |
| `SubagentUsageRolledUpRecord` | child usage ref, parent usage ref, cost/currency, terminal status |
| `SubagentCompletedRecord` | child terminal status, result ref, error ref, policy outcome |
| `ChildLifecycleRecord` | child shutdown intent/result or detach intent/ack/reclaim policy |

`SubagentStartedRecord`, `SubagentCompletedRecord`, and `ChildLifecycleRecord` must embed or map one-to-one to the shared effect fields. Child start uses `EffectKind::ChildAgentStart`; parent-driven shutdown uses `EffectKind::ChildArtifactShutdown`; detach uses `EffectKind::DetachTransfer`. Feature-specific records can add child package, mailbox, clarification, and usage fields, but they cannot bypass intent-before-effect or terminal `EffectResult`.

## Promotion Rule

Child transcript is not a top-level conversation by default. Host promotion requires:

- explicit host policy
- new conversation ID
- audit link to parent/child run IDs
- privacy review of child context

## Compatibility Boundary

Provider narrative text about "subagents" is not an SDK child run. External runtime compatibility notifications can be observed as external runtime events but cannot bypass `SubagentSupervisor`.

## Acceptance Tests

- `child_package_strips_subagent_tools`
- `child_cannot_be_addressed_as_normal_chat`
- `parent_cancel_cancels_child`
- `child_usage_rollup_preserves_child_run_id`
- `unknown_child_provider_route_fails_closed`
- `extension_subagent_name_is_namespaced`
- `provider_narrative_subagent_text_is_ignored`
- `child_transcript_promotion_requires_host_policy`
- `child_can_ask_parent_without_becoming_user_chat`
- `clarification_round_trip_is_parent_owned`
- `duplicate_clarification_reply_is_deduped`
- `child_read_parent_messages_is_policy_scoped`
- `parent_message_events_use_content_refs_by_default`
- `subagent_parent_message_events_are_wrapped`
- `subagent_clarification_records_replay_after_resume`
- `agent_as_tool_lowers_to_subagent_request`
- `simple_spawn_and_explicit_subagent_request_emit_equivalent_events`
- `parent_manual_cancel_cascades_to_child_processes`
- `child_run_cannot_outlive_parent_without_detach_policy`
- `detached_child_run_records_parent_detach_intent`
- `before_subagent_start_hook_can_deny_or_narrow_child_request`

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
    .with_context_handoff(ContextHandoffPolicy::SelectedRefs { refs: vec![context_ref] })
    .read_only()
    .await?;
```

Advanced API:

```rust
// Non-compiling contract sketch.
let request = SubagentRequestBuilder::new(AgentId::new("reviewer"))
    .parent_run(parent_run_id)
    .parent_tool_call(parent_tool_call_id)
    .handoff_context(ContextHandoffPolicy::SelectedRefs { refs: vec![context_ref] })
    .route_policy(SubagentRoutePolicy::InheritParentUnlessAllowed)
    .parent_mailbox_max_messages(20)
    .tool_policy(SubagentToolPolicy::ReadOnly)
    .depth_budget(DepthBudget::max_depth(1))
    .build()?;
```

Canonical lowering:

- `Agent::as_tool` lowers into a subagent capability in `RuntimePackage`.
- `spawn_child("reviewer")` lowers into `SubagentRequest`.
- Child package construction still strips recursive subagent tools and validates route/model policy.

Equivalence:

- Simple and advanced spawn paths emit the same subagent events and journal records.
- Parent mailbox, clarification, cancellation, and usage rollup behavior are identical.

SDK owns / Host owns:

- SDK owns helper lowering, child package stripping, parent-owned supervision, mailbox/clarification records, and event wrapping.
- Host owns inspector UI, promotion to conversation, concrete child runner, and user-facing names/descriptions.

Tests:

- `agent_as_tool_lowers_to_subagent_request`
- `simple_spawn_and_explicit_subagent_request_emit_equivalent_events`
- `child_package_strips_subagent_tools`

## Complete Example

Typed shape:

```rust
// Non-compiling contract sketch.
let request = SubagentRequest {
    child_agent_id: AgentId::new("reviewer"),
    parent_run_id,
    parent_tool_call_id,
    handoff_context: ContextHandoffPolicy::SelectedRefs { refs: vec![context_ref] },
    route_policy: SubagentRoutePolicy::InheritParentUnlessAllowed,
    parent_mailbox_max_messages: Some(20),
    tool_policy: SubagentToolPolicy::ReadOnly,
    depth_budget: DepthBudget::max_depth(1),
    lifecycle_policy: SubagentLifecyclePolicy::parent_owned_defaults(),
};

let child = supervisor.start_child(&parent_context, request).await?;

supervisor.send_parent_message(SubagentParentMessage {
    parent_message_id: ParentMessageId::new(),
    parent_run_id,
    child_run_id: child.run_id,
    parent_tool_call_id,
    source: SourceRef::parent_agent(parent_run_id),
    destination: DestinationRef::child_agent(child.run_id),
    content_ref: ContentRef::new("parent_message/ref_1"),
    visibility: ParentMessageVisibility::ChildOnly,
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
3. Parent appends `SubagentStarted` and child begins.
4. Child can ask parent for clarification through `SubagentClarificationRequested`.
5. Parent rolls up child usage and terminal status.

Events:

- `SubagentStarted`
- wrapped `SubagentEvent`
- `SubagentParentMessageSent`
- `SubagentParentMessageRead`
- `SubagentClarificationRequested`
- `SubagentClarificationResponded`
- `SubagentUsageRolledUp`

Journal:

- `SubagentStartedRecord`
- `SubagentParentMessageRecord`
- `SubagentClarificationRequestedRecord`
- `SubagentClarificationRespondedRecord`
- `SubagentUsageRolledUpRecord`
- `SubagentCompletedRecord`

Policies and failures:

- Unknown child provider route fails closed.
- Child cannot be addressed as a normal user chat.
- Parent cancellation cancels child.
- Parent completion requires terminal child state, usage rollup, or explicit detach records.
- Duplicate clarification replies are deduped by idempotency key.

SDK owns / Host owns:

- SDK owns parent/child IDs, supervision rules, lifecycle policy, mailbox/clarification records, event wrapping, and usage rollup contract.
- Host owns subagent inspector UI, promotion to conversation, concrete child runner process management, and continued supervision of explicitly detached child runs.

Tests:

- `child_package_strips_subagent_tools`
- `child_can_ask_parent_without_becoming_user_chat`
- `subagent_clarification_records_replay_after_resume`
- `child_run_cannot_outlive_parent_without_detach_policy`
