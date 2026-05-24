# Owner Role 10: Generic Scenario Coverage

## Owner Role

Scenario coverage and SDK boundary agent.

## Writable Files

- `docs/examples/*.md`

## Future Implementation Writable Scope

This workstream remains scenario/documentation-oriented. Once SDK code exists, scenario tests should be created only when the stitching owner assigns a disjoint test path for them.

## Read-Only Inputs

All SDK contracts, architecture docs, and workstream validation gates.

## Contract To Deliver

Keep the scenario examples aligned with SDK contracts while preserving the boundary that hosts are products built on top of the SDK, not SDK core. Identify any workflow that lacks an SDK primitive and propose the product-neutral primitive through the handoff proposal path.

Scenarios must be generic enough for many hosts to reuse:

- desktop or web chat;
- CLI and headless runs;
- realtime voice or streaming input;
- remote channel input/output;
- external runtime session adapters;
- app/live event projection;
- telemetry and trace export;
- structured output;
- isolation;
- hook lifecycle;
- long-running process detach;
- stream rules;
- subagent supervision.

## Must Not Own

SDK core contract authority, product runtime compatibility, UI routing implementation, host trace-store schema, external runtime process lifecycle, marketplace UX, workflow/DAG engines, or product-specific recommendation/evolution features.

## Integration Handoff

Send missing-primitive proposals and SDK/host boundary risks to the stitching owner. Put proposal text in the handoff; do not edit shared reference or architecture files unless the stitching owner delegates it.

## Required Validation

- Scenario mapping audit: every listed scenario maps to SDK primitives plus host-owned adapters without naming product-specific source/destination/helper types.
- Boundary audit: active architecture, contract, workstream, and example docs contain no product-specific host adapter as implementation authority.
- Coverage audit: every scenario names events, journal records, policy decisions, telemetry/cost records, recovery behavior, and host-owned storage/transport.
- Example audit: scenario diagrams remain concrete but product-neutral.
- Missing primitive process: any gap becomes a product-neutral proposal block in the handoff, not an ad hoc host-only core change.
- Primitive-lowering review: every scenario maps to the primitive kernel plus feature layers and host-owned adapters; no scenario introduces a new core primitive without a proposal.
- Handoff evidence: scenario checklist, boundary grep output, changed examples, primitive mapping table, and missing-primitive proposal blocks.
