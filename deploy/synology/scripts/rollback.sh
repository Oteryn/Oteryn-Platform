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
last_good_file="$state_dir/last-good-release.env"
schema_file="$state_dir/schema-state.env"
active_file="$state_dir/current-release.env"
[[ -f "$state_dir/candidate-release.env" ]] && active_file="$state_dir/candidate-release.env"

for file in "$active_file" "$last_good_file" "$schema_file"; do
    [[ -f "$file" ]] || { echo "Rollback rejected: required compatibility metadata is missing: $file" >&2; exit 1; }
done

bash "$SCRIPT_DIR/release-state.sh" validate "$active_file"
bash "$SCRIPT_DIR/release-state.sh" validate "$last_good_file"

schema_state="$(sed -n 's/^SCHEMA_STATE=//p' "$schema_file" | head -n 1)"
schema_identity="$(sed -n 's/^SCHEMA_COMPATIBILITY_ID=//p' "$schema_file" | head -n 1)"
candidate_sha="$(sed -n 's/^RELEASE_SHA=//p' "$active_file" | head -n 1)"
if [[ "$schema_state" != known || -z "$schema_identity" ]]; then
    echo "Rollback rejected: current database schema identity is unknown or incomplete." >&2
    echo "Image rollback never restores schema. Use recover-schema.sh only with matching verified backup evidence." >&2
    exit 1
fi

bash "$SCRIPT_DIR/release-state.sh" compatible-schema "$schema_identity" "$last_good_file" "$candidate_sha"

# Load immutable last-good runtime identity only after all compatibility gates.
# shellcheck disable=SC1090
source "$last_good_file"
for name in RELEASE_SHA PLATFORM_IMAGE GATEWAY_IMAGE CANARY_IMAGE GAME_WORLD_ID GAME_WORLD_SLUG GAME_WORLD_NAME GAME_WORLD_REGION GAME_WORLD_HOST GAME_WORLD_PORT; do
    [[ -n "${!name:-}" ]] || { echo "Rollback configuration is incomplete: $name" >&2; exit 1; }
done

[[ "${CANARY_GAME_BIND_ADDRESS:-}" == "$GAME_WORLD_HOST" ]] || {
    echo "Rollback rejected: configured Canary bind address does not match persisted last-good world host." >&2
    exit 1
}
[[ "${CANARY_SERVER_IP:-}" == "$GAME_WORLD_HOST" ]] || {
    echo "Rollback rejected: configured Canary server IP does not match persisted last-good world host." >&2
    exit 1
}

GATEWAY_VERSION="sha-$RELEASE_SHA"
export PLATFORM_IMAGE GATEWAY_IMAGE CANARY_IMAGE GATEWAY_VERSION
compose=(docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE")
"${compose[@]}" pull platform gateway canary

# Re-prove that immutable last-good Platform/Gateway artifacts still identify the
# exact persisted application release before starting any rollback runtime.
last_good_revision="$(_oteryn_release_sha_for_images "$PLATFORM_IMAGE" "$GATEWAY_IMAGE")"
[[ "$last_good_revision" == "$RELEASE_SHA" ]] || {
    echo "Rollback rejected: last-good runtime image revision does not match persisted release identity." >&2
    exit 1
}

"${compose[@]}" up -d canary platform internal-proxy gateway

# Marketplace is an optional Platform-image consumer outside the base manifest.
# Reconcile both the browser-facing Platform service and scheduler to the selected
# last-good image/effective state before health checks and release-state promotion.
_oteryn_reconcile_marketplace_scheduler_after_runtime_change

OTERYN_ENV_FILE="$ENV_FILE" bash "$SCRIPT_DIR/health-check.sh"
"${compose[@]}" exec -T platform php artisan game-auth:world:ensure \
    --id="$GAME_WORLD_ID" --slug="$GAME_WORLD_SLUG" --name="$GAME_WORLD_NAME" \
    --region="$GAME_WORLD_REGION" --host="$GAME_WORLD_HOST" --port="$GAME_WORLD_PORT" \
    --status=online --login-enabled=1

cp "$last_good_file" "$state_dir/current-release.env.tmp"
chmod 600 "$state_dir/current-release.env.tmp"
mv "$state_dir/current-release.env.tmp" "$state_dir/current-release.env"
rm -f "$state_dir/candidate-release.env"

echo "Compatible runtime image rollback completed. Database schema was NOT rolled back or changed by this operation."
