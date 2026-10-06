#!/usr/bin/env bash
# CI selects an exact cached executable; local use retains Cargo's rebuild checks.
set -euo pipefail
source "$(dirname "$0")/container.sh"
cd "$(dirname "$0")/.."
if [[ -n "${MNM_STATS_COLLECTOR_BINARY:-}" ]]; then
    binary="$(realpath -e -- "$MNM_STATS_COLLECTOR_BINARY")"
    exec "$binary" "$@"
fi
exec cargo run --locked --quiet -p mnm-stats-collector -- "$@"
