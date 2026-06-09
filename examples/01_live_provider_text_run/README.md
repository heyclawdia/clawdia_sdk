# Live Provider Text Run

Text-run example with deterministic fake-provider fallback.

Run deterministic mode:

```sh
cargo run -p clawdia-sdk-example-01-live-provider-text-run
```

Expected deterministic output shape:

```text
output=fake provider text run; status=Completed; records=10
```

Run live OpenAI mode:

```sh
OPENAI_API_KEY=... OPENAI_MODEL=gpt-4.1-mini cargo run -p clawdia-sdk-example-01-live-provider-text-run
```

No credentials are required for the default path. `OPENAI_API_KEY` opts into a
host-owned live provider call; `OPENAI_MODEL` defaults to `gpt-4.1-mini`.

Common failures:

- Invalid or missing live credentials only affect the live path.
- Network, model availability, and provider billing are host-owned live-provider
  concerns, not SDK runtime state.

## Under The Hood

SDK-owned boundaries:

- `ProviderAdapter` projection, `RunRequest`, runtime execution, journals, and
  events.

Host-owned boundaries:

- API key lookup, model selection, endpoint/network access, billing, prompt
  copy, and production retry policy.

The live path still uses the canonical `ProviderAdapter` port. `AgentApp` only
wires the provider into the core runtime.
