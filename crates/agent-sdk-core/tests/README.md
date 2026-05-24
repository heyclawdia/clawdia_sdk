# agent-sdk-core Test Layout

Integration tests mirror the source responsibility folders while preserving stable Cargo test target names through thin root-level shims.

| Folder | Responsibility |
| --- | --- |
| `domain/` | ID/ref/privacy/error/policy contracts. |
| `package/` | Runtime package and capability fingerprint contracts. |
| `records/` | Event, journal, content/context, and output record contracts. |
| `ports/` | Provider, event bus, sink, archive, and adapter conformance contracts. |
| `runtime/` | Agent runtime, run-control, loop-state, and handle integration contracts. |
| `feature_layers/` | Feature-layer contracts that lower into the primitive kernel without owning a second runtime path. |
| `p0/` | First complete fake-provider text-run integration contracts. |
| `testing/` | Fake fixture harness and SDK test-kit contracts. |
| `fixtures/` | Golden JSON fixtures grouped by contract family. |

Root `*_contract.rs` files should contain only `#[path = "..."] mod ...;` wiring so launch-doc commands like `cargo test --test event_contract` stay stable.
