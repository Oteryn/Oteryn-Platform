#!/usr/bin/env bash
set -euo pipefail

RUNNER_SCOPE="${RUNNER_SCOPE:-repository}"
RUNNER_URL="${RUNNER_URL:-}"
RUNNER_GROUP="${RUNNER_GROUP:-}"
RUNNER_NAME="${RUNNER_NAME-}"
RUNNER_LABELS="${RUNNER_LABELS-}"
RUNNER_WORKDIR="${RUNNER_WORKDIR:-/work}"
RUNNER_CONFIG_DIR="${RUNNER_CONFIG_DIR:-/runner}"
RUNNER_DIST_DIR="${RUNNER_DIST_DIR:-/opt/actions-runner-dist}"
RUNNER_TOKEN_FILE="${RUNNER_TOKEN_FILE:-}"

mkdir -p "$RUNNER_CONFIG_DIR" "$RUNNER_WORKDIR"

if [[ ! -x "$RUNNER_CONFIG_DIR/run.sh" ]]; then
    cp -a "$RUNNER_DIST_DIR/." "$RUNNER_CONFIG_DIR/"
fi

cd "$RUNNER_CONFIG_DIR"

if [[ ! -f .runner ]]; then
    if [[ -z "$RUNNER_URL" ]]; then
        if [[ "$RUNNER_SCOPE" == "repository" ]]; then
            echo "Runner is not registered. Provide the exact repository RUNNER_URL before first registration." >&2
        else
            echo "Runner is not registered. Provide the exact organization RUNNER_URL before first registration." >&2
        fi
        exit 1
    fi

    case "$RUNNER_SCOPE" in
        repository)
            RUNNER_NAME="${RUNNER_NAME:-oteryn-synology-staging}"
            RUNNER_LABELS="${RUNNER_LABELS:-oteryn-staging}"
            if [[ ! "$RUNNER_URL" =~ ^https://github\.com/[A-Za-z0-9][A-Za-z0-9-]{0,38}/[A-Za-z0-9._-]+$ ]]; then
                echo "RUNNER_URL must be an exact github.com owner/repository URL for repository scope." >&2
                exit 1
            fi
            if [[ -n "$RUNNER_GROUP" ]]; then
                echo "RUNNER_GROUP is not valid for repository-scoped registration." >&2
                exit 1
            fi
            ;;
        organization)
            if [[ ! "$RUNNER_URL" =~ ^https://github\.com/[A-Za-z0-9][A-Za-z0-9-]{0,38}$ ]]; then
                echo "RUNNER_URL must be an exact github.com organization URL for organization scope." >&2
                exit 1
            fi
            if [[ ! "$RUNNER_GROUP" =~ ^[A-Za-z0-9][A-Za-z0-9._-]{0,99}$ ]]; then
                echo "RUNNER_GROUP is required for organization scope and must use a strict group name." >&2
                exit 1
            fi
            if [[ -z "$RUNNER_NAME" ]]; then
                echo "RUNNER_NAME is required for organization scope." >&2
                exit 1
            fi
            if [[ -z "$RUNNER_LABELS" ]]; then
                echo "RUNNER_LABELS is required for organization scope." >&2
                exit 1
            fi
            ;;
        *)
            echo "RUNNER_SCOPE must be exactly repository or organization." >&2
            exit 1
            ;;
    esac

    if [[ ! "$RUNNER_NAME" =~ ^[A-Za-z0-9][A-Za-z0-9._-]{0,99}$ ]]; then
        echo "RUNNER_NAME must use only alphanumeric, dot, underscore or hyphen characters." >&2
        exit 1
    fi
    if [[ ! "$RUNNER_LABELS" =~ ^[A-Za-z0-9][A-Za-z0-9._-]{0,99}(,[A-Za-z0-9][A-Za-z0-9._-]{0,99})*$ ]]; then
        echo "RUNNER_LABELS must be a non-empty comma-separated list of strict custom labels." >&2
        exit 1
    fi

    token="${RUNNER_TOKEN:-}"
    if [[ -n "$RUNNER_TOKEN_FILE" ]]; then
        if [[ ! -f "$RUNNER_TOKEN_FILE" || ! -r "$RUNNER_TOKEN_FILE" ]]; then
            echo "RUNNER_TOKEN_FILE must reference a readable one-time registration token file." >&2
            exit 1
        fi
        token="$(<"$RUNNER_TOKEN_FILE")"
    fi
    if [[ -z "$token" ]]; then
        echo "Runner is not registered. Provide a one-time RUNNER_TOKEN or RUNNER_TOKEN_FILE." >&2
        exit 1
    fi

    config_args=(
        --url "$RUNNER_URL"
        --token "$token"
        --name "$RUNNER_NAME"
        --labels "$RUNNER_LABELS"
        --no-default-labels
        --work "$RUNNER_WORKDIR"
        --unattended
        --replace
    )
    if [[ "$RUNNER_SCOPE" == "organization" ]]; then
        config_args+=(--runnergroup "$RUNNER_GROUP")
    fi

    ./config.sh "${config_args[@]}"

    unset token RUNNER_TOKEN
fi

exec ./run.sh
