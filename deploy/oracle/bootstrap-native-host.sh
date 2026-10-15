#!/usr/bin/env bash
# Protected Actions entry point. No credential is written to disk or arguments.
set -euo pipefail
umask 077
[[ $EUID == 0 ]] || { echo 'Run as root.' >&2; exit 1; }
action="${1:-inspect}"
case "$action" in inspect|preflight|initialize) ;; *) exit 2;; esac
SOURCE="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
if [[ $action != inspect ]]; then
    [[ ${GITHUB_ACTIONS:-} == true && ${GITHUB_REPOSITORY:-} == Oteryn/Oteryn-Platform \
        && ${GITHUB_REF:-} == refs/heads/main && ${GITHUB_SHA:-} =~ ^[0-9a-f]{40}$ \
        && ${GITHUB_RUN_ID:-} =~ ^[1-9][0-9]*$ ]] || { echo 'Protected main Actions context required.' >&2; exit 1; }
    IFS= read -r GH_TOKEN || [[ -n ${GH_TOKEN:-} ]]
    [[ -n ${GH_TOKEN:-} ]] || { echo 'The short-lived Actions token is missing.' >&2; exit 1; }
    export GH_TOKEN
fi
docker build --platform linux/arm64 -t oteryn-oracle-bootstrap-ops \
    -f "$SOURCE/deploy/oracle/ops.Dockerfile" "$SOURCE/deploy/oracle" >&2
# The read-only named volume proves the real Docker volume/device mapping to the
# existing location validator. The evidence bind is separate and restore-excluded.
docker run --rm --name "oteryn-oracle-bootstrap-${GITHUB_RUN_ID:-inspect}" \
    --hostname "oteryn-oracle-bootstrap-${GITHUB_RUN_ID:-inspect}" \
    --label com.docker.compose.project=oteryn-oracle \
    --env GH_TOKEN --env GITHUB_ACTIONS --env GITHUB_REPOSITORY --env GITHUB_REF \
    --env GITHUB_SHA --env GITHUB_RUN_ID --env OTERYN_CUTOVER_ANCHOR_ISSUE \
    --env OTERYN_ENV_FILE=/opt/oteryn-oracle/platform.env \
    --mount type=bind,source="$SOURCE",target=/src,readonly \
    --mount type=bind,source=/var/run/docker.sock,target=/var/run/docker.sock \
    --mount type=bind,source=/opt/oteryn-oracle/platform.env,target=/opt/oteryn-oracle/platform.env,readonly \
    --mount type=bind,source=/srv/oteryn-attempt-evidence,target=/var/lib/oteryn-attempt-evidence \
    --mount type=volume,source=oteryn-oracle_platform_storage,target=/oracle-platform-storage,readonly \
    oteryn-oracle-bootstrap-ops "$action"
