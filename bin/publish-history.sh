#!/usr/bin/env bash
# Publish only from the separate data checkout, using checkout's host credentials.
set -euo pipefail
commands="$(cd "$(dirname "$0")" && pwd)"
checkout="${1:?Usage: publish-history.sh DATA_CHECKOUT}"
cd "$(realpath -e -- "$checkout")"
if [[ "$(git rev-parse --show-toplevel)" != "$PWD" || "$(git symbolic-ref --short HEAD)" != data ]]; then
    echo "History must be published from the root of the separate data branch checkout." >&2
    exit 1
fi
observed_at="$("$commands/validate-history.sh" "$PWD/data/history.jsonl" --latest-observed-at)"
if git diff --quiet HEAD -- data/history.jsonl; then
    echo "History unchanged; no commit needed."
    exit 0
else
    status=$?
    if [[ "$status" != 1 ]]; then exit "$status"; fi
fi
if [[ -z "$observed_at" ]]; then
    echo "Cannot publish changed history without an observation." >&2
    exit 1
fi
# --only leaves any unrelated staged or working-tree changes out of this commit.
git -c user.name='github-actions[bot]' \
    -c user.email='41898282+github-actions[bot]@users.noreply.github.com' \
    commit --only -m "collector: record snapshot at $observed_at" -- data/history.jsonl
if ! git push origin HEAD:refs/heads/data; then
    echo "History push failed; the local commit retains the observation. Recover the workflow's history artifact before retrying. No force-push was attempted." >&2
    exit 1
fi
