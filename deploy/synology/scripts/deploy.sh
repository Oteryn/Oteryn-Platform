#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd -- "$DEPLOY_DIR/../.." && pwd)"
ENV_FILE="${OTERYN_ENV_FILE:-$DEPLOY_DIR/.env}"
COMPOSE_FILE="$DEPLOY_DIR/compose.yml"
CORE_DEPLOY="$SCRIPT_DIR/deploy-core.sh"

# shellcheck source=deploy/synology/scripts/lib.sh
source "$SCRIPT_DIR/lib.sh"
load_oteryn_env_file "$ENV_FILE"

required_vars=(
    OTERYN_RELEASE_SHA PLATFORM_SOURCE_SHA GATEWAY_SOURCE_SHA
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

command -v docker >/dev/null 2>&1 || { echo "docker is required on the deployment runner." >&2; exit 1; }
docker compose version >/dev/null 2>&1 || { echo "Docker Compose v2 plugin is required on the deployment runner." >&2; exit 1; }
[[ -f "$CORE_DEPLOY" ]] || { echo "Canonical full deployment core is missing: $CORE_DEPLOY" >&2; exit 1; }

validate_ipv4_policy() {
    local address="$1" policy="$2"
    bash "$SCRIPT_DIR/validate-ipv4.sh" "$address" "$policy"
}
validate_port() {
    local name="$1" value="${!1}"
    if [[ ! "$value" =~ ^[1-9][0-9]*$ ]] || (( value > 65535 )); then
        echo "$name must be an integer in 1..65535." >&2
        exit 1
    fi
}

validate_ipv4_policy "$PLATFORM_BIND_ADDRESS" loopback || { echo "PLATFORM_BIND_ADDRESS is invalid; Platform must remain loopback-only." >&2; exit 1; }
validate_ipv4_policy "$GATEWAY_BIND_ADDRESS" loopback || { echo "GATEWAY_BIND_ADDRESS is invalid; Gateway must remain loopback-only." >&2; exit 1; }
validate_ipv4_policy "$CANARY_LOGIN_BIND_ADDRESS" loopback || { echo "CANARY_LOGIN_BIND_ADDRESS is invalid; legacy login must remain loopback-only." >&2; exit 1; }
validate_ipv4_policy "$CANARY_GAME_BIND_ADDRESS" private-or-loopback || { echo "CANARY_GAME_BIND_ADDRESS must be loopback or an exact private LAN IPv4 address." >&2; exit 1; }
[[ "$CANARY_SERVER_IP" == "$CANARY_GAME_BIND_ADDRESS" ]] || { echo "CANARY_SERVER_IP must exactly match CANARY_GAME_BIND_ADDRESS." >&2; exit 1; }
[[ "$GAME_WORLD_HOST" == "$CANARY_GAME_BIND_ADDRESS" ]] || { echo "GAME_WORLD_HOST must exactly match CANARY_GAME_BIND_ADDRESS." >&2; exit 1; }
for port_name in PLATFORM_PORT GATEWAY_PORT CANARY_LOGIN_PORT CANARY_GAME_PORT GAME_WORLD_PORT; do
    validate_port "$port_name"
done
[[ "$GAME_WORLD_PORT" == "$CANARY_GAME_PORT" ]] || { echo "GAME_WORLD_PORT must exactly match CANARY_GAME_PORT." >&2; exit 1; }
[[ "$GAME_WORLD_ID" =~ ^[1-9][0-9]*$ ]] || { echo "GAME_WORLD_ID must be a positive integer." >&2; exit 1; }
[[ "$GAME_WORLD_SLUG" =~ ^[a-z0-9][a-z0-9-]{0,63}$ ]] || { echo "GAME_WORLD_SLUG must be a bounded lower-case slug." >&2; exit 1; }
[[ "$GAME_WORLD_REGION" =~ ^[A-Za-z0-9_-]{1,32}$ ]] || { echo "GAME_WORLD_REGION must be a simple bounded label." >&2; exit 1; }
(( ${#GAME_WORLD_NAME} >= 1 && ${#GAME_WORLD_NAME} <= 100 )) || { echo "GAME_WORLD_NAME must contain 1..100 characters." >&2; exit 1; }

compose=(docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE")
state_dir="${OTERYN_STATE_DIR:-/var/lib/oteryn-staging-state}"
mkdir -p "$state_dir"
chmod 700 "$state_dir"

# Presentation-only changes stay on deploy-core.sh's lighter platform-fast path.
_oteryn_presentation_path() {
    local path="$1"
    case "$path" in
        resources/*|lang/*|public/css/*|public/js/*|public/images/*) return 0 ;;
        *) return 1 ;;
    esac
}

# These are the exact/prefix inputs that build only the Platform component in
# scripts/ci/classify_synology_builds.py. Database/schema and build/deployment
# control-plane inputs are deliberately excluded and therefore fall back full.
_oteryn_platform_runtime_path() {
    local path="$1"
    case "$path" in
        composer.json|composer.lock|artisan|\
        deploy/synology/docker/platform.Dockerfile|\
        deploy/synology/docker/platform-media.ini|\
        deploy/synology/docker/platform-entrypoint.sh|\
        docs/testing/WIKI_EXPECTED_CONTENT_INVENTORY.json|\
        docs/architecture/adr/0004-authoritative-platform-account-ownership.md|\
        docs/architecture/adr/0005-character-creation-product-policy.md|\
        docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md|\
        docs/contracts/OTCLIENT_GAME_AUTH_CONTRACT.md|\
        docs/agents/PROJECT_STATE.md|\
        docs/architecture/PUBLIC_WEBSITE_EXPANSION_PLAN.md|\
        docs/architecture/SECURITY_ARCHITECTURE.md|\
        docs/architecture/adr/0013-wiki-administration.md)
            return 0
            ;;
        app/*|bootstrap/*|config/*|public/*|resources/*|routes/*|lang/*|storage/*)
            return 0
            ;;
        *)
            return 1
            ;;
    esac
}

# Ignore only repository inputs known not to affect a deployed runtime image or
# Synology reconciliation contract. Control-plane and unknown paths remain full.
_oteryn_non_runtime_path() {
    local path="$1"
    case "$path" in
        .github/workflows/build-synology-staging-images.yml|\
        .github/workflows/deploy-synology-staging.yml|\
        scripts/ci/classify_synology_builds.py|\
        deploy/synology/scripts/repository-ghcr-image.sh)
            return 1
            ;;
        deploy/synology/README.md|\
        deploy/synology/PUBLIC_ENDPOINTS.md|\
        deploy/synology/BUILD_ROUTING.md|\
        deploy/synology/.gitignore|\
        deploy/synology/tests/*|\
        docs/*|tests/*|scripts/acceptance/*|.github/*|\
        AGENTS.md|README.md|LICENSE|.gitignore|.gitattributes|.editorconfig|\
        phpunit.xml|phpunit.xml.dist|phpstan.neon|phpstan.neon.dist|pint.json|\
        package.json|package-lock.json)
            return 0
            ;;
        *)
            return 1
            ;;
    esac
}

_oteryn_running_service_id() {
    local service="$1" expected_image="${2:-}" container_id running image_id expected_image_id
    container_id="$("${compose[@]}" ps -q "$service" 2>/dev/null || true)"
    [[ -n "$container_id" ]] || { echo "Platform reconcile rejected: $service is not created." >&2; return 1; }
    running="$(docker inspect --format '{{.State.Running}}' "$container_id" 2>/dev/null || true)"
    [[ "$running" == true ]] || { echo "Platform reconcile rejected: $service is not running." >&2; return 1; }
    if [[ -n "$expected_image" ]]; then
        image_id="$(docker inspect --format '{{.Image}}' "$container_id" 2>/dev/null || true)"
        expected_image_id="$(docker image inspect --format '{{.Id}}' "$expected_image" 2>/dev/null || true)"
        [[ -n "$expected_image_id" && "$image_id" == "$expected_image_id" ]] || {
            echo "Platform reconcile rejected: running $service image does not match persisted release identity." >&2
            return 1
        }
    fi
    printf '%s\n' "$container_id"
}

RECONCILE_REASON='not evaluated'
RECONCILE_RUNTIME_PATHS=0
RECONCILE_NON_PRESENTATION_PATHS=0
RECONCILE_OLD_RELEASE_SHA=''
RECONCILE_OLD_PLATFORM_SOURCE_SHA=''
RECONCILE_OLD_GATEWAY_SOURCE_SHA=''
RECONCILE_OLD_PLATFORM_IMAGE=''
RECONCILE_OLD_GATEWAY_IMAGE=''
RECONCILE_OLD_CANARY_IMAGE=''
RECONCILE_SCHEMA_ID=''
RECONCILE_MARIADB_ID=''
RECONCILE_REDIS_ID=''
RECONCILE_CANARY_ID=''
RECONCILE_PROXY_ID=''
RECONCILE_GATEWAY_ID=''
RECONCILE_OLD_PLATFORM_ID=''
RECONCILE_SWAP_STARTED=0
RECONCILE_STATE_PROMOTED=0
RECONCILE_HAD_PREVIOUS_LAST_GOOD=0
RECONCILE_PREVIOUS_LAST_GOOD="$state_dir/.platform-reconcile-previous-last-good.env"

_oteryn_platform_reconcile_candidate() {
    local current_file="$state_dir/current-release.env"
    local schema_file="$state_dir/schema-state.env"
    local current_schema_state current_schema_id checked_out path
    local -a current_state changed_paths

    RECONCILE_REASON='candidate is not Platform-only'
    RECONCILE_RUNTIME_PATHS=0
    RECONCILE_NON_PRESENTATION_PATHS=0

    [[ -f "$current_file" ]] || { RECONCILE_REASON='managed current release state is missing'; return 1; }
    [[ ! -f "$state_dir/candidate-release.env" ]] || { RECONCILE_REASON='an unresolved migration candidate exists'; return 1; }
    [[ ! -e "$RECONCILE_PREVIOUS_LAST_GOOD" ]] || {
        echo "Platform reconcile recovery marker already exists: $RECONCILE_PREVIOUS_LAST_GOOD" >&2
        return 2
    }
    bash "$SCRIPT_DIR/release-state.sh" validate "$current_file" || return 2

    mapfile -t current_state < <(bash -c '
        set -euo pipefail
        source "$1"
        printf "%s\n" \
            "$RELEASE_SHA" "$PLATFORM_SOURCE_SHA" "$GATEWAY_SOURCE_SHA" \
            "$PLATFORM_IMAGE" "$GATEWAY_IMAGE" "$CANARY_IMAGE" \
            "$SCHEMA_COMPATIBILITY_ID" "$APP_ACCEPTS_SCHEMA_IDS" \
            "${GAME_WORLD_ID:-}" "${GAME_WORLD_SLUG:-}" "${GAME_WORLD_NAME:-}" \
            "${GAME_WORLD_REGION:-}" "${GAME_WORLD_HOST:-}" "${GAME_WORLD_PORT:-}"
    ' bash "$current_file")

    RECONCILE_OLD_RELEASE_SHA="${current_state[0]:-}"
    RECONCILE_OLD_PLATFORM_SOURCE_SHA="${current_state[1]:-}"
    RECONCILE_OLD_GATEWAY_SOURCE_SHA="${current_state[2]:-}"
    RECONCILE_OLD_PLATFORM_IMAGE="${current_state[3]:-}"
    RECONCILE_OLD_GATEWAY_IMAGE="${current_state[4]:-}"
    RECONCILE_OLD_CANARY_IMAGE="${current_state[5]:-}"

    [[ "$RECONCILE_OLD_RELEASE_SHA" != "$OTERYN_RELEASE_SHA" ]] || { RECONCILE_REASON='target release is already current'; return 1; }
    checked_out="$(git -C "$REPO_ROOT" rev-parse HEAD)"
    [[ "$checked_out" == "$OTERYN_RELEASE_SHA" ]] || { echo "Platform reconcile rejected: checkout is not the exact target release." >&2; return 2; }
    git -C "$REPO_ROOT" merge-base --is-ancestor "$RECONCILE_OLD_RELEASE_SHA" "$OTERYN_RELEASE_SHA" || {
        echo "Platform reconcile rejected: persisted current release is not in target protected-main lineage." >&2
        return 2
    }

    mapfile -d '' -t changed_paths < <(
        git -C "$REPO_ROOT" diff --name-only --no-renames --diff-filter=ACDMRTUXB -z \
            "$RECONCILE_OLD_RELEASE_SHA" "$OTERYN_RELEASE_SHA" --
    )
    ((${#changed_paths[@]} > 0)) || { RECONCILE_REASON='no accumulated file changes'; return 1; }

    for path in "${changed_paths[@]}"; do
        if _oteryn_platform_runtime_path "$path"; then
            RECONCILE_RUNTIME_PATHS=$((RECONCILE_RUNTIME_PATHS + 1))
            if ! _oteryn_presentation_path "$path"; then
                RECONCILE_NON_PRESENTATION_PATHS=$((RECONCILE_NON_PRESENTATION_PATHS + 1))
            fi
            continue
        fi
        if _oteryn_non_runtime_path "$path"; then
            continue
        fi
        RECONCILE_REASON="accumulated runtime/control impact includes $path"
        return 1
    done
    ((RECONCILE_RUNTIME_PATHS > 0)) || { RECONCILE_REASON='no Platform runtime changes'; return 1; }
    ((RECONCILE_NON_PRESENTATION_PATHS > 0)) || { RECONCILE_REASON='presentation-only candidate delegated to platform-fast'; return 1; }

    [[ "$PLATFORM_SOURCE_SHA" == "$OTERYN_RELEASE_SHA" ]] || { RECONCILE_REASON='Platform was not built from the exact target release'; return 1; }
    [[ "$PLATFORM_IMAGE" != "$RECONCILE_OLD_PLATFORM_IMAGE" ]] || { RECONCILE_REASON='Platform image did not change'; return 1; }
    [[ "$GATEWAY_SOURCE_SHA" == "$RECONCILE_OLD_GATEWAY_SOURCE_SHA" && "$GATEWAY_IMAGE" == "$RECONCILE_OLD_GATEWAY_IMAGE" ]] || {
        RECONCILE_REASON='Gateway provenance changed'; return 1;
    }
    [[ "$CANARY_IMAGE" == "$RECONCILE_OLD_CANARY_IMAGE" ]] || { RECONCILE_REASON='Canary provenance changed'; return 1; }
    [[ "${current_state[8]:-}" == "$GAME_WORLD_ID" \
        && "${current_state[9]:-}" == "$GAME_WORLD_SLUG" \
        && "${current_state[10]:-}" == "$GAME_WORLD_NAME" \
        && "${current_state[11]:-}" == "$GAME_WORLD_REGION" \
        && "${current_state[12]:-}" == "$GAME_WORLD_HOST" \
        && "${current_state[13]:-}" == "$GAME_WORLD_PORT" ]] || {
        RECONCILE_REASON='staging world identity changed'; return 1;
    }

    [[ -f "$schema_file" ]] || { RECONCILE_REASON='managed schema state is missing'; return 1; }
    current_schema_state="$(_oteryn_read_state_key "$schema_file" SCHEMA_STATE)" || return 2
    current_schema_id="$(_oteryn_read_state_key "$schema_file" SCHEMA_COMPATIBILITY_ID)" || return 2
    [[ "$current_schema_state" == known ]] || { RECONCILE_REASON='managed schema state is not known'; return 1; }
    [[ "$current_schema_id" == "${current_state[6]:-}" ]] || { RECONCILE_REASON='current release/schema identity is not exact'; return 1; }

    _oteryn_release_sha >/dev/null || return 2
    _oteryn_load_candidate_contract || return 2
    [[ "$OTERYN_SCHEMA_COMPATIBILITY_ID" == "$current_schema_id" ]] || { RECONCILE_REASON='candidate primary schema identity changed'; return 1; }
    _oteryn_schema_list_contains "$current_schema_id" "$OTERYN_APP_ACCEPTS_SCHEMA_IDS" || {
        RECONCILE_REASON='candidate does not accept the current schema'; return 1;
    }
    RECONCILE_SCHEMA_ID="$current_schema_id"

    RECONCILE_OLD_PLATFORM_ID="$(_oteryn_running_service_id platform "$RECONCILE_OLD_PLATFORM_IMAGE")" || return 2
    RECONCILE_GATEWAY_ID="$(_oteryn_running_service_id gateway "$RECONCILE_OLD_GATEWAY_IMAGE")" || return 2
    RECONCILE_CANARY_ID="$(_oteryn_running_service_id canary "$RECONCILE_OLD_CANARY_IMAGE")" || return 2
    RECONCILE_MARIADB_ID="$(_oteryn_running_service_id mariadb)" || return 2
    RECONCILE_REDIS_ID="$(_oteryn_running_service_id redis)" || return 2
    RECONCILE_PROXY_ID="$(_oteryn_running_service_id internal-proxy)" || return 2

    RECONCILE_REASON='schema-stable Platform-only accumulated runtime impact'
    return 0
}

_oteryn_reconcile_restore_on_exit() {
    local rc="$?" restored_platform_id expected_old_id
    trap - EXIT
    if [[ "$rc" -eq 0 || "$RECONCILE_SWAP_STARTED" -ne 1 ]]; then
        exit "$rc"
    fi

    echo "Platform reconcile failed; restoring the previously proven Platform release." >&2
    if [[ "$RECONCILE_STATE_PROMOTED" -eq 1 && -f "$state_dir/last-good-release.env" ]]; then
        cp "$state_dir/last-good-release.env" "$state_dir/current-release.env.tmp" || true
        chmod 600 "$state_dir/current-release.env.tmp" 2>/dev/null || true
        mv "$state_dir/current-release.env.tmp" "$state_dir/current-release.env" 2>/dev/null || true
    fi

    export OTERYN_RELEASE_SHA="$RECONCILE_OLD_RELEASE_SHA"
    export PLATFORM_SOURCE_SHA="$RECONCILE_OLD_PLATFORM_SOURCE_SHA"
    export GATEWAY_SOURCE_SHA="$RECONCILE_OLD_GATEWAY_SOURCE_SHA"
    export PLATFORM_IMAGE="$RECONCILE_OLD_PLATFORM_IMAGE"
    export GATEWAY_IMAGE="$RECONCILE_OLD_GATEWAY_IMAGE"
    export CANARY_IMAGE="$RECONCILE_OLD_CANARY_IMAGE"
    export GATEWAY_VERSION="sha-$RECONCILE_OLD_GATEWAY_SOURCE_SHA"

    command docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE" \
        up -d --no-deps --force-recreate platform >/dev/null 2>&1 || \
        echo "Platform reconcile recovery could not recreate the previous Platform container." >&2
    _oteryn_reconcile_marketplace_scheduler_after_runtime_change >/dev/null 2>&1 || \
        echo "Platform reconcile recovery could not reconcile the previous Marketplace runtime." >&2

    restored_platform_id="$(command docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE" ps -q platform 2>/dev/null || true)"
    expected_old_id="$(command docker image inspect --format '{{.Id}}' "$RECONCILE_OLD_PLATFORM_IMAGE" 2>/dev/null || true)"
    if [[ -z "$restored_platform_id" \
        || "$(command docker inspect --format '{{.State.Running}}' "$restored_platform_id" 2>/dev/null || true)" != true \
        || "$(command docker inspect --format '{{.Image}}' "$restored_platform_id" 2>/dev/null || true)" != "$expected_old_id" ]]; then
        echo "Platform reconcile recovery could not prove the previous Platform runtime was restored." >&2
    else
        echo "Previous Platform runtime restored after Platform reconcile failure." >&2
    fi

    if [[ "$RECONCILE_HAD_PREVIOUS_LAST_GOOD" -eq 1 && -f "$RECONCILE_PREVIOUS_LAST_GOOD" ]]; then
        mv "$RECONCILE_PREVIOUS_LAST_GOOD" "$state_dir/last-good-release.env" 2>/dev/null || true
    else
        rm -f "$state_dir/last-good-release.env" "$RECONCILE_PREVIOUS_LAST_GOOD" 2>/dev/null || true
    fi
    rm -f "$state_dir/current-release.env.reconcile" 2>/dev/null || true
    exit "$rc"
}

_oteryn_assert_preserved_service() {
    local service="$1" expected_id="$2" actual_id running
    actual_id="$("${compose[@]}" ps -q "$service" 2>/dev/null || true)"
    [[ "$actual_id" == "$expected_id" ]] || { echo "Platform reconcile changed $service container identity." >&2; return 1; }
    running="$(docker inspect --format '{{.State.Running}}' "$actual_id" 2>/dev/null || true)"
    [[ "$running" == true ]] || { echo "Platform reconcile left $service non-running." >&2; return 1; }
}

_oteryn_platform_smoke() {
    local platform_id expected_id actual_id actual_binding status ready=0
    platform_id="$("${compose[@]}" ps -q platform)"
    [[ -n "$platform_id" ]] || { echo "Platform reconcile did not create Platform." >&2; return 1; }
    expected_id="$(docker image inspect --format '{{.Id}}' "$PLATFORM_IMAGE")"
    actual_id="$(docker inspect --format '{{.Image}}' "$platform_id")"
    [[ "$actual_id" == "$expected_id" ]] || { echo "Platform reconcile image identity mismatch." >&2; return 1; }
    actual_binding="$(docker inspect --format '{{with index .NetworkSettings.Ports "8000/tcp"}}{{if eq (len .) 1}}{{(index . 0).HostIp}}:{{(index . 0).HostPort}}{{end}}{{end}}' "$platform_id")"
    [[ "$actual_binding" == "${PLATFORM_BIND_ADDRESS}:${PLATFORM_PORT}" ]] || {
        echo "Platform reconcile changed Platform published binding: ${actual_binding:-none}" >&2
        return 1
    }
    for _ in $(seq 1 20); do
        if curl --fail --silent --show-error --max-time 5 \
            -H 'Host: oteryn.molehill.cloud' \
            -H 'X-Forwarded-Host: oteryn.molehill.cloud' \
            -H 'X-Forwarded-Proto: https' \
            -H 'X-Forwarded-Port: 443' \
            "http://127.0.0.1:${PLATFORM_PORT}/health" >/dev/null; then
            ready=1
            break
        fi
        sleep 1
    done
    [[ "$ready" -eq 1 ]] || { echo "Platform reconcile /health did not become ready." >&2; return 1; }
    status="$(curl --silent --show-error --max-time 5 -o /dev/null -w '%{http_code}' \
        -H 'Host: oteryn.molehill.cloud' \
        -H 'X-Forwarded-Host: oteryn.molehill.cloud' \
        -H 'X-Forwarded-Proto: https' \
        -H 'X-Forwarded-Port: 443' \
        "http://127.0.0.1:${PLATFORM_PORT}/login?locale=en")"
    [[ "$status" == 200 ]] || { echo "Platform reconcile public login smoke returned HTTP $status." >&2; return 1; }
}

_oteryn_platform_reconcile() {
    local current_file="$state_dir/current-release.env"
    local last_good_file="$state_dir/last-good-release.env"
    local started="$SECONDS"

    if [[ -f "$last_good_file" ]]; then
        cp "$last_good_file" "$RECONCILE_PREVIOUS_LAST_GOOD"
        chmod 600 "$RECONCILE_PREVIOUS_LAST_GOOD"
        RECONCILE_HAD_PREVIOUS_LAST_GOOD=1
    fi
    cp "$current_file" "$last_good_file.tmp"
    chmod 600 "$last_good_file.tmp"
    mv "$last_good_file.tmp" "$last_good_file"

    trap _oteryn_reconcile_restore_on_exit EXIT
    RECONCILE_SWAP_STARTED=1

    "${compose[@]}" config --quiet
    "${compose[@]}" up -d --no-deps --force-recreate platform
    _oteryn_reconcile_marketplace_scheduler_after_runtime_change
    _oteryn_platform_smoke

    # Broader application/runtime changes get the complete existing cross-service
    # staging health contract while all unrelated service containers remain warm.
    OTERYN_ENV_FILE="$ENV_FILE" bash "$SCRIPT_DIR/health-check.sh"

    _oteryn_assert_preserved_service mariadb "$RECONCILE_MARIADB_ID"
    _oteryn_assert_preserved_service redis "$RECONCILE_REDIS_ID"
    _oteryn_assert_preserved_service canary "$RECONCILE_CANARY_ID"
    _oteryn_assert_preserved_service internal-proxy "$RECONCILE_PROXY_ID"
    _oteryn_assert_preserved_service gateway "$RECONCILE_GATEWAY_ID"

    bash "$SCRIPT_DIR/release-state.sh" write "$state_dir/current-release.env.reconcile" \
        "$OTERYN_RELEASE_SHA" "$PLATFORM_SOURCE_SHA" "$GATEWAY_SOURCE_SHA" \
        "$RECONCILE_SCHEMA_ID" "$OTERYN_APP_ACCEPTS_SCHEMA_IDS" \
        "$PLATFORM_IMAGE" "$GATEWAY_IMAGE" "$CANARY_IMAGE" 1
    mv "$state_dir/current-release.env.reconcile" "$current_file"
    RECONCILE_STATE_PROMOTED=1

    rm -f "$RECONCILE_PREVIOUS_LAST_GOOD"
    RECONCILE_SWAP_STARTED=0
    trap - EXIT
    echo "Oteryn Synology staging Platform reconcile is healthy in $((SECONDS - started))s; MariaDB, Redis, Canary, internal proxy and Gateway were preserved."
}

reconcile_rc=0
_oteryn_platform_reconcile_candidate || reconcile_rc=$?
if [[ "$reconcile_rc" -eq 0 ]]; then
    echo "Synology staging deploy profile: platform-reconcile ($RECONCILE_REASON; $RECONCILE_RUNTIME_PATHS Platform runtime path(s), $RECONCILE_NON_PRESENTATION_PATHS non-presentation)."
    _oteryn_platform_reconcile
    exit 0
elif [[ "$reconcile_rc" -gt 1 ]]; then
    echo "Platform reconcile preflight failed closed; refusing to continue with ambiguous state." >&2
    exit "$reconcile_rc"
fi

# Existing presentation-fast and complete migration/recovery behavior remain
# canonical in the preserved core. Delegation occurs before any mutation here.
exec bash "$CORE_DEPLOY"
