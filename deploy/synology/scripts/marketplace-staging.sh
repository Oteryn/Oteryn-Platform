#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd -- "$DEPLOY_DIR/../.." && pwd)"
ENV_FILE="${OTERYN_ENV_FILE:-$DEPLOY_DIR/.env}"
BASE_COMPOSE_FILE="$DEPLOY_DIR/compose.yml"
MARKETPLACE_COMPOSE_FILE="$DEPLOY_DIR/compose.marketplace.yml"
ACTION="${1:-verify}"

# shellcheck source=deploy/synology/scripts/lib.sh
source "$SCRIPT_DIR/lib.sh"
load_oteryn_env_file "$ENV_FILE"

case "$ACTION" in
    enable|verify|prepare-rollback) ;;
    *)
        echo "Usage: $0 enable|verify|prepare-rollback" >&2
        exit 2
        ;;
esac

# Marketplace recreates Platform from this env file: never with native admission on and the projection
# identities cleared (deploy.sh's gate would have refused that configuration).
if [[ "$ACTION" != verify && "${OTERYN_NATIVE_ADMISSION_ENABLED:-false}" == true ]]; then
    oteryn_native_admission_projection_ready || exit 1
fi

# The native-only cut removed the character transfer and escrow contract that auction
# creation and reconciliation depend on. Enabling or verifying the Bazaar would report
# success while escrow and settlement cannot run, so refuse until a native replacement
# exists. prepare-rollback stays available to drain and disable an existing deployment.
if [[ "$ACTION" == enable || "$ACTION" == verify ]]; then
    echo "Character Bazaar $ACTION refused: no character transfer/escrow contract is available after the native-only cut." >&2
    exit 1
fi

required_vars=(
    PLATFORM_IMAGE
    APP_NAME APP_ENV APP_KEY APP_DEBUG APP_URL LOG_LEVEL
    MARIADB_ROOT_PASSWORD PLATFORM_DB_NAME PLATFORM_DB_USER PLATFORM_DB_PASSWORD
)
for name in "${required_vars[@]}"; do
    value="${!name:-}"
    if [[ -z "$value" || "$value" == REQUIRED_* ]]; then
        echo "Required Character Bazaar staging value is missing: $name" >&2
        exit 1
    fi
done

safe_value_vars=(
    PLATFORM_DB_NAME PLATFORM_DB_USER PLATFORM_DB_PASSWORD
)
for name in "${safe_value_vars[@]}"; do
    value="${!name}"
    if [[ ! "$value" =~ ^[A-Za-z0-9_.-]+$ ]]; then
        echo "$name contains unsupported characters." >&2
        exit 1
    fi
done

compose=(
    docker compose
    --env-file "$ENV_FILE"
    -f "$BASE_COMPOSE_FILE"
    -f "$MARKETPLACE_COMPOSE_FILE"
)

update_env_value() {
    local key="$1"
    local value="$2"
    local temporary="$ENV_FILE.tmp"

    # Replace the key in place (append it when absent) without an interpreter:
    # the platform runner image has no python3.
    awk -v key="$key" -v value="$value" '
        BEGIN { prefix = key "="; done = 0 }
        index($0, prefix) == 1 && !done { print prefix value; done = 1; next }
        { print }
        END { if (!done) print prefix value }
    ' "$ENV_FILE" >"$temporary"
    chmod --reference="$ENV_FILE" "$temporary" 2>/dev/null || chmod 600 "$temporary"
    mv "$temporary" "$ENV_FILE"
}

service_container_id() {
    local service="$1"
    local ids

    ids="$("${compose[@]}" ps -a -q "$service")"
    if [[ -z "$ids" || "$(printf '%s\n' "$ids" | sed '/^$/d' | wc -l | tr -d ' ')" != "1" ]]; then
        echo "Expected exactly one Compose container for service: $service" >&2
        exit 1
    fi

    printf '%s\n' "$ids"
}

require_running_service() {
    local service="$1"
    local container_id
    local running

    container_id="$(service_container_id "$service")"
    running="$(docker inspect --format '{{.State.Running}}' "$container_id")"
    if [[ "$running" != "true" ]]; then
        echo "Required staging service is not running: $service" >&2
        exit 1
    fi
}

root_sql() {
    "${compose[@]}" exec -T -e MYSQL_PWD="$MARIADB_ROOT_PASSWORD" mariadb \
        mariadb -uroot "$@"
}

verify_current_image_supports_marketplace() {
    local platform_container

    platform_container="$(service_container_id platform)"
    docker exec "$platform_container" sh -ec '
        test -f /var/www/html/app/Marketplace/Actions/ReconcileCharacterAuctions.php
        grep -q "marketplace:reconcile" /var/www/html/routes/console.php
    '
}

assert_no_pending_marketplace_work() {
    local pending_count

    "${compose[@]}" exec -T platform php artisan marketplace:reconcile --limit=1000
    pending_count="$(root_sql -N -B -e \
        "SELECT COUNT(*) FROM \`$PLATFORM_DB_NAME\`.character_auctions WHERE status IN ('escrow_pending','active','settlement_pending','cancel_pending','recovery_required');")"
    if [[ "$pending_count" != "0" ]]; then
        echo "Character Bazaar cannot be disabled or rolled back while non-terminal auctions exist." >&2
        exit 1
    fi
}

probe_marketplace_route() {
    local platform_container

    platform_container="$(service_container_id platform)"
    for _ in $(seq 1 30); do
        if docker run --rm \
            --network "container:$platform_container" \
            alpine:3.22 \
            /bin/sh -ec \
            "wget -qO- -T 5 http://127.0.0.1:8000/en/bazaar >/dev/null"; then
            return 0
        fi
        sleep 2
    done

    echo "Character Bazaar staging route did not become healthy." >&2
    return 1
}

verify_enabled_runtime() {
    local platform_container
    local scheduler_container
    local running

    platform_container="$(service_container_id platform)"
    scheduler_container="$(service_container_id marketplace-scheduler)"
    running="$(docker inspect --format '{{.State.Running}}' "$scheduler_container")"
    if [[ "$running" != "true" ]]; then
        echo "The Character Bazaar scheduler is not running." >&2
        exit 1
    fi

    docker exec "$platform_container" php -r '
        $enabled = getenv("MARKETPLACE_ENABLED");
        exit($enabled === "true" ? 0 : 1);
    '
    docker exec "$scheduler_container" php -r '
        $enabled = getenv("MARKETPLACE_ENABLED");
        exit($enabled === "true" ? 0 : 1);
    '

    probe_marketplace_route
}

for service in mariadb platform internal-proxy gateway; do
    require_running_service "$service"
done
verify_current_image_supports_marketplace

case "$ACTION" in
    enable)
        export MARKETPLACE_ENABLED=false
        update_env_value MARKETPLACE_ENABLED false
        # Ingress stays stopped until the final Platform passes the trust gate (lib.sh).
        oteryn_native_admission_ingress_stop "${compose[@]}" >/dev/null
        "${compose[@]}" up -d --no-deps --force-recreate platform
        "${compose[@]}" exec -T platform php artisan migrate --force --no-interaction
        oteryn_publish_native_admission_trust_or_stop "${compose[@]}" || exit 1
        "${compose[@]}" exec -T platform php artisan marketplace:reconcile --limit=1000

        export MARKETPLACE_ENABLED=true
        update_env_value MARKETPLACE_ENABLED true
        "${compose[@]}" up -d --no-deps --force-recreate platform
        oteryn_publish_native_admission_trust_or_stop "${compose[@]}" || exit 1
        oteryn_native_admission_self_check_or_stop "${compose[@]}" || exit 1
        "${compose[@]}" up -d marketplace-scheduler
        "${compose[@]}" up -d --no-deps --force-recreate internal-proxy
        "${compose[@]}" start gateway
        verify_enabled_runtime
        echo "Character Bazaar staging enablement passed."
        ;;
    verify)
        export MARKETPLACE_ENABLED=true
        update_env_value MARKETPLACE_ENABLED true
        verify_enabled_runtime
        echo "Character Bazaar staging verification passed."
        ;;
    prepare-rollback)
        assert_no_pending_marketplace_work
        export MARKETPLACE_ENABLED=false
        update_env_value MARKETPLACE_ENABLED false
        # Ingress stays stopped until the recreated Platform passes the trust gate (lib.sh).
        oteryn_native_admission_ingress_stop "${compose[@]}" >/dev/null
        "${compose[@]}" up -d --no-deps --force-recreate platform
        oteryn_publish_native_admission_trust_or_stop "${compose[@]}" || exit 1
        oteryn_native_admission_self_check_or_stop "${compose[@]}" || exit 1
        # The scheduler is an always-on base service: it stays up and is recreated with the disabled switch.
        "${compose[@]}" up -d --no-deps --force-recreate marketplace-scheduler
        "${compose[@]}" up -d --no-deps --force-recreate internal-proxy
        "${compose[@]}" start gateway
        if "${compose[@]}" exec -T platform php artisan route:list --name=marketplace.index --json \
            | grep -q 'marketplace.index'; then
            echo "Character Bazaar routes remain registered after rollback preparation." >&2
            exit 1
        fi
        echo "Character Bazaar staging is drained and disabled; image rollback may proceed through the standard workflow."
        ;;
esac
