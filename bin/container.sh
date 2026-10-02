#!/usr/bin/env bash
# Sourced by shared commands, which re-enter once inside the development image.
if [[ "${MNM_STATS_CONTAINER:-}" != 1 ]]; then
    exec "$(dirname "${BASH_SOURCE[0]}")/dev.sh" -- "${BASH_SOURCE[1]}" "$@"
fi
