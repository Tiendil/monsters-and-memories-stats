#!/usr/bin/env bash
# Run from a collector checkout. Git stays on the host to use checkout's credentials.
set -euo pipefail
commands="$(cd "$(dirname "$0")" && pwd)"
branch="${1:?Usage: publish-history.sh BRANCH}"
git check-ref-format "refs/heads/$branch"
cd "$(git rev-parse --show-toplevel)"
"$commands/validate-history.sh" "$PWD/data/history.jsonl"
if git diff --quiet HEAD -- data/history.jsonl; then
    echo "History unchanged; no commit needed."
    exit 0
fi
# --only leaves any unrelated staged or working-tree changes out of this commit.
git -c user.name='github-actions[bot]' \
    -c user.email='41898282+github-actions[bot]@users.noreply.github.com' \
    commit --only -m 'Collect current metrics' -- data/history.jsonl
if ! git push origin "HEAD:refs/heads/$branch"; then
    echo "History push failed; the local commit retains the observation. Recover the workflow's history artifact before retrying. No force-push was attempted." >&2
    exit 1
fi
