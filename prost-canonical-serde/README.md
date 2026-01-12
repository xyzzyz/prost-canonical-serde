# prost-canonical-serde

Canonical JSON encoding for Prost-generated protobuf bindings.

## Why this exists

Protobuf has a canonical JSON mapping that differs from plain Serde JSON (for
example, `int64`/`uint64` are encoded as strings and `bytes` use base64). Prost
provides efficient Rust bindings, but it does not implement canonical JSON on
its own. This project fills that gap by generating `serde::Serialize` and
`serde::Deserialize` implementations that follow the protobuf canonical JSON
spec, while keeping the normal `serde_json` API surface.

## Highlights

- Seamless Prost integration: derive macros and build helpers work with
  prost-generated message types.
- Well-known types support: `prost-types` (Timestamp, Duration, Any, Struct,
  etc.) are handled with their canonical JSON mappings.
- `no_std` friendly: the core crate works without `std` (alloc required).
- High conformance: validated against the upstream protobuf conformance test
  suite. Remaining non-conformance aligns with limitations in Prost itself
  (for example, unknown field preservation and MessageSet support).

## Quick start

Use the derive macros for prost-generated types and keep using `serde_json`.
See the crate documentation on
[docs.rs](https://docs.rs/prost-canonical-serde/latest/prost_canonical_serde/)
for a full end-to-end example with a `.proto`, `build.rs`, and a runnable usage
snippet.

## License

Apache-2.0. See `LICENSE`.
