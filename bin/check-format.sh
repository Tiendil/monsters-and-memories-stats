#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
exec cargo fmt --all -- --check
