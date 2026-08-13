#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd -- "$DEPLOY_DIR/../.." && pwd)"
ENV_FILE="${OTERYN_ENV_FILE:-$DEPLOY_DIR/.env}"
COMPOSE_FILE="$DEPLOY_DIR/compose.yml"

# shellcheck source=deploy/synology/scripts/lib.sh
source "$SCRIPT_DIR/lib.sh"
load_oteryn_env_file "$ENV_FILE"

required_vars=(
    PLATFORM_IMAGE GATEWAY_IMAGE CANARY_IMAGE
    PLATFORM_BIND_ADDRESS GATEWAY_BIND_ADDRESS CANARY_LOGIN_BIND_ADDRESS CANARY_GAME_BIND_ADDRESS
    PLATFORM_PORT GATEWAY_PORT CANARY_LOGIN_PORT CANARY_GAME_PORT CANARY_SERVER_IP
    GAME_WORLD_ID GAME_WORLD_SLUG GAME_WORLD_NAME GAME_WORLD_REGION GAME_WORLD_HOST GAME_WORLD_PORT
    APP_KEY MARIADB_ROOT_PASSWORD PLATFORM_DB_NAME PLATFORM_DB_USER PLATFORM_DB_PASSWORD
    CANARY_DB_NAME CANARY_DB_USER CANARY_DB_PASSWORD
    CANARY_READONLY_DB_USER CANARY_READONLY_DB_PASSWORD
    CANARY_PROVISIONING_DB_USER CANARY_PROVISIONING_DB_PASSWORD
    CANARY_CHARACTER_CREATE_DB_USER CANARY_CHARACTER_CREATE_DB_PASSWORD
    REDIS_PASSWORD CANARY_RUNTIME_REDIS_USERNAME CANARY_RUNTIME_REDIS_PASSWORD
    OTERYN_PLATFORM_SERVICE_TOKEN GAME_AUTH_GATEWAY_SERVICE_TOKEN_SHA256
    GAME_SESSION_SERVICE_TOKEN CANARY_GAME_SESSION_SERVICE_TOKEN_SHA256
)

for name in "${required_vars[@]}"; do
    value="${!name:-}"
    if [[ -z "$value" || "$value" == REQUIRED_* ]]; then
        echo "Required staging value is missing: $name" >&2
        exit 1
    fi
done

safe_credential_vars=(
    PLATFORM_DB_NAME PLATFORM_DB_USER PLATFORM_DB_PASSWORD
    CANARY_DB_NAME CANARY_DB_USER CANARY_DB_PASSWORD
    CANARY_READONLY_DB_USER CANARY_READONLY_DB_PASSWORD
    CANARY_PROVISIONING_DB_USER CANARY_PROVISIONING_DB_PASSWORD
    CANARY_CHARACTER_CREATE_DB_USER CANARY_CHARACTER_CREATE_DB_PASSWORD
    REDIS_PASSWORD CANARY_RUNTIME_REDIS_USERNAME CANARY_RUNTIME_REDIS_PASSWORD
    OTERYN_PLATFORM_SERVICE_TOKEN GAME_SESSION_SERVICE_TOKEN
)
for name in "${safe_credential_vars[@]}"; do
    value="${!name}"
    if [[ ! "$value" =~ ^[A-Za-z0-9_.-]+$ ]]; then
        echo "$name contains unsupported characters; use generated hex/alphanumeric staging values." >&2
        exit 1
    fi
done

if [[ ! "$APP_KEY" =~ ^base64:[A-Za-z0-9+/=]+$ ]]; then
    echo "APP_KEY must be a Laravel base64 application key." >&2
    exit 1
fi
if [[ ! "$GAME_AUTH_GATEWAY_SERVICE_TOKEN_SHA256" =~ ^[A-Fa-f0-9]{64}$ ]]; then
    echo "GAME_AUTH_GATEWAY_SERVICE_TOKEN_SHA256 must be a 64-character SHA-256 hex digest." >&2
    exit 1
fi
if [[ ! "$CANARY_GAME_SESSION_SERVICE_TOKEN_SHA256" =~ ^[A-Fa-f0-9]{64}$ ]]; then
    echo "CANARY_GAME_SESSION_SERVICE_TOKEN_SHA256 must be a 64-character SHA-256 hex digest." >&2
    exit 1
fi

expected_platform_hash="$(printf '%s' "$OTERYN_PLATFORM_SERVICE_TOKEN" | sha256sum | awk '{print $1}')"
expected_session_hash="$(printf '%s' "$GAME_SESSION_SERVICE_TOKEN" | sha256sum | awk '{print $1}')"
if [[ "$expected_platform_hash" != "${GAME_AUTH_GATEWAY_SERVICE_TOKEN_SHA256,,}" ]]; then
    echo "Gateway -> Platform service token/hash mismatch." >&2
    exit 1
fi
if [[ "$expected_session_hash" != "${CANARY_GAME_SESSION_SERVICE_TOKEN_SHA256,,}" ]]; then
    echo "Gateway -> Canary service token/hash mismatch." >&2
    exit 1
fi

if ! command -v docker >/dev/null 2>&1; then
    echo "docker is required on the deployment runner." >&2
    exit 1
fi
if ! docker compose version >/dev/null 2>&1; then
    echo "Docker Compose v2 plugin is required on the deployment runner." >&2
    exit 1
fi
if ! command -v python3 >/dev/null 2>&1; then
    echo "python3 is required on the deployment runner." >&2
    exit 1
fi

validate_ipv4_policy() {
    local address="$1"
    local policy="$2"
    python3 - "$address" "$policy" <<'PY'
import ipaddress
import sys

try:
    address = ipaddress.ip_address(sys.argv[1])
except ValueError as exc:
    raise SystemExit(str(exc)) from exc

policy = sys.argv[2]
if address.version != 4 or address.is_unspecified or address.is_multicast or address.is_link_local:
    raise SystemExit('address must be a usable IPv4 address')
if policy == 'loopback' and str(address) != '127.0.0.1':
    raise SystemExit('service must remain bound to exact loopback 127.0.0.1')
if policy == 'private-or-loopback' and not (address.is_private or address.is_loopback):
    raise SystemExit('game bind must be loopback or an RFC1918 private IPv4 address')
PY
}

validate_port() {
    local name="$1"
    local value="${!name}"
    if [[ ! "$value" =~ ^[1-9][0-9]*$ ]] || (( value > 65535 )); then
        echo "$name must be an integer in 1..65535." >&2
        exit 1
    fi
}

if ! validate_ipv4_policy "$PLATFORM_BIND_ADDRESS" loopback; then
    echo "PLATFORM_BIND_ADDRESS is invalid; Platform must remain loopback-only." >&2
    exit 1
fi
if ! validate_ipv4_policy "$GATEWAY_BIND_ADDRESS" loopback; then
    echo "GATEWAY_BIND_ADDRESS is invalid; Gateway must remain loopback-only." >&2
    exit 1
fi
if ! validate_ipv4_policy "$CANARY_LOGIN_BIND_ADDRESS" loopback; then
    echo "CANARY_LOGIN_BIND_ADDRESS is invalid; legacy login must remain loopback-only." >&2
    exit 1
fi
if ! validate_ipv4_policy "$CANARY_GAME_BIND_ADDRESS" private-or-loopback; then
    echo "CANARY_GAME_BIND_ADDRESS must be loopback or an exact private LAN IPv4 address." >&2
    exit 1
fi
if [[ "$CANARY_SERVER_IP" != "$CANARY_GAME_BIND_ADDRESS" ]]; then
    echo "CANARY_SERVER_IP must exactly match CANARY_GAME_BIND_ADDRESS." >&2
    exit 1
fi
if [[ "$GAME_WORLD_HOST" != "$CANARY_GAME_BIND_ADDRESS" ]]; then
    echo "GAME_WORLD_HOST must exactly match CANARY_GAME_BIND_ADDRESS." >&2
    exit 1
fi

for port_name in PLATFORM_PORT GATEWAY_PORT CANARY_LOGIN_PORT CANARY_GAME_PORT GAME_WORLD_PORT; do
    validate_port "$port_name"
done
if [[ "$GAME_WORLD_PORT" != "$CANARY_GAME_PORT" ]]; then
    echo "GAME_WORLD_PORT must exactly match CANARY_GAME_PORT." >&2
    exit 1
fi
if [[ ! "$GAME_WORLD_ID" =~ ^[1-9][0-9]*$ ]]; then
    echo "GAME_WORLD_ID must be a positive integer." >&2
    exit 1
fi
if [[ ! "$GAME_WORLD_SLUG" =~ ^[a-z0-9][a-z0-9-]{0,63}$ ]]; then
    echo "GAME_WORLD_SLUG must be a bounded lower-case slug." >&2
    exit 1
fi
if [[ ! "$GAME_WORLD_REGION" =~ ^[A-Za-z0-9_-]{1,32}$ ]]; then
    echo "GAME_WORLD_REGION must be a simple bounded label." >&2
    exit 1
fi
if (( ${#GAME_WORLD_NAME} < 1 || ${#GAME_WORLD_NAME} > 100 )); then
    echo "GAME_WORLD_NAME must contain 1..100 characters." >&2
    exit 1
fi

compose=(docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE")
state_dir="${OTERYN_STATE_DIR:-/var/lib/oteryn-staging-state}"
mariadb_ready_timeout_seconds="${OTERYN_MARIADB_READY_TIMEOUT_SECONDS:-420}"
if [[ ! "$mariadb_ready_timeout_seconds" =~ ^[1-9][0-9]*$ ]]; then
    echo "OTERYN_MARIADB_READY_TIMEOUT_SECONDS must be a positive integer." >&2
    exit 1
fi
mkdir -p "$state_dir"
chmod 700 "$state_dir"

stage_bootstrap_files() {
    local tls_container

    "${compose[@]}" create tls-init >/dev/null
    tls_container="$("${compose[@]}" ps -a -q tls-init)"
    if [[ -z "$tls_container" ]]; then
        echo "Unable to create the TLS bootstrap container." >&2
        exit 1
    fi

    docker cp "$DEPLOY_DIR/tls/init.sh" "$tls_container:/bootstrap/tls-init.sh"
    docker cp "$DEPLOY_DIR/nginx/internal.conf" "$tls_container:/bootstrap/internal-nginx.conf"
}

snapshot_current_images() {
    local tmp="$state_dir/last-good.env.tmp"
    local found=0
    local service container_id image_id image
    : > "$tmp"
    chmod 600 "$tmp"

    for service in platform gateway canary; do
        container_id="$("${compose[@]}" ps -q "$service" 2>/dev/null || true)"
        if [[ -n "$container_id" ]]; then
            image_id="$(docker inspect --format '{{.Image}}' "$container_id")"
            image="$(bash "$SCRIPT_DIR/release-state.sh" resolve-image "$image_id")"
            case "$service" in
                platform) printf 'PLATFORM_IMAGE=%q\n' "$image" >> "$tmp" ;;
                gateway) printf 'GATEWAY_IMAGE=%q\n' "$image" >> "$tmp" ;;
                canary) printf 'CANARY_IMAGE=%q\n' "$image" >> "$tmp" ;;
            esac
            found=1
        fi
    done

    if [[ "$found" -eq 1 ]]; then
        mv "$tmp" "$state_dir/last-good.env"
    else
        rm -f "$tmp"
    fi
}

apply_sql_template() {
    local template="$1"
    local awk_script="$2"

    awk "$awk_script" "$template" \
        | "${compose[@]}" exec -T -e MYSQL_PWD="$MARIADB_ROOT_PASSWORD" mariadb \
            mariadb -uroot
}

snapshot_current_images

"${compose[@]}" config --quiet
"${compose[@]}" pull
stage_bootstrap_files
"${compose[@]}" up -d mariadb redis tls-init

mariadb_deadline=$((SECONDS + mariadb_ready_timeout_seconds))
while ! "${compose[@]}" exec -T -e MYSQL_PWD="$MARIADB_ROOT_PASSWORD" mariadb \
    mariadb -uroot -N -e 'SELECT 1' >/dev/null 2>&1; do
    if (( SECONDS >= mariadb_deadline )); then
        echo "MariaDB did not become ready within ${mariadb_ready_timeout_seconds} seconds; refusing to continue deployment." >&2
        "${compose[@]}" ps mariadb >&2 || true
        "${compose[@]}" logs --no-color --tail 100 mariadb >&2 || true
        exit 1
    fi
    sleep 2
done

"${compose[@]}" exec -T -e MYSQL_PWD="$MARIADB_ROOT_PASSWORD" mariadb mariadb -uroot <<SQL
CREATE DATABASE IF NOT EXISTS \`$PLATFORM_DB_NAME\` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
CREATE USER IF NOT EXISTS '$PLATFORM_DB_USER'@'%' IDENTIFIED BY '$PLATFORM_DB_PASSWORD';
ALTER USER '$PLATFORM_DB_USER'@'%' IDENTIFIED BY '$PLATFORM_DB_PASSWORD';
GRANT ALL PRIVILEGES ON \`$PLATFORM_DB_NAME\`.* TO '$PLATFORM_DB_USER'@'%';
CREATE DATABASE IF NOT EXISTS \`$CANARY_DB_NAME\` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
CREATE USER IF NOT EXISTS '$CANARY_DB_USER'@'%' IDENTIFIED BY '$CANARY_DB_PASSWORD';
ALTER USER '$CANARY_DB_USER'@'%' IDENTIFIED BY '$CANARY_DB_PASSWORD';
GRANT ALL PRIVILEGES ON \`$CANARY_DB_NAME\`.* TO '$CANARY_DB_USER'@'%';
FLUSH PRIVILEGES;
SQL

"${compose[@]}" exec -T redis redis-cli -a "$REDIS_PASSWORD" \
    ACL SETUSER "$CANARY_RUNTIME_REDIS_USERNAME" on \
    ">${CANARY_RUNTIME_REDIS_PASSWORD}" resetkeys '~cluster:channel:*:runtime' \
    -@all +hmget +pttl +ping +select >/dev/null

"${compose[@]}" up -d canary

schema_ready=0
for _ in $(seq 1 90); do
    if "${compose[@]}" exec -T -e MYSQL_PWD="$MARIADB_ROOT_PASSWORD" mariadb \
        mariadb -uroot -N -e \
        "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema='$CANARY_DB_NAME' AND table_name IN ('accounts','players','guilds','guild_membership','guild_ranks','channels','cluster_sessions');" \
        | grep -qx '7'; then
        schema_ready=1
        break
    fi
    sleep 2
done
if [[ "$schema_ready" -ne 1 ]]; then
    echo "Canary schema did not become ready; refusing to start Platform/Gateway." >&2
    exit 1
fi

export OTERYN_CANARY_DB_USER="$CANARY_READONLY_DB_USER"
export OTERYN_CANARY_DB_HOST="%"
export OTERYN_CANARY_DB_PASSWORD="$CANARY_READONLY_DB_PASSWORD"
export OTERYN_CANARY_PROVISIONING_DB_USER="$CANARY_PROVISIONING_DB_USER"
export OTERYN_CANARY_PROVISIONING_DB_HOST="%"
export OTERYN_CANARY_PROVISIONING_DB_PASSWORD="$CANARY_PROVISIONING_DB_PASSWORD"
export OTERYN_CANARY_CHARACTER_CREATE_DB_USER="$CANARY_CHARACTER_CREATE_DB_USER"
export OTERYN_CANARY_CHARACTER_CREATE_DB_HOST="%"
export OTERYN_CANARY_CHARACTER_CREATE_DB_PASSWORD="$CANARY_CHARACTER_CREATE_DB_PASSWORD"

apply_sql_template "$REPO_ROOT/database/provisioning/canary-readonly.sql.template" '
    !/^SHOW GRANTS/ {
        gsub(/\{\{OTERYN_CANARY_DB_USER\}\}/, ENVIRON["OTERYN_CANARY_DB_USER"])
        gsub(/\{\{OTERYN_CANARY_DB_HOST\}\}/, ENVIRON["OTERYN_CANARY_DB_HOST"])
        gsub(/\{\{OTERYN_CANARY_DB_PASSWORD\}\}/, ENVIRON["OTERYN_CANARY_DB_PASSWORD"])
        gsub(/\{\{CANARY_DB_NAME\}\}/, ENVIRON["CANARY_DB_NAME"])
        print
    }'

apply_sql_template "$REPO_ROOT/database/provisioning/canary-provisioning.sql.template" '
    !/^SHOW GRANTS/ {
        gsub(/\{\{OTERYN_CANARY_PROVISIONING_DB_USER\}\}/, ENVIRON["OTERYN_CANARY_PROVISIONING_DB_USER"])
        gsub(/\{\{OTERYN_CANARY_PROVISIONING_DB_HOST\}\}/, ENVIRON["OTERYN_CANARY_PROVISIONING_DB_HOST"])
        gsub(/\{\{OTERYN_CANARY_PROVISIONING_DB_PASSWORD\}\}/, ENVIRON["OTERYN_CANARY_PROVISIONING_DB_PASSWORD"])
        gsub(/\{\{CANARY_DB_NAME\}\}/, ENVIRON["CANARY_DB_NAME"])
        print
    }'

apply_sql_template "$REPO_ROOT/database/provisioning/canary-character-create.sql.template" '
    !/^SHOW GRANTS/ {
        gsub(/\{\{OTERYN_CANARY_CHARACTER_CREATE_DB_USER\}\}/, ENVIRON["OTERYN_CANARY_CHARACTER_CREATE_DB_USER"])
        gsub(/\{\{OTERYN_CANARY_CHARACTER_CREATE_DB_HOST\}\}/, ENVIRON["OTERYN_CANARY_CHARACTER_CREATE_DB_HOST"])
        gsub(/\{\{OTERYN_CANARY_CHARACTER_CREATE_DB_PASSWORD\}\}/, ENVIRON["OTERYN_CANARY_CHARACTER_CREATE_DB_PASSWORD"])
        gsub(/\{\{CANARY_DB_NAME\}\}/, ENVIRON["CANARY_DB_NAME"])
        print
    }'

# The first managed migration of a truly empty Platform DB receives its own
# verified pre-migration dump. This runs immediately before lib.sh's migration
# preparation hook and before the candidate Platform container can start.
OTERYN_ENV_FILE="$ENV_FILE" bash "$SCRIPT_DIR/prepare-fresh-schema-baseline.sh"

"${compose[@]}" up -d platform
"${compose[@]}" exec -T platform php artisan migrate --force --no-interaction
if ! "${compose[@]}" exec -T platform sh -ec 'test -s storage/oauth-private.key && test -s storage/oauth-public.key'; then
    "${compose[@]}" exec -T platform php artisan passport:keys --no-interaction
fi
"${compose[@]}" exec -T platform php artisan game-auth:oauth-client:ensure
"${compose[@]}" exec -T platform php artisan canary:verify-db-privileges
"${compose[@]}" exec -T platform php artisan canary:verify-provisioning-db-privileges
"${compose[@]}" exec -T platform php artisan canary:verify-character-create-db-privileges

"${compose[@]}" up -d --no-deps --force-recreate internal-proxy
"${compose[@]}" up -d gateway

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

rm -f "$state_dir/backups/fresh-empty-before-"*/evidence.env "$state_dir/backups/fresh-empty-before-"*/platform.sql 2>/dev/null || true
rmdir "$state_dir/backups/fresh-empty-before-"* 2>/dev/null || true

echo "Oteryn Synology staging deployment is healthy."
