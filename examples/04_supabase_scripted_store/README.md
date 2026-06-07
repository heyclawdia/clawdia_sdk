# Supabase Scripted Store Example

## Command

```sh
cargo run -p clawdia-sdk-example-04-supabase-scripted-store
```

## Expected Output

```text
content.provider_arguments.7d6441497d2a000b8143602a "README.md" https://example.supabase.co/rest/v1/agent_sdk_provider_arguments
```

## What It Proves

This example uses the Supabase store adapter with an injected scripted
transport. It stores provider arguments, reads them back by content ref, and
shows the PostgREST table endpoint without requiring live credentials.

## SDK-Owned Boundary

The SDK owns the Supabase REST request shape, schema/profile headers,
content-ref readback contract, and migration-backed adapter surface.

## Host-Owned Boundary

The host owns live Supabase project provisioning, credentials, RLS policy,
network transport, service-role rotation, and migration rollout.

## Failure Modes

- Scripted transport exhaustion returns a typed SDK contract error.
- Non-2xx Supabase responses fail through the adapter.
- Malformed stored JSON fails during provider-argument readback.
