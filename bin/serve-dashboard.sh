#!/usr/bin/env bash
set -euo pipefail
project_root="$(cd "$(dirname "$0")/.." && pwd)"
history="${MNM_STATS_HISTORY:-}"
tokens="${MNM_STATS_TOKENS:-$project_root/mnm-stats/mnm-stats-dashboard/design-tokens.tokens.json}"
demo=0
explicit_history=0
args=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        --history)
            [[ $# -ge 2 ]] || { echo "--history requires a path" >&2; exit 2; }
            history="$2"; explicit_history=1; shift 2 ;;
        --demo) demo=1; shift ;;
        --port)
            [[ $# -ge 2 ]] || { echo "--port requires a number" >&2; exit 2; }
            export MNM_STATS_PORT="$2"; shift 2 ;;
        -h|--help)
            echo "Usage: $0 [--demo | --history PATH] [--port PORT] [TRUNK OPTIONS...]"
            exit 0 ;;
        *) args+=("$1"); shift ;;
    esac
done
if (( demo && explicit_history )); then
    echo "Use either --demo or --history." >&2; exit 2
fi
if (( demo )); then history="$project_root/mnm-stats/mnm-stats-dashboard/tests/fixtures/empty-history.jsonl"; fi
if [[ ! "${MNM_STATS_PORT:-8080}" =~ ^[1-9][0-9]{0,4}$ ]] || (( ${MNM_STATS_PORT:-8080} > 65535 )); then
    echo "--port must be between 1 and 65535." >&2; exit 2
fi
history="$("$project_root/bin/history-input.sh" "$history")"
tokens="$(realpath -e -- "$tokens")"
cd "$project_root"
if [[ "${MNM_STATS_CONTAINER:-}" != 1 ]]; then
    if ! docker image inspect mnm-stats-dev:local >/dev/null 2>&1; then
        echo "Development image missing. Run ./bin/dev.sh setup first." >&2; exit 1
    fi
    export MNM_STATS_UID="$(id -u)" MNM_STATS_GID="$(id -g)"
    export MNM_STATS_HISTORY_DIR="$(dirname "$history")"
    export MNM_STATS_CONTAINER_HISTORY="/input-history/$(basename "$history")"
    export MNM_STATS_TOKENS_DIR="$(dirname "$tokens")"
    export MNM_STATS_CONTAINER_TOKENS="/input-tokens/$(basename "$tokens")"
    mkdir -p .cache/docker/cargo .cache/docker/tools target/docker .session/preview
    printf 'Preview: http://127.0.0.1:%s/\n' "${MNM_STATS_PORT:-8080}"
    echo "Browser MCP: http://dashboard:8080/"
    for i in "${!args[@]}"; do
        case "${args[$i]}" in
            "$project_root"/*) args[$i]="/workspace/${args[$i]#"$project_root"/}" ;;
        esac
    done
    if (( demo )); then args=(--demo "${args[@]}"); fi
    exec docker compose run --rm --no-deps --pull never --service-ports --use-aliases -T dashboard \
        ./bin/serve-dashboard.sh "${args[@]}"
fi

export MNM_STATS_DEMO="$demo"
if (( demo )); then
    mkdir -p .session/preview
    history="$project_root/.session/preview/demo-history.jsonl"
    demo_args=()
    if [[ -n "${MNM_STATS_DEMO_AT:-}" ]]; then demo_args+=("$MNM_STATS_DEMO_AT"); fi
    cargo run --locked --quiet -p mnm-stats-dashboard --example demo-history -- "${demo_args[@]}" > "$history.tmp"
    mv "$history.tmp" "$history"
fi
export MNM_STATS_HISTORY="$history"
export MNM_STATS_TOKENS="$tokens"
printf 'History: %s\n' "$history"
./bin/validate-history.sh "$history"
cd mnm-stats/mnm-stats-dashboard
# History may live outside this crate, which is Trunk's default watch root.
# Separate output prevents release checks from replacing a running preview.
# Polling keeps watching the path after an editor/collector atomically replaces its inode.
exec env -u NO_COLOR TRUNK_BUILD_DIST="$project_root/.session/preview/dist" trunk --color never --skip-version-check serve \
    --address 0.0.0.0 --port "${MNM_STATS_PORT:-8080}" \
    --poll --poll-interval 1s \
    --watch "$project_root/mnm-stats" --watch "$project_root/Cargo.toml" \
    --watch "$project_root/Cargo.lock" --watch "$history" --watch "$tokens" "${args[@]}"
