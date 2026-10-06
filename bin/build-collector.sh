#!/usr/bin/env bash
# Prepare a standalone executable for reuse by validation and collection.
set -euo pipefail
source "$(dirname "$0")/container.sh"
cd "$(dirname "$0")/.."
destination="${MNM_STATS_COLLECTOR_BINARY:?Set MNM_STATS_COLLECTOR_BINARY to the prepared executable path.}"
cargo build --locked --quiet -p mnm-stats-collector --bin mnm-stats-collector
# Strip the cached copy only; Cargo retains its normal development artifacts.
install -D -s -m 755 "${CARGO_TARGET_DIR}/debug/mnm-stats-collector" "$destination"
