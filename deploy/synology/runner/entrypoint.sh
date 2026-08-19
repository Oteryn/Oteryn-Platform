#!/usr/bin/env bash
set -euo pipefail

RUNNER_URL="${RUNNER_URL:-}"
RUNNER_NAME="${RUNNER_NAME:-oteryn-synology-staging}"
RUNNER_LABELS="${RUNNER_LABELS:-oteryn-staging}"
RUNNER_WORKDIR="${RUNNER_WORKDIR:-/work}"
RUNNER_CONFIG_DIR="${RUNNER_CONFIG_DIR:-/runner}"
RUNNER_DIST_DIR="${RUNNER_DIST_DIR:-/opt/actions-runner-dist}"

mkdir -p "$RUNNER_CONFIG_DIR" "$RUNNER_WORKDIR"

if [[ ! -x "$RUNNER_CONFIG_DIR/run.sh" ]]; then
    cp -a "$RUNNER_DIST_DIR/." "$RUNNER_CONFIG_DIR/"
fi

cd "$RUNNER_CONFIG_DIR"

if [[ ! -f .runner ]]; then
    if [[ -z "$RUNNER_URL" ]]; then
        echo "Runner is not registered. Provide the exact repository RUNNER_URL before first registration." >&2
        exit 1
    fi
    if [[ ! "$RUNNER_URL" =~ ^https://github\.com/[A-Za-z0-9][A-Za-z0-9-]{0,38}/[A-Za-z0-9._-]+$ ]]; then
        echo "RUNNER_URL must be an exact github.com owner/repository URL." >&2
        exit 1
    fi

    token="${RUNNER_TOKEN:-}"
    if [[ -n "${RUNNER_TOKEN_FILE:-}" && -f "$RUNNER_TOKEN_FILE" ]]; then
        token="$(<"$RUNNER_TOKEN_FILE")"
    fi
    if [[ -z "$token" ]]; then
        echo "Runner is not registered. Provide a one-time RUNNER_TOKEN or RUNNER_TOKEN_FILE." >&2
        exit 1
    fi

    ./config.sh \
        --url "$RUNNER_URL" \
        --token "$token" \
        --name "$RUNNER_NAME" \
        --labels "$RUNNER_LABELS" \
        --work "$RUNNER_WORKDIR" \
        --unattended \
        --replace

    unset token RUNNER_TOKEN
fi

exec ./run.sh
