#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
export MNM_STATS_UID="$(id -u)" MNM_STATS_GID="$(id -g)"
mkdir -p .session/playwright
# Compose joins the browser to the dashboard network; stdout is MCP JSON-RPC.
exec docker compose run --rm --no-deps --pull never -T playwright-mcp
