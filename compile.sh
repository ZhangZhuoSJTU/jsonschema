#!/bin/bash
# ProgramBench-style build: must produce ./executable at the repo root.
# jsonschema is a workspace; the tested CLI is the jsonschema-cli crate. The
# reference binary was built with the `arbitrary-precision` feature (serde_json
# arbitrary_precision + num-bigint) so multipleOf checks on >2^53 integers keep
# full precision; without it f64 rounding makes non-multiples validate as valid.
set -e
cd "$(dirname "$0")"
cargo build --release -p jsonschema-cli --features jsonschema/arbitrary-precision
cp "target/release/jsonschema-cli" executable
