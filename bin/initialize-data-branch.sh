#!/usr/bin/env bash
# One-time operational migration. Requires explicit permission to fetch and push.
# Pause collection and wait for active runs before executing this command.
set -euo pipefail
cd "$(dirname "$0")/.."
project_root="$PWD"
source_branch="${1:?Usage: initialize-data-branch.sh SOURCE_BRANCH}"
git check-ref-format "refs/heads/$source_branch"
if git ls-remote --exit-code --heads origin refs/heads/data; then
    echo "The data branch already exists; refusing to replace it." >&2
    exit 1
else
    status=$?
    if [[ "$status" != 2 ]]; then exit "$status"; fi
fi
git fetch --no-tags origin "refs/heads/$source_branch"
source_revision="$(git rev-parse FETCH_HEAD)"
remote="$(git remote get-url origin)"
mkdir -p .session/data-migration
scratch="$(mktemp -d "$project_root/.session/data-migration/run-XXXXXX")"
mkdir -p "$scratch/data"
git show "$source_revision:data/history.jsonl" > "$scratch/data/history.jsonl"
"$project_root/bin/validate-history.sh" "$scratch/data/history.jsonl"
cat > "$scratch/README.md" <<EOF
# Collected statistics

This branch contains the append-only Monsters & Memories observation archive.
Application code, schema definitions, and workflows live on the default branch.
Do not merge this branch into the code branch or force-push its history.

The initial archive was copied unchanged from source commit $source_revision.
The dashboard embeds this JSONL and publishes an identical downloadable copy.
Each deployment's build-info.json identifies its code and data commits.
EOF
git -C "$scratch" init --initial-branch=data
git -C "$scratch" remote add origin "$remote"
git -C "$scratch" add -- README.md data/history.jsonl
git -C "$scratch" -c user.name='github-actions[bot]' \
    -c user.email='41898282+github-actions[bot]@users.noreply.github.com' \
    commit -m "Initialize statistics archive from $source_revision"
git -C "$scratch" push origin HEAD:refs/heads/data
printf 'Archive initialized from %s. Migration checkout retained at %s\n' "$source_revision" "$scratch"
