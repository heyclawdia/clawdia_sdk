# Reference TUI Agent Orchestrator Example Plan

## Objective

After the core SDK and toolkit adapter plan are implemented, build a product-neutral CLI/TUI reference host that proves how SDK users can compose multiple agents, providers, ACP agents, MCP tools, isolation/access profiles, events, journals, approvals, cancellation, replay, and fake harnesses without copying product-specific host behavior.

This is an example host, not SDK core. It should live outside `agent-sdk-core` and should use the SDK exactly the way a downstream user would.

## Readiness Answer

If the existing implementation and toolkit plans land completely, the SDK should have enough primitives for a TUI orchestrator example. What is still missing is the example-host plan and its gates:

- a reference-host package boundary;
- a simple product-neutral TUI/user workflow contract;
- a config/profile format for agents, routes, models, access profiles, MCP selections, ACP agents, and fake/live mode;
- a TUI state reducer that projects SDK events and journal state into terminal views without becoming a second event store;
- fake-first end-to-end scenarios that prove the example works before live providers or real runtimes are used;
- terminal rendering/snapshot checks, no-overlap checks, and scripted CLI flows;
- documentation that teaches users how to adapt the example without treating it as SDK authority.

## Required Existing SDK Capabilities

The example should not start until these are implemented and validated:

| Required capability | Why the TUI needs it | Source plan |
| --- | --- | --- |
| Core run API | Start runs, wait, cancel, reconnect, and inspect terminal status. | `docs/implementation-workstreams/03-run-control/` |
| Events and journals | Drive TUI panels from `EventFrame` and use `RunJournal` as durable truth. | `docs/implementation-workstreams/02-core-records/`, `docs/implementation-workstreams/11-replay-hardening/` |
| Agent pool and subagents | Show multiple agents, child runs, handoffs, cancellation, and usage rollup. | `docs/implementation-workstreams/05-agent-pool-coordination/`, `docs/implementation-workstreams/10-feature-ports/10c-subagents.md` |
| Provider routes and model catalog | Let users choose provider/model refs without hardcoding names in the example. | `docs/agent-sdk-toolkit/adapter-and-runtime-plan.md` |
| Public testing namespace | Run the whole example in fake mode and let SDK consumers copy the harness. | `coding_standards.md`, `docs/workstreams/validation-gates.md` |
| ACP JSON-RPC harness | Treat ACP agents as external-agent routes through real protocol framing. | `docs/agent-sdk-toolkit/adapter-and-runtime-plan.md` |
| MCP exact capability selection | Attach only selected tools/resources/prompts from a server. | `docs/agent-sdk-toolkit/adapter-and-runtime-plan.md` |
| Isolation/access profiles | Show prebuilt profiles without ambient `~`, host network, shell, or write access. | `docs/agent-sdk-toolkit/adapter-and-runtime-plan.md` |
| Output delivery and approvals | Terminal prompts and outputs stay host-owned while using SDK policy/effect paths. | `docs/examples/remote-headless-approval.md` |

## Example Boundary

Recommended package shape once code is allowed:

```text
examples/agent-orchestrator-tui/
  Cargo.toml
  README.md
  src/
    domain/          # config DTOs, view model IDs, product-neutral screen state
    application/     # orchestration service over SDK runtime, no terminal I/O
    adapters/
      tui/           # terminal renderer and input mapping
      config/        # profile loading and validation
      fake_host/     # fake provider/ACP/MCP/isolation wiring for demo mode
    testing/         # reusable scripted terminal and scenario harness
  profiles/
    fake.toml
    live.example.toml
  tests/
    scenarios/
    snapshots/
```

Rules:

- The example may depend on `agent-sdk-core`, `agent-sdk-toolkit`, and optional adapter crates.
- The example must not be a dependency of SDK core or toolkit.
- The example must not define new provider, event, journal, policy, MCP, ACP, or isolation authority.
- The example may define host config DTOs and terminal view models, but they lower into SDK primitives before execution.
- The default profile is fake-only and must work without credentials, containers, network, or installed MCP/ACP servers.
- Reusable fake providers, ACP harnesses, MCP harnesses, isolation harnesses, output sinks, and approval brokers belong in `agent_sdk_core::testing` or optional toolkit/adapter test-support surfaces. The example wires those public fakes together; it does not hide new reusable fakes inside the example.

## User Workflow Contract

The first usable screen should be the orchestrator, not a marketing or docs page.

Minimum workflows:

1. Start a fake-mode TUI with one command.
2. Inspect configured agents, provider/model route refs, access profile, selected MCP capabilities, and ACP agent routes.
3. Start a run from a prompt.
4. Watch event stream, model/tool/approval progress, journal cursor, and terminal state update live.
5. Approve or deny a tool request from the terminal host.
6. Cancel a run and verify cancellation is journaled and reflected in events.
7. Replay or reconnect to a completed run from journal/event cursors.
8. Switch between fake provider, fake ACP agent provider, and fake local provider routes.
9. Show denied file, terminal, MCP, and network attempts as policy outcomes rather than terminal crashes.

Advanced workflows can wait:

- live OpenAI-compatible provider profile;
- live ACP agent process;
- live MCP server process;
- concrete container or lightweight VM runtime;
- local MLX/llama.cpp artifact resolver;
- remote or web dashboard.

## Config/Profile Sketch

```toml
# Non-compiling design sketch.
[profile]
id = "profile.fake.orchestrator"
mode = "fake"

[[agents]]
id = "agent.main"
name = "Main"
default_route = "route.fake.model"
access_profile = "access.repo_readonly"
tools = ["tool.workspace.read", "tool.mcp.git.status"]

[[provider_routes]]
id = "route.fake.model"
provider = "fake"
model_ref = "model.fake.fast"
catalog_hash = "catalog.fake.v1"

[[agent_provider_routes]]
id = "route.fake.acp"
kind = "acp"
command_ref = "agent.acp.fake"
transport = "stdio"

[[mcp_servers]]
id = "server.git"
transport = "stdio"
allow_tools = ["git.status", "git.diff"]
allow_resources = []
allow_prompts = []

[[access_profiles]]
id = "access.repo_readonly"
workspace = "current_dir"
filesystem = "read_only"
network = "deny"
shell = "deny"
```

Config rules:

- Config is host-owned input. The execution authority is the validated `RuntimePackage`.
- Model names and capabilities come from the provider model catalog or host catalog overlay.
- MCP entries are discovery candidates until filtered into selected SDK capabilities.
- ACP commands are stable command refs, not raw process IDs or ambient editor handles.
- Live profiles must fail closed without credentials, endpoint allowlist, and conformance profile.

## TUI Projection Rules

- Terminal views subscribe to SDK events and query durable journal state; they do not become a second source of truth.
- View state is a reducer over `EventFrame`, journal cursors, run handles, package summaries, and redacted content refs.
- Raw prompts, raw tool args/results, secrets, host paths, cookies, auth headers, and provider request bodies are hidden unless explicit test/demo policy allows bounded redacted display.
- Slow terminal rendering cannot block the agent loop. Dropped UI frames are acceptable when terminal run events and journal truth are preserved.
- Terminal actions such as approve, deny, cancel, replay, and switch route call SDK APIs or host config reload paths; they do not mutate active `RuntimePackage` snapshots.

## Workstreams

### Phase 0: Readiness Stitching

Run after the existing implementation and toolkit adapter plans have passing evidence.

Deliver:

- readiness matrix mapping implemented SDK/toolkit APIs to the TUI workflows above;
- list of unsupported live paths with fake alternatives;
- decision on example package location and Cargo workspace inclusion.

Validation:

- docs audit proving no new core primitive is required;
- product-neutrality audit;
- reviewer PASS before code starts.

### Phase 1: Example Contract And Config

Deliver:

- example README;
- host config/profile schema;
- non-compiling API sketch promoted to compile tests only after code exists;
- config-to-`RuntimePackage` lowering contract;
- fake/live profile separation.

Validation:

- config parser unit tests with fake profiles;
- denied-by-default tests for missing credentials, endpoint allowlist, MCP selection, ACP command ref, and isolation policy;
- no live service calls.

### Phase 2: Fake-First Runtime Wiring

Deliver:

- wiring for the public fake provider route from SDK/toolkit test support;
- wiring for the public fake ACP subprocess over JSON-RPC stdio;
- wiring for the public fake MCP server/proxy with exact tool/resource/prompt selection;
- wiring for the public fake isolation/access profile;
- wiring for public fake output sink and approval broker;
- one full scripted run.

Validation:

- `cargo test -p agent-sdk-example-orchestrator-tui --test fake_scenarios`;
- hostile scenarios: malformed ACP frame, duplicate event, file denial, terminal denial, MCP unselected tool denial, provider malformed stream, unsupported model capability, cancel during tool approval;
- reusable SDK-consumer test harness documented.
- any missing reusable fake/conformance harness blocks upstream SDK/toolkit work instead of being implemented privately inside the example.

### Phase 3: TUI State And Rendering

Deliver:

- terminal event reducer;
- screen model for agents, runs, events, journal, package, tools, approvals, and logs;
- keyboard/action mapping through host-owned commands;
- terminal renderer behind an adapter boundary.

Validation:

- reducer tests use recorded fake event/journal fixtures;
- terminal snapshot tests for narrow and wide viewports;
- no-overlap/no-truncation checks for key screens;
- rendering backpressure test proving terminal refresh does not block runtime event emission.

### Phase 4: Optional Live Adapter Profiles

Deliver only after each adapter has its own conformance evidence:

- OpenAI Responses or OpenAI-compatible live profile;
- ACP real-process profile;
- MCP real-server profile;
- local MLX/llama.cpp profile;
- concrete isolation runtime profile.

Validation:

- live smoke tests are opt-in and credential-gated;
- fake conformance remains the release gate;
- live profile docs name unsupported capabilities and fail-closed setup errors.

### Phase 5: Example Release Gate

Deliver:

- README walkthrough;
- architecture diagram;
- profile examples;
- troubleshooting guide;
- contract-to-example traceability table;
- final independent SDK review.

Validation:

```bash
cargo fmt --check
cargo test --workspace
cargo test -p agent-sdk-example-orchestrator-tui
cargo run -p agent-sdk-example-orchestrator-tui -- --profile examples/agent-orchestrator-tui/profiles/fake.toml --script smoke-basic
# Run the standard product-neutrality audit for product host names over the example and toolkit docs.
```

The smoke command may use a non-interactive script mode so CI can prove the same path a person uses in the TUI.

## Agent Launch Plan

Once readiness stitching passes, agents can work in parallel by phase:

| Agent | Phase | Write scope | Output |
| --- | --- | --- | --- |
| Readiness/stitching agent | Phase 0 | readiness matrix, unsupported-path list, package-location decision | approval to start example code or upstream-blocker list |
| Contract/config agent | Phase 1 | `examples/agent-orchestrator-tui/README.md`, config DTO docs/tests | config lowering and fail-closed tests |
| Fake wiring agent | Phase 2 | example wiring over public fake provider/ACP/MCP/isolation/output/approval harnesses | deterministic fake E2E |
| TUI reducer agent | Phase 3 | view model and reducer modules/tests | event/journal projection |
| TUI renderer agent | Phase 3 | terminal adapter/rendering/snapshots | usable no-overlap screens |
| Live profile agent | Phase 4 | opt-in profiles and adapter wiring | credential-gated smoke only |
| Review/stitching agent | every phase | docs, traceability, validation reports | phase exit and reviewer findings |

Phase 2 must not start until Phase 1 config lowering passes. Phase 3 may start after Phase 2 has stable fake fixtures. Phase 4 must wait for adapter-specific conformance evidence. Phase 5 waits for all in-scope phases.

## Missing-Primitive Escalation

If implementation discovers a gap, do not patch around it inside the example. Record a primitive decision before coding:

- Is this core SDK behavior, optional adapter behavior, or host-owned behavior?
- Which existing primitive can carry it?
- If a new primitive is required, which contract owns it?
- What events, journal records, policy refs, privacy classes, and fake tests prove it?
- Can the TUI still run fake-only without the new primitive?

## Definition Of Done

The TUI example is ready only when:

- fake mode runs without credentials, network, containers, live providers, real MCP servers, or real ACP agents;
- every terminal action lowers to SDK primitives or host-owned config reload;
- model/provider selection reads the catalog;
- MCP exposes only selected capabilities;
- ACP fake communicates over JSON-RPC stdio;
- isolation/access defaults deny ambient filesystem, shell, and network;
- events and journals remain the source of truth for terminal views;
- scripted smoke and snapshot tests pass;
- independent SDK review says product-neutrality, primitive fit, mockability, DDD layout, and TDD gates pass.
