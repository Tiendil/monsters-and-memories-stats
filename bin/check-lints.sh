#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo clippy --locked -p mnm-stats-dashboard --target wasm32-unknown-unknown -- -D warnings
