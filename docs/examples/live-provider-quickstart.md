# Live Provider Quickstart

Start here for an app-facing run. The provider is real, but it still lowers
through the SDK runtime contracts:

`Agent` -> `RunRequest` -> `RuntimePackage` -> `AgentRuntime` ->
`ProviderAdapter` -> `RunJournal` -> `AgentEventBus` -> `RunResult`.

```rust
use agent_sdk_core::{
    Agent, AgentRuntime, AgentId, PolicyKind, PolicyRef, ProviderRouteSnapshot,
    RuntimePackage, RuntimePackageId, RunId, RunRequest, SourceKind, SourceRef,
    testing::{FakeContentResolver, FakeJournalStore},
};
use agent_sdk_provider::OpenAiResponsesAdapter;

let agent = Agent::builder()
    .id(AgentId::new("agent.quickstart.live"))
    .name("quickstart")
    .build()?;

let package = RuntimePackage::builder(RuntimePackageId::new("package.quickstart.live"))
    .agent(agent.snapshot())
    .provider_route(ProviderRouteSnapshot::new(
        "provider.openai.responses",
        "gpt-4.1",
    ))
    .policy(PolicyRef::with_kind(
        PolicyKind::RuntimePackage,
        "policy.quickstart.package",
    ))
    .build()?;

let provider = OpenAiResponsesAdapter::from_env("gpt-4.1")?;
let journal = FakeJournalStore::default();
let event_bus = agent_sdk_core::InMemoryAgentEventBus::default();

let runtime = AgentRuntime::builder()
    .default_package(package)
    .provider("provider.openai.responses", provider)?
    .journal(journal.clone())
    .event_bus(event_bus.clone())
    .content(FakeContentResolver::default())
    .build()?;

let result = runtime.run_text(RunRequest::text(
    RunId::new("run.quickstart.live"),
    agent.id().clone(),
    SourceRef::with_kind(SourceKind::Host, "source.quickstart"),
    "Say hello.",
))?;
```

## Provider Options

`agent-sdk-provider` exposes live adapters in one aggregate crate:

```rust
use agent_sdk_provider::{
    AnthropicMessagesAdapter, GeminiGenerateContentAdapter, OpenAiResponsesAdapter,
};

let openai = OpenAiResponsesAdapter::from_env("gpt-4.1")?;
let anthropic = AnthropicMessagesAdapter::from_env("claude-sonnet-4-5")?;
let gemini = GeminiGenerateContentAdapter::from_env("gemini-2.5-flash")?;
```

The default live HTTP transport uses the system `curl` executable and keeps the
SDK synchronous. Hosts that need a different HTTP stack can inject a
`JsonHttpTransport` while preserving the same `ProviderAdapter` contract.

## What This Proves

- The provider route is frozen into `RuntimePackage`; provider choice is not
  hidden runtime state.
- Live providers are still just `ProviderAdapter` implementations.
- Provider credentials are resolved by the host at adapter construction time.
  They do not enter runtime package fingerprints, journals, events, or content
  refs.
- Events are live observation. The journal is durable truth.
- Deterministic provider transports remain for tests, but they are not the
  onboarding path.
