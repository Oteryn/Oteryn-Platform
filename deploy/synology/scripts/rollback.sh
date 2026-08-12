#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd)"
ENV_FILE="${OTERYN_ENV_FILE:-$DEPLOY_DIR/.env}"
COMPOSE_FILE="$DEPLOY_DIR/compose.yml"

# shellcheck source=deploy/synology/scripts/lib.sh
source "$SCRIPT_DIR/lib.sh"
load_oteryn_env_file "$ENV_FILE"

state_dir="${OTERYN_STATE_DIR:-/var/lib/oteryn-staging-state}"
current_file="$state_dir/current-release.env"
last_good_file="$state_dir/last-good-release.env"
schema_file="$state_dir/schema-state.env"

for file in "$current_file" "$last_good_file" "$schema_file"; do
    if [[ ! -f "$file" ]]; then
        echo "Rollback rejected: required compatibility metadata is missing: $file" >&2
        exit 1
    fi
done

# shellcheck disable=SC1090
source "$schema_file"
if [[ "${SCHEMA_STATE:-}" != "known" || -z "${SCHEMA_COMPATIBILITY_ID:-}" ]]; then
    echo "Rollback rejected: current database schema identity is unknown or incomplete." >&2
    echo "Use recover-schema.sh only with a verified pre-migration backup if recovery is required." >&2
    exit 1
fi
schema_identity="$SCHEMA_COMPATIBILITY_ID"

bash "$SCRIPT_DIR/release-state.sh" compatible "$current_file" "$last_good_file"

# Capture the immutable last-good application identity only after compatibility
# has been proven against the current schema.
# shellcheck disable=SC1090
source "$last_good_file"
if [[ "$schema_identity" != "${SCHEMA_COMPATIBILITY_ID:-}" ]] && \
   ! bash "$SCRIPT_DIR/release-state.sh" compatible "$current_file" "$last_good_file" >/dev/null; then
    echo "Rollback rejected: stale schema identity." >&2
    exit 1
fi

for name in \
    PLATFORM_IMAGE GATEWAY_IMAGE CANARY_IMAGE \
    GAME_WORLD_ID GAME_WORLD_SLUG GAME_WORLD_NAME GAME_WORLD_REGION GAME_WORLD_HOST GAME_WORLD_PORT; do
    if [[ -z "${!name:-}" ]]; then
        echo "Rollback configuration is incomplete: $name" >&2
        exit 1
    fi
done

export PLATFORM_IMAGE GATEWAY_IMAGE CANARY_IMAGE
compose=(docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE")

"${compose[@]}" pull platform gateway canary
"${compose[@]}" up -d canary platform internal-proxy gateway

OTERYN_ENV_FILE="$ENV_FILE" bash "$SCRIPT_DIR/health-check.sh"

"${compose[@]}" exec -T platform php artisan game-auth:world:ensure \
    --id="$GAME_WORLD_ID" \
    --slug="$GAME_WORLD_SLUG" \
    --name="$GAME_WORLD_NAME" \
    --region="$GAME_WORLD_REGION" \
    --host="$GAME_WORLD_HOST" \
    --port="$GAME_WORLD_PORT" \
    --status=online \
    --login-enabled=1

echo "Compatible runtime image rollback completed. Database schema was NOT rolled back or changed by this operation."
