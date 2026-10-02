#!/usr/bin/env bash
# Deliberately failing operational notification probe; never contacts the source.
set -euo pipefail
source "$(dirname "$0")/container.sh"
cd "$(dirname "$0")/.."
mkdir -p .session/notification-check
probe="$(mktemp -d .session/notification-check/run-XXXXXX)"
printf '%s\n' '{invalid fixture' > "$probe/source.json"
echo "Intentional notification verification: replaying a local invalid fixture." >&2
./bin/collect.sh --history "$probe/history.jsonl" --replay "$probe/source.json"
echo "The invalid fixture unexpectedly succeeded; notification verification must fail." >&2
exit 1
