#!/usr/bin/env bash
# Resolve a local build input without fetching production data.
set -euo pipefail
project_root="$(cd "$(dirname "$0")/.." && pwd)"
history="${1:-${MNM_STATS_HISTORY:-}}"
if [[ -z "$history" ]]; then
    history="$project_root/data/history.jsonl"
    if [[ ! -f "$history" ]]; then
        history="$project_root/mnm-stats/mnm-stats-dashboard/tests/fixtures/empty-history.jsonl"
    fi
fi
realpath -e -- "$history"
