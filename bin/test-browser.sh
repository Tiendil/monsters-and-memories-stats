#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/container.sh"
cd "$(dirname "$0")/.."
# Tool installation is a separate setup step, never part of a test run.
for tool in trunk wasm-bindgen python3 "${CHROMEDRIVER:-chromedriver}" "${CHROME:-google-chrome}"; do
    if ! command -v "$tool" >/dev/null; then
        echo "Missing browser-test tool: $tool. Follow the README setup instructions." >&2
        exit 1
    fi
done
exec cargo test --locked -p mnm-stats-dashboard --features browser-tests --test browser -- --nocapture
