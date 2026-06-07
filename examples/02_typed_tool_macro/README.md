# Typed Tool Macro Example

## Command

```sh
cargo run -p clawdia-sdk-example-02-typed-tool-macro
```

## Expected Output

```text
lookup_docs:1
```

## What It Proves

This example uses the `clawdia-sdk` facade macro exports to derive typed tool
arguments/output and build a typed tool declaration through the optional macro
path.

## SDK-Owned Boundary

The SDK owns deterministic schema generation, typed tool identity construction,
and package/tool declaration lowering.

## Host-Owned Boundary

The host owns whether the generated tool is registered in a runtime, which
executor policy applies, and which approval dispatcher or store adapters are
used during execution.

## Failure Modes

- Missing or invalid macro attributes fail at compile time through the macro
  contract tests.
- Schema changes alter the generated tool declaration and must be reviewed as
  runtime package fingerprint inputs.
