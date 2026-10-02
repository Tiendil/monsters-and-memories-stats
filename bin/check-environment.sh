#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
docker compose config --quiet
# Configuration and shell validation do not install tools or start services.
./bin/dev.sh -- bash -c 'for script in bin/*.sh; do bash -n "$script"; done'
