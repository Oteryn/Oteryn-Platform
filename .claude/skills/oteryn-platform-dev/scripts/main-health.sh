#!/usr/bin/env bash
# Show the last push runs of one workflow on main and which jobs failed, so a
# merge decision starts from the real state of main.
# Usage: main-health.sh [workflow-file] [runs]
#   default workflow portal-rs-ci.yml, default 10 runs; needs gh with REST access.
set -euo pipefail
wf="${1:-portal-rs-ci.yml}"
n="${2:-10}"
repo="${OTERYN_REPO:-Oteryn/Oteryn-Platform}"

gh api "repos/$repo/actions/workflows/$wf/runs?branch=main&event=push&per_page=$n" \
  --jq '.workflow_runs[] | [.id, .head_sha[0:10], .created_at[0:16], (.conclusion // .status)] | @tsv' |
while IFS=$'\t' read -r id sha at state; do
  failed=""
  if [ "$state" = "failure" ]; then
    failed=$(gh api "repos/$repo/actions/runs/$id/jobs?per_page=100" \
      --jq '[.jobs[] | select(.conclusion == "failure" and .name != "platform-gate") | .name] | join("; ")')
  fi
  printf '%s  %s  %-9s %s\n' "$at" "$sha" "$state" "$failed"
done
