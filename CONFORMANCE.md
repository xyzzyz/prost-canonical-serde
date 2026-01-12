# Conformance testing

This project is validated against the upstream protobuf conformance test suite.
The C++ conformance test runner drives the Rust testee binary provided by
`prost-canonical-serde-conformance`.

## Prerequisites

- The protobuf submodule is checked out (see `protobuf/`).
- CMake + a C++ toolchain are installed to build the conformance runner.

## Build the conformance test runner

From the repository root:

```bash
cmake -S protobuf -B protobuf-build -Dprotobuf_BUILD_CONFORMANCE=ON
cmake --build protobuf-build --target conformance_test_runner
```

This produces `protobuf-build/conformance_test_runner`.

## Run the Rust conformance testee

Build the Rust testee:

```bash
cargo build -p prost-canonical-serde-conformance
```

Run the suite:

```bash
./protobuf-build/conformance_test_runner \
  --failure_list prost-canonical-serde-conformance/conformance/failure_list_rust_cc.txt \
  --text_format_failure_list prost-canonical-serde-conformance/conformance/text_format_failure_list_rust_cc.txt \
  ./target/debug/prost-canonical-serde-conformance
```

The output will list passing tests, expected failures, and any unexpected
failures.

## Skipped tests

The conformance runner also executes the text-format test suite. The
`prost-canonical-serde-conformance` testee only implements protobuf binary and
JSON formats, so the runner marks the text-format cases as skipped. These show
up as the `0 successes, N skipped` summary after the JSON/binary run and are
expected until a text-format implementation is added.

## Known failures

The failure list in
`prost-canonical-serde-conformance/conformance/failure_list_rust_cc.txt` covers
cases that are not supported by prost itself or are intentionally out of scope
for this crate:

- Unknown fields are not preserved by prost, so binary conformance cases that
  check unknown field retention fail.
- MessageSet encoding is not supported by prost, so MessageSet-related binary
  cases fail.
- `Any` JSON support is not implemented in the canonical serde layer yet.
- Enum aliases (case variants or alias names) are not accepted by prost enum
  parsing, so alias-related JSON cases fail.
