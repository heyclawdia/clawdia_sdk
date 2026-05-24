# agent-sdk-core Source Layout

The crate keeps stable public facade modules in `lib.rs`, but implementation files live under SDK responsibility folders:

| Folder | Responsibility |
| --- | --- |
| `domain/` | Ubiquitous language primitives: IDs, refs, privacy classes, errors, and policies. |
| `package/` | Runtime package authority, capability specs, sidecars, catalogs, deltas, and fingerprints. |
| `records/` | Durable and observable records: events, journals, effects, context/content, and output. |
| `ports/` | Public adapter and subscription boundaries such as providers and event buses. |
| `application/` | Agent/runtime/run coordination, projection orchestration, recovery, and lowering helpers. |
| `testing/` | Deterministic fakes, fixtures, and SDK-consumer conformance helpers. |

New implementation files should enter the owning folder first. Root-level modules should remain facade wiring only unless a conventional Cargo layout choice is documented in the phase report.
