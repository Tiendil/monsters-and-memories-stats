#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/container.sh"
cd "$(dirname "$0")/.."
if [[ $# == 0 ]]; then
    set -- "$(./bin/history-input.sh)"
fi
exec ./bin/run-collector.sh validate-history "$@"
