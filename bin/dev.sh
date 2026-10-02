#!/usr/bin/env bash
# Container setup and arbitrary development commands; product scripts reuse this.
set -euo pipefail
cd "$(dirname "$0")/.."
project_root="$PWD"
export MNM_STATS_UID="$(id -u)" MNM_STATS_GID="$(id -g)"
mkdir -p .cache/docker/cargo .cache/docker/tools target/docker .session/playwright

history="$(realpath -e -- "${MNM_STATS_HISTORY:-data/history.jsonl}")"
export MNM_STATS_HISTORY_DIR="$(dirname "$history")"
export MNM_STATS_CONTAINER_HISTORY="/input-history/$(basename "$history")"

case "${1:-}" in
    setup)
        docker compose build dev
        docker compose pull playwright-mcp
        exec docker compose run --rm --no-deps -T dev cargo fetch --locked
        ;;
    --) shift ;;
    *) echo "Usage: $0 setup | -- COMMAND [ARGS...]" >&2; exit 2 ;;
esac
if [[ $# == 0 ]]; then echo "A command is required." >&2; exit 2; fi
if ! docker image inspect mnm-stats-dev:local >/dev/null 2>&1; then
    echo "Development image missing. Run ./bin/dev.sh setup first." >&2; exit 1
fi
# Translate repository paths used by callers (for example an absolute --dist).
args=()
for arg in "$@"; do
    case "$arg" in
        "$project_root"/*) args+=("/workspace/${arg#"$project_root"/}") ;;
        *) args+=("$arg") ;;
    esac
done
exec docker compose run --rm --no-deps --pull never -T dev "${args[@]}"
