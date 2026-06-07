# agent-sdk-store-supabase

Supabase REST-backed durable store adapters for the Agent SDK.

The crate uses Supabase's PostgREST surface through an injectable transport,
so tests can verify request paths, headers, and payloads without live
credentials. Hosts can provide their own HTTP transport or enable the optional
curl transport.
