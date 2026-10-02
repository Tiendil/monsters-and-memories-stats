#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ $# == 0 ]]; then
    set -- data/history.jsonl
fi
exec cargo run --locked --quiet -p mnm-stats-collector -- validate-history "$@"
