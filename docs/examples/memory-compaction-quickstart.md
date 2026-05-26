# Memory Compaction Quickstart

Memory should feel like a small port, not a research project. The host owns the
store; the SDK-owned shape is still:

`MemoryPort` -> `ContextContribution` -> `ContextItem` -> `ContextProjection`

```rust
use agent_sdk_core::{
    AgentId, AgentMessage, ContextBudgetSummary, ContextContribution,
    ContextContributionId, ContextContributionKind, ContextItem, ContextItemId,
    ContextProjection, ContextProjectionId, DestinationKind, DestinationRef, EntityRef,
    MessageId, PolicyKind, PolicyRef, ProjectionRole, SourceKind, SourceRef,
};
use agent_sdk_core::domain::ContentRef as ContentRefId;

trait MemoryPort {
    fn recall(&self, query: &str) -> Vec<ContextContribution>;
}

struct FakeMemory;

impl MemoryPort for FakeMemory {
    fn recall(&self, _query: &str) -> Vec<ContextContribution> {
        let mut pinned = ContextContribution::new(
            ContextContributionId::new("context.contribution.memory.pinned"),
            ContextContributionKind::MemoryRecall,
            EntityRef::agent(AgentId::new("agent.quickstart.memory")),
            SourceRef::with_kind(SourceKind::Host, "source.memory.fake"),
            PolicyRef::with_kind(PolicyKind::Privacy, "policy.memory.refs_only"),
            "remembered project preference",
        )
        .with_content_ref(ContentRefId::new("content.memory.preference"))
        .protected();
        pinned.inline_redacted_summary =
            Some("Prefer live provider examples for onboarding.".to_string());
        vec![pinned]
    }
}

let memory = FakeMemory;
let user = AgentMessage::user_text(
    MessageId::new("message.quickstart.memory.user"),
    "make the next example shorter",
    SourceRef::with_kind(SourceKind::Host, "source.user.quickstart"),
    PolicyRef::with_kind(PolicyKind::Context, "policy.context.user"),
);

let mut recalled = memory.recall("quickstart style");
let compacted = ContextContribution::new(
    ContextContributionId::new("context.contribution.compaction.summary"),
    ContextContributionKind::CompactionSummary,
    EntityRef::agent(AgentId::new("agent.quickstart.memory")),
    SourceRef::with_kind(SourceKind::Sdk, "source.compaction.quickstart"),
    PolicyRef::with_kind(PolicyKind::Context, "policy.context.compaction"),
    "summary of older context",
)
.protected();
recalled.push(compacted);

let provider_destination =
    DestinationRef::with_kind(DestinationKind::Provider, "destination.provider.fake");
let items = recalled
    .into_iter()
    .enumerate()
    .map(|(index, contribution)| {
        ContextItem::admit(
            contribution,
            ContextItemId::new(format!("context.item.quickstart.{index}")),
            provider_destination.clone(),
            ProjectionRole::AssistantContext,
        )
    })
    .collect::<Vec<_>>();

let projection = ContextProjection::build(
    ContextProjectionId::new("context.projection.quickstart.memory"),
    vec![user],
    items,
    Vec::new(),
    provider_destination,
    ContextBudgetSummary {
        max_items: Some(4),
        included_items: 2,
        ..ContextBudgetSummary::default()
    },
    PolicyRef::with_kind(PolicyKind::Redaction, "policy.redaction.refs_only"),
    "runtime.package.quickstart",
)?;

assert_eq!(projection.items.len(), 2);
```

## Rules

- Memory returns candidates, not provider messages.
- Compaction summaries are candidates too, with lineage and policy refs.
- Protected contributions must not be omitted silently; projection fails closed.
- Raw memory content stays behind content refs unless projection policy admits it.
- A future concrete memory crate should provide stores and compaction helpers, not
  a second run loop or shadow transcript.
