#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/container.sh"
cd "$(dirname "$0")/.."
exec cargo run --locked --quiet -p mnm-stats-collector -- collect "$@"
