# Phase 13 Feature Flag Matrix

## Status

This matrix is the release-readiness evidence for package separation. It is not a publish approval.

| Package | Feature set | Expected dependency boundary | Required command |
| --- | --- | --- | --- |
| `agent-sdk-core` | default | Empty feature set. Core depends only on `serde`, `serde_json`, `sha2`, `thiserror`, and transitive support crates. It does not depend on toolkit, isolation, OTel, extension, workflow, provider, network, async runtime, or product-host crates. | `cargo test -p agent-sdk-core` |
| `agent-sdk-core` | `--no-default-features` | Same as default because default features are empty. Proves optional crates are not required by the primitive kernel. | `cargo test -p agent-sdk-core --no-default-features` |
| `agent-sdk-core` | `--all-features` | Currently adds only the reserved `test-support` feature. Deterministic fakes remain under `agent_sdk_core::testing` and do not add live providers or host infrastructure. | `cargo test -p agent-sdk-core --all-features` |
| `agent-sdk-toolkit` | default | Optional crate that depends on core and concrete helper dependencies such as `regex`. Core has no reverse dependency. | `cargo test -p agent-sdk-toolkit` |
| workspace | default | Runs all current crates without publishing, tagging, live providers, concrete containers, or product adapters. | `cargo test --workspace` |

## Unsupported Optional Crates

The implementation handoff does not include `agent-sdk-isolation`, `agent-sdk-otel`, `agent-sdk-extension`, or `agent-sdk-workflow` packages. Their contracts and reserved ports are represented in core where needed, but concrete adapters/exporters/workflow engines remain unsupported until a later phase adds separate crates, manifests, tests, and release notes.
