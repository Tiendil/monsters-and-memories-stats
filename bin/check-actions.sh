#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/container.sh"
cd "$(dirname "$0")/.."
exec actionlint -color -shellcheck= -pyflakes=
