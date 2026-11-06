#!/usr/bin/env bash
# Local CI for the Rust port. Fails if formatting, clippy or tests fail.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
