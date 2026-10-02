#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../mnm-stats/mnm-stats-dashboard"
bindgen_version=$(sed -n 's/^wasm_bindgen = "\(.*\)"/\1/p' Trunk.toml)
if ! command -v wasm-bindgen >/dev/null || [[ "$(wasm-bindgen --version)" != "wasm-bindgen $bindgen_version" ]]; then
    echo "Install wasm-bindgen-cli $bindgen_version before building (see README)." >&2
    exit 1
fi
# Skip Trunk's upstream update notification; the required version is in Trunk.toml.
exec env -u NO_COLOR trunk --color never --skip-version-check build --release "$@"
