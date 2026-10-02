#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../mnm-stats/mnm-stats-dashboard"
exec env -u NO_COLOR trunk --color never --skip-version-check serve "$@"
