#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/container.sh"
cd "$(dirname "$0")/.."
# Reuse the cache across build identities, then verify the development fallback.
check_build_identity() {
    MNM_STATS_BUILD_REVISION="$1" MNM_STATS_BUILD_BRANCH="$2" MNM_STATS_EXPECT_USER_AGENT="$3" \
        cargo test --locked -p mnm-stats-collector --test acquisition \
        initializes_http_session_then_receives_delayed_local_websocket_updates -- --exact
}
check_build_identity 0123456789abcdef0123456789abcdef01234567 main \
    'mnm-stats-collector/0123456789ab (branch=main; +https://github.com/Tiendil/monsters-and-memories-stats)'
check_build_identity fedcba9876543210fedcba9876543210fedcba98 'feature/(trial)' \
    'mnm-stats-collector/fedcba987654 (branch=feature/\(trial\); +https://github.com/Tiendil/monsters-and-memories-stats)'
check_build_identity '' '' \
    'mnm-stats-collector/dev (+https://github.com/Tiendil/monsters-and-memories-stats)'
