# File Store Example

## Command

```sh
cargo run -p clawdia-sdk-example-03-file-store
```

## Expected Output

```text
content.provider_arguments.7d6441497d2a000b8143602a "README.md"
```

## What It Proves

This example stores raw provider tool arguments in the file-backed
`ProviderArgumentStore`, receives a content ref, and reads the JSON arguments
back through the same typed store port.

## SDK-Owned Boundary

The SDK owns the file adapter contract, content-ref return shape, by-ref JSON
readback, and the rule that raw provider arguments stay out of journals,
events, debug output, and reports.

## Host-Owned Boundary

The host owns the filesystem root, retention policy, backup behavior, and
permission model around local store files.

## Failure Modes

- Malformed stored JSON returns a typed SDK error during readback.
- Unsafe or unavailable file paths fail through the file-store adapter instead
  of being hidden behind a global store.
