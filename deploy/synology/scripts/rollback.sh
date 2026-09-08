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
for name in RELEASE_SHA PLATFORM_SOURCE_SHA GATEWAY_SOURCE_SHA PLATFORM_IMAGE GATEWAY_IMAGE CANARY_IMAGE GAME_WORLD_ID GAME_WORLD_SLUG GAME_WORLD_NAME GAME_WORLD_REGION GAME_WORLD_HOST GAME_WORLD_PORT; do
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

OTERYN_RELEASE_SHA="$RELEASE_SHA"
GATEWAY_VERSION="sha-$GATEWAY_SOURCE_SHA"
export OTERYN_RELEASE_SHA PLATFORM_SOURCE_SHA GATEWAY_SOURCE_SHA PLATFORM_IMAGE GATEWAY_IMAGE CANARY_IMAGE GATEWAY_VERSION

rollback_env="$state_dir/rollback-health.env"
cleanup() { rm -f "$rollback_env" "$rollback_env.tmp"; }
trap cleanup EXIT
awk \
    -v release_sha="$OTERYN_RELEASE_SHA" \
    -v platform_source_sha="$PLATFORM_SOURCE_SHA" \
    -v gateway_source_sha="$GATEWAY_SOURCE_SHA" \
    -v platform_image="$PLATFORM_IMAGE" \
    -v gateway_image="$GATEWAY_IMAGE" \
    -v canary_image="$CANARY_IMAGE" \
    -v gateway_version="$GATEWAY_VERSION" '
    BEGIN { r=0; ps=0; gs=0; p=0; g=0; c=0; v=0 }
    /^OTERYN_RELEASE_SHA=/ { print "OTERYN_RELEASE_SHA=" release_sha; r=1; next }
    /^PLATFORM_SOURCE_SHA=/ { print "PLATFORM_SOURCE_SHA=" platform_source_sha; ps=1; next }
    /^GATEWAY_SOURCE_SHA=/ { print "GATEWAY_SOURCE_SHA=" gateway_source_sha; gs=1; next }
    /^PLATFORM_IMAGE=/ { print "PLATFORM_IMAGE=" platform_image; p=1; next }
    /^GATEWAY_IMAGE=/ { print "GATEWAY_IMAGE=" gateway_image; g=1; next }
    /^CANARY_IMAGE=/ { print "CANARY_IMAGE=" canary_image; c=1; next }
    /^GATEWAY_VERSION=/ { print "GATEWAY_VERSION=" gateway_version; v=1; next }
    { print }
    END { if (!(r && ps && gs && p && g && c && v)) exit 42 }
' "$ENV_FILE" >"$rollback_env.tmp" || {
    echo "Rollback rejected: unable to construct bounded last-good runtime environment." >&2
    exit 1
}
chmod 600 "$rollback_env.tmp"
mv "$rollback_env.tmp" "$rollback_env"

compose=(
    env
    "OTERYN_RELEASE_SHA=$OTERYN_RELEASE_SHA"
    "PLATFORM_SOURCE_SHA=$PLATFORM_SOURCE_SHA"
    "GATEWAY_SOURCE_SHA=$GATEWAY_SOURCE_SHA"
    "PLATFORM_IMAGE=$PLATFORM_IMAGE"
    "GATEWAY_IMAGE=$GATEWAY_IMAGE"
    "CANARY_IMAGE=$CANARY_IMAGE"
    "GATEWAY_VERSION=$GATEWAY_VERSION"
    docker compose --env-file "$rollback_env" -f "$COMPOSE_FILE"
)
"${compose[@]}" pull platform gateway canary

# Re-prove each immutable last-good artifact against its own persisted source
# identity. The overall release SHA intentionally remains independent.
_oteryn_verify_component_image_source "$PLATFORM_IMAGE" "$PLATFORM_SOURCE_SHA" Platform
_oteryn_verify_component_image_source "$GATEWAY_IMAGE" "$GATEWAY_SOURCE_SHA" Gateway

"${compose[@]}" up -d canary platform internal-proxy gateway

# Marketplace is an optional Platform-image consumer outside the base manifest.
# Reconcile both the browser-facing Platform service and scheduler to the selected
# last-good image/effective state before health checks and release-state promotion.
OTERYN_ENV_FILE="$rollback_env" _oteryn_reconcile_marketplace_scheduler_after_runtime_change

OTERYN_ENV_FILE="$rollback_env" bash "$SCRIPT_DIR/health-check.sh"
"${compose[@]}" exec -T platform php artisan game-auth:world:ensure \
    --id="$GAME_WORLD_ID" --slug="$GAME_WORLD_SLUG" --name="$GAME_WORLD_NAME" \
    --region="$GAME_WORLD_REGION" --host="$GAME_WORLD_HOST" --port="$GAME_WORLD_PORT" \
    --status=online --login-enabled=1

cp "$last_good_file" "$state_dir/current-release.env.tmp"
chmod 600 "$state_dir/current-release.env.tmp"
mv "$state_dir/current-release.env.tmp" "$state_dir/current-release.env"
rm -f "$state_dir/candidate-release.env"

echo "Compatible runtime image rollback completed. Database schema was NOT rolled back or changed by this operation."
