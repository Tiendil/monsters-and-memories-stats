#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
exec cargo run --locked --quiet -p mnm-stats-collector -- collect "$@"
