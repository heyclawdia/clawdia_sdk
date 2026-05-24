# Phase NN: Name

## Objective

State the single gate this phase must close before later phases start.

## Prerequisites

- Previous phase exit checklist has passed.
- Required shared docs have been read.

## Goals

| Goal | Owner role | Parallelism | Purpose |
| --- | --- | --- | --- |
| `NNa-name.md` | `docs/workstreams/_roles/NN-owner.md` | all goals in this folder are parallel-safe | One sentence. |

## Phase Exit Checklist

- Primitive reuse is proven.
- New feature-layer primitives have owners, sidecar contracts, fingerprint impact, events, journal records, and validation.
- No goal introduced a parallel run loop, package registry, event stream, journal, policy path, context projection path, or side-effect path.
- Required docs audits/tests named by the goal docs pass.

## Handoff

Every goal final response must include changed files, validation evidence, primitive-lowering evidence, open risks, and cross-cutting proposals.
