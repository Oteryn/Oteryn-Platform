#!/usr/bin/env bash

# Docker Hub index digests verified 2026-08-12. Health probes keep their exact
# historical behavior while the Docker boundary substitutes immutable identities.
OTERYN_HEALTH_ALPINE_IMAGE='alpine@sha256:14358309a308569c32bdc37e2e0e9694be33a9d99e68afb0f5ff33cc1f695dce'
OTERYN_HEALTH_PYTHON_IMAGE='python@sha256:6d43704baacd1bfbe7c295d7f13079d5d8104ed33568873133f8fc69980419df'

# Resolves a non-secret native-login staging setting without a hand-set GitHub repository variable.
# Precedence: the repository variable (passed as the second argument) wins; otherwise the committed
# deploy/synology/native-defaults.env value; otherwise empty (the native admission gate then fails
# closed with its own message). The file is read as data, never sourced.
oteryn_native_default() {
    local name="$1" current="${2:-}" file value
    if [[ -n "$current" ]]; then
        printf '%s' "$current"
        return 0
    fi
    if [[ ! "$name" =~ ^OTERYN_STAGING_NATIVE_[A-Z_]+$ ]]; then
        echo "Invalid native default name: $name" >&2
        return 1
    fi
    file="${OTERYN_NATIVE_DEFAULTS_FILE:-$(dirname "${BASH_SOURCE[0]}")/../native-defaults.env}"
    [[ -f "$file" ]] || return 0
    value="$(sed -n "s/^${name}=//p" "$file" | tail -n 1)"
    printf '%s' "${value%$'\r'}"
}

load_oteryn_env_file() {
    local env_file="$1"
    local line key value

    if [[ ! -f "$env_file" ]]; then
        echo "Missing staging environment file: $env_file" >&2
        return 1
    fi

    while IFS= read -r line || [[ -n "$line" ]]; do
        line="${line%$'\r'}"
        if [[ -z "$line" || "$line" =~ ^[[:space:]]*# ]]; then
            continue
        fi
        if [[ "$line" != *=* ]]; then
            echo "Invalid staging environment line; expected KEY=VALUE." >&2
            return 1
        fi
        key="${line%%=*}"
        value="${line#*=}"
        if [[ ! "$key" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]]; then
            echo "Invalid staging environment key: $key" >&2
            return 1
        fi
        printf -v "$key" '%s' "$value"
        export "$key"
    done < "$env_file"

    if [[ "${GITHUB_WORKFLOW:-}" == "Character Bazaar Staging Control" && "$(basename -- "$env_file")" == ".env" ]]; then
        case "${APP_URL:-}" in
            https://oteryn.molehill.cloud|http://127.0.0.1:8000) ;;
            *) echo "Character Bazaar public staging APP_URL must be the canonical origin or the exact legacy loopback value." >&2; return 1 ;;
        esac
        case "${SESSION_SECURE_COOKIE:-}" in
            true|false|'') ;;
            *) echo "Character Bazaar public staging SESSION_SECURE_COOKIE must be boolean." >&2; return 1 ;;
        esac
        APP_URL=https://oteryn.molehill.cloud
        SESSION_SECURE_COOKIE=true
        export APP_URL SESSION_SECURE_COOKIE
    fi

    oteryn_resolve_platform_mtls_settings
}

# Platform internal mTLS listener for the Game node (compose.yml internal-mtls, README: Platform
# internal mTLS listener). Each setting comes from the environment file (or the caller's environment)
# when set, otherwise from the committed non-secret OTERYN_STAGING_NATIVE_PLATFORM_MTLS_* value in
# deploy/synology/native-defaults.env, otherwise a fail-closed built-in (listener off, loopback). The
# three file settings are host paths of PUBLIC certificates and of the server key file; the key
# bytes are never read by these scripts and nothing here is a secret.
oteryn_resolve_platform_mtls_settings() {
    local pair name default_name fallback value
    for pair in \
        OTERYN_PLATFORM_MTLS_ENABLED:ENABLED:false \
        PLATFORM_MTLS_BIND_ADDRESS:BIND_ADDRESS:127.0.0.1 \
        PLATFORM_MTLS_PORT:PORT:8543 \
        PLATFORM_MTLS_SERVER_CERT_FILE:SERVER_CERT_FILE:/volume1/oteryn/preprod-platform-tls/platform-internal-preprod.fullchain.pem \
        PLATFORM_MTLS_SERVER_KEY_FILE:SERVER_KEY_FILE:/volume1/oteryn/preprod-platform-tls/platform-internal-preprod.key \
        PLATFORM_MTLS_CLIENT_CA_FILE:CLIENT_CA_FILE:/volume1/oteryn/preprod-platform-tls/oteryn-preprod-ca.crt; do
        name="${pair%%:*}"
        default_name="OTERYN_STAGING_NATIVE_PLATFORM_MTLS_$(cut -d: -f2 <<<"$pair")"
        fallback="${pair#*:*:}"
        value="$(oteryn_native_default "$default_name" "${!name:-}")" || return 1
        printf -v "$name" '%s' "${value:-$fallback}"
        export "${name?}"
    done
}

# Refuses malformed listener settings (deploy.sh, before any stack change). Off needs nothing else.
oteryn_platform_mtls_settings_valid() {
    local name value
    case "${OTERYN_PLATFORM_MTLS_ENABLED:-}" in
        false) return 0 ;;
        true) ;;
        *) echo "OTERYN_PLATFORM_MTLS_ENABLED must be true or false." >&2; return 1 ;;
    esac
    if ! bash "$(dirname "${BASH_SOURCE[0]}")/validate-ipv4.sh" "${PLATFORM_MTLS_BIND_ADDRESS:-}" private-or-loopback; then
        echo "PLATFORM_MTLS_BIND_ADDRESS must be loopback or an exact private LAN IPv4 address." >&2
        return 1
    fi
    if [[ ! "${PLATFORM_MTLS_PORT:-}" =~ ^[1-9][0-9]{0,4}$ ]] || ((PLATFORM_MTLS_PORT > 65535)); then
        echo "PLATFORM_MTLS_PORT must be an integer in 1..65535." >&2
        return 1
    fi
    for name in PLATFORM_MTLS_SERVER_CERT_FILE PLATFORM_MTLS_SERVER_KEY_FILE PLATFORM_MTLS_CLIENT_CA_FILE; do
        value="${!name:-}"
        if [[ ! "$value" =~ ^/[A-Za-z0-9._/-]+$ || "$value" == *..* || "$value" == */ ]]; then
            echo "$name must be an absolute host file path." >&2
            return 1
        fi
    done
}

# True when a Platform image carries the FastCGI pool the listener needs (php-fpm and the pool file
# from platform.Dockerfile). Images built before the listener have neither: status 3. Any other
# non-zero status means the image could not be inspected.
oteryn_platform_image_has_fastcgi() {
    local image="$1"
    [[ -n "$image" ]] || return 2
    docker run --rm --network none --entrypoint sh "$image" \
        -c 'if command -v php-fpm >/dev/null && test -f /usr/local/etc/php-fpm.d/oteryn-internal-mtls.conf; then exit 0; fi; exit 3' \
        >/dev/null 2>&1
}

# The listener setting a given Platform image can serve: the resolved setting, except false for an
# image built before the listener (legacy path, deploy.sh finalize_previous_candidate_if_healthy).
# Fails when the image cannot be inspected.
oteryn_platform_mtls_flag_for_image() {
    local image="$1" rc=0
    [[ "${OTERYN_PLATFORM_MTLS_ENABLED:-false}" == true ]] || { echo false; return 0; }
    oteryn_platform_image_has_fastcgi "$image" || rc=$?
    case "$rc" in
        0) echo true ;;
        3) echo false ;;
        *) echo "Unable to inspect Platform image $image for the FastCGI pool." >&2; return 1 ;;
    esac
}

# Pins the listener setting into a derived environment file (the file's value wins when
# health-check.sh loads it).
oteryn_pin_platform_mtls_flag() {
    local env_file="$1" flag="$2"
    { grep -v '^OTERYN_PLATFORM_MTLS_ENABLED=' "$env_file" || true; printf 'OTERYN_PLATFORM_MTLS_ENABLED=%s\n' "$flag"; } >"$env_file.pin"
    chmod 600 "$env_file.pin"
    mv "$env_file.pin" "$env_file"
}

# Refuses, before any stack change, a Platform image that cannot serve the listener while it is on:
# the listener would accept the Game node's TLS session and answer 502 for every request. A no-op
# while the listener is off. Arguments: the Platform image reference (already pulled).
oteryn_platform_mtls_image_ready() {
    local image="$1"
    [[ "${OTERYN_PLATFORM_MTLS_ENABLED:-false}" == true ]] || return 0
    if ! oteryn_platform_image_has_fastcgi "$image"; then
        echo "Platform mTLS listener is on but Platform image $image has no FastCGI pool (built before the listener); set OTERYN_PLATFORM_MTLS_ENABLED=false for this image or use a newer image." >&2
        return 1
    fi
}

# Before any stack change, with the bootstrap files staged: prove the bind address exists on the host
# and that nginx accepts the staged configuration with the mounted server chain, key and client CA
# (a missing host file fails here, because Compose never creates the bind sources), and that
# PLATFORM_IMAGE carries the FastCGI pool. Arguments: the
# route's Compose command words. A no-op while the listener is off.
oteryn_platform_mtls_preflight() {
    (($# > 0)) || { echo "Platform mTLS preflight needs the Compose command." >&2; return 1; }
    [[ "${OTERYN_PLATFORM_MTLS_ENABLED:-false}" == true ]] || return 0
    oteryn_platform_mtls_image_ready "${PLATFORM_IMAGE:-}" || return 1
    if ! docker run --rm --network host alpine:3.22 ip -o -4 addr show \
        | awk -v address="$PLATFORM_MTLS_BIND_ADDRESS" '{ split($4, cidr, "/"); if (cidr[1] == address) found = 1 } END { exit found ? 0 : 1 }'; then
        echo "Platform mTLS bind address $PLATFORM_MTLS_BIND_ADDRESS is not an address of this host." >&2
        return 1
    fi
    if ! "$@" run --rm --no-deps -T --entrypoint nginx internal-mtls -t -q -c /bootstrap/internal-mtls-nginx.conf; then
        echo "Platform mTLS listener configuration or its host certificate files were refused (README: Platform internal mTLS listener)." >&2
        return 1
    fi
    echo "Platform mTLS listener preflight passed for ${PLATFORM_MTLS_BIND_ADDRESS}:${PLATFORM_MTLS_PORT}."
}

# One digest of everything the running listener depends on: bind address, port, the three host
# paths, the contents of the server chain, key and client CA, and the nginx config. It is stored as
# the oteryn.platform-mtls.fingerprint label of internal-mtls, so the fast deploy path can tell
# whether the listener needs a full reconcile. The files are hashed in a throwaway container with no
# network (they may be root-only); only the combined digest leaves it. --mount fails on a missing
# file instead of creating it.
oteryn_platform_mtls_fingerprint() {
    local conf="${OTERYN_PLATFORM_MTLS_NGINX_CONF:-$(dirname "${BASH_SOURCE[0]}")/../nginx/internal-mtls.conf}" files
    files="$(docker run --rm --network none \
        --mount "type=bind,source=${PLATFORM_MTLS_SERVER_CERT_FILE},target=/m/server-fullchain.pem,readonly" \
        --mount "type=bind,source=${PLATFORM_MTLS_SERVER_KEY_FILE},target=/m/server.key,readonly" \
        --mount "type=bind,source=${PLATFORM_MTLS_CLIENT_CA_FILE},target=/m/client-ca.crt,readonly" \
        alpine:3.22 sha256sum /m/server-fullchain.pem /m/server.key /m/client-ca.crt 2>/dev/null)" || return 1
    [[ -n "$files" && -r "$conf" ]] || return 1
    {
        printf '%s\n' "$PLATFORM_MTLS_BIND_ADDRESS" "$PLATFORM_MTLS_PORT" "$PLATFORM_MTLS_SERVER_CERT_FILE" \
            "$PLATFORM_MTLS_SERVER_KEY_FILE" "$PLATFORM_MTLS_CLIENT_CA_FILE" "$files"
        sha256sum <"$conf"
    } | sha256sum | awk '{print $1}'
}

# The fingerprint label of a running internal-mtls container (empty when absent).
oteryn_platform_mtls_fingerprint_of() {
    command docker inspect --format '{{index .Config.Labels "oteryn.platform-mtls.fingerprint"}}' "$1" 2>/dev/null
}

# Brings the listener to the resolved setting after Platform is up: recreated while on (it then
# re-reads the host certificate files and carries their fingerprint), removed while off. Arguments:
# the route's Compose command words.
# Stops the Game-node mTLS listener together with the rest of the ingress while Platform is replaced
# and migrated (deploy.sh full path, rollback.sh), so no Game-node request reaches a Platform that has
# not passed its gates. oteryn_reconcile_platform_mtls recreates it after the gates.
oteryn_platform_mtls_stop() {
    (($# > 0)) || { echo "Platform mTLS stop needs the Compose command." >&2; return 1; }
    local id
    id="$("$@" ps -q internal-mtls 2>/dev/null)" || return 1
    [[ -z "$id" ]] || "$@" stop internal-mtls >&2
}

oteryn_reconcile_platform_mtls() {
    (($# > 0)) || { echo "Platform mTLS reconcile needs the Compose command." >&2; return 1; }
    if [[ "${OTERYN_PLATFORM_MTLS_ENABLED:-false}" == true ]]; then
        local fingerprint
        fingerprint="$(oteryn_platform_mtls_fingerprint)" || {
            echo "Platform mTLS listener files could not be fingerprinted (README: Platform internal mTLS listener)." >&2
            return 1
        }
        OTERYN_PLATFORM_MTLS_FINGERPRINT="$fingerprint" "$@" up -d --no-deps --force-recreate internal-mtls
    else
        "$@" rm -s -f internal-mtls >/dev/null
    fi
}

# The Platform FastCGI switch a running Platform container carries (absent means false).
oteryn_platform_fastcgi_flag_of() {
    local container_id="$1" value
    value="$(command docker inspect --format '{{range .Config.Env}}{{println .}}{{end}}' "$container_id" \
        | sed -n 's/^OTERYN_PLATFORM_FASTCGI_ENABLED=//p')" || return 1
    printf '%s' "${value:-false}"
}

_oteryn_platform_container_http_status() {
    local container_id="$1"
    local path="$2"

    [[ -n "$container_id" ]] || return 2
    [[ "$path" == /* && "$path" != *$'\r'* && "$path" != *$'\n'* ]] || return 2

    # The Platform Actions runner is itself a Docker container. Host-published
    # 127.0.0.1 ports therefore belong to the Synology host namespace, not the
    # runner namespace. Probe from inside the exact Platform container instead.
    command docker exec "$container_id" php -r '
$path = $argv[1] ?? "/";
$socket = @fsockopen("127.0.0.1", 8000, $errno, $errstr, 2.0);
if ($socket === false) {
    exit(10);
}
stream_set_timeout($socket, 2);
$request = "GET {$path} HTTP/1.1\r\n"
    . "Host: oteryn.molehill.cloud\r\n"
    . "X-Forwarded-Host: oteryn.molehill.cloud\r\n"
    . "X-Forwarded-Proto: https\r\n"
    . "X-Forwarded-Port: 443\r\n"
    . "Connection: close\r\n\r\n";
if (@fwrite($socket, $request) === false) {
    fclose($socket);
    exit(11);
}
$statusLine = fgets($socket);
fclose($socket);
if (! is_string($statusLine)
    || preg_match("/^HTTP\\/[0-9.]+\\s+([0-9]{3})(?:\\s|$)/", trim($statusLine), $matches) !== 1) {
    exit(12);
}
fwrite(STDOUT, $matches[1]);
' "$path"
}

_oteryn_wait_platform_http_200() {
    local container_id="$1"
    local path="${2:-/health}"
    local attempts="${3:-30}"
    local status

    [[ "$attempts" =~ ^[1-9][0-9]*$ ]] || return 2
    for _ in $(seq 1 "$attempts"); do
        status="$(_oteryn_platform_container_http_status "$container_id" "$path" 2>/dev/null || true)"
        if [[ "$status" == 200 ]]; then
            return 0
        fi
        sleep 1
    done
    return 1
}

_oteryn_deploy_state_dir() { printf '%s\n' "${OTERYN_STATE_DIR:-/var/lib/oteryn-staging-state}"; }

# Legacy bootstrap only: old managed releases used one shared application SHA
# for Platform and Gateway. New releases never derive the overall release from
# component image metadata.
_oteryn_release_sha_for_images() {
    local platform_image="$1" gateway_image="$2"
    local platform_revision gateway_revision
    platform_revision="$(command docker image inspect --format '{{ index .Config.Labels "org.opencontainers.image.revision" }}' "$platform_image" 2>/dev/null || true)"
    gateway_revision="$(command docker image inspect --format '{{ index .Config.Labels "org.opencontainers.image.revision" }}' "$gateway_image" 2>/dev/null || true)"

    [[ "$platform_revision" =~ ^[0-9a-f]{40}$ ]] || {
        echo "Cannot prove Platform OCI application revision; refusing migration-bearing deployment." >&2
        return 1
    }
    [[ "$gateway_revision" =~ ^[0-9a-f]{40}$ ]] || {
        echo "Cannot prove Gateway OCI application revision; refusing migration-bearing deployment." >&2
        return 1
    }
    [[ "$platform_revision" == "$gateway_revision" ]] || {
        echo "Legacy Platform/Gateway OCI application revisions disagree; refusing bootstrap." >&2
        return 1
    }
    printf '%s\n' "$platform_revision"
}

_oteryn_verify_component_image_source() {
    local image="$1" expected_sha="$2" label="$3" revision
    [[ "$expected_sha" =~ ^[0-9a-f]{40}$ ]] || {
        echo "$label source SHA is missing or invalid." >&2
        return 1
    }
    [[ "$image" =~ @sha256:[0-9a-f]{64}$ ]] || {
        echo "$label image is not an immutable digest reference." >&2
        return 1
    }
    revision="$(command docker image inspect --format '{{ index .Config.Labels "org.opencontainers.image.revision" }}' "$image" 2>/dev/null || true)"
    [[ "$revision" == "$expected_sha" ]] || {
        echo "$label OCI application revision does not match its persisted component source SHA." >&2
        return 1
    }
}

_oteryn_release_sha() {
    local release_sha="${OTERYN_RELEASE_SHA:-}"
    [[ "$release_sha" =~ ^[0-9a-f]{40}$ ]] || {
        echo "OTERYN_RELEASE_SHA must be the exact protected-main release identity." >&2
        return 1
    }
    [[ "${PLATFORM_SOURCE_SHA:-}" =~ ^[0-9a-f]{40}$ ]] || {
        echo "PLATFORM_SOURCE_SHA must be an exact component source SHA." >&2
        return 1
    }
    [[ "${GATEWAY_SOURCE_SHA:-}" =~ ^[0-9a-f]{40}$ ]] || {
        echo "GATEWAY_SOURCE_SHA must be an exact component source SHA." >&2
        return 1
    }
    _oteryn_verify_component_image_source "$PLATFORM_IMAGE" "$PLATFORM_SOURCE_SHA" Platform || return 1
    _oteryn_verify_component_image_source "$GATEWAY_IMAGE" "$GATEWAY_SOURCE_SHA" Gateway || return 1
    if [[ "${GATEWAY_VERSION:-}" =~ ^sha-([0-9a-f]{40})$ && "${BASH_REMATCH[1]}" != "$GATEWAY_SOURCE_SHA" ]]; then
        echo "GATEWAY_VERSION disagrees with Gateway component source SHA." >&2
        return 1
    fi
    printf '%s\n' "$release_sha"
}

_oteryn_contract_from_platform_image() {
    local platform_image="$1" payload line key value
    local policy='' schema='' accepts=''
    payload="$(command docker run --rm --network none --read-only --entrypoint cat "$platform_image" /var/www/html/deploy/synology/release-contract.env 2>/dev/null)" || {
        echo "Cannot read release compatibility contract from Platform image; refusing migration-bearing deployment." >&2
        return 1
    }
    while IFS= read -r line || [[ -n "$line" ]]; do
        line="${line%$'\r'}"
        [[ -z "$line" || "$line" =~ ^[[:space:]]*# ]] && continue
        [[ "$line" == *=* ]] || { echo "Invalid release contract line in Platform image." >&2; return 1; }
        key="${line%%=*}"
        value="${line#*=}"
        case "$key" in
            OTERYN_MIGRATION_POLICY) policy="$value" ;;
            OTERYN_SCHEMA_COMPATIBILITY_ID) schema="$value" ;;
            OTERYN_APP_ACCEPTS_SCHEMA_IDS) accepts="$value" ;;
            *) echo "Unexpected release contract key in Platform image: $key" >&2; return 1 ;;
        esac
    done <<<"$payload"

    [[ "$policy" == expand-contract ]] || { echo "Platform image migration policy must be expand-contract." >&2; return 1; }
    [[ "$schema" =~ ^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$ ]] || { echo "Platform image schema compatibility identity is invalid." >&2; return 1; }
    [[ "$accepts" =~ ^[A-Za-z0-9._-]+(,[A-Za-z0-9._-]+)*$ ]] || { echo "Platform image accepted schema identity list is invalid." >&2; return 1; }
    printf '%s\t%s\t%s\n' "$policy" "$schema" "$accepts"
}

_oteryn_load_candidate_contract() {
    local contract
    contract="$(_oteryn_contract_from_platform_image "$PLATFORM_IMAGE")" || return 1
    IFS=$'\t' read -r OTERYN_MIGRATION_POLICY OTERYN_SCHEMA_COMPATIBILITY_ID OTERYN_APP_ACCEPTS_SCHEMA_IDS <<<"$contract" || return 1
    [[ -n "${OTERYN_MIGRATION_POLICY:-}" && -n "${OTERYN_SCHEMA_COMPATIBILITY_ID:-}" && -n "${OTERYN_APP_ACCEPTS_SCHEMA_IDS:-}" ]] || {
        echo "Unable to load candidate release contract from Platform image." >&2
        return 1
    }
    export OTERYN_MIGRATION_POLICY OTERYN_SCHEMA_COMPATIBILITY_ID OTERYN_APP_ACCEPTS_SCHEMA_IDS
}

_oteryn_read_state_key() {
    local file="$1" key="$2" value
    value="$(sed -n "s/^${key}=//p" "$file" | head -n 1)"
    [[ -n "$value" ]] || { echo "Missing $key in $file" >&2; return 1; }
    printf '%s\n' "$value"
}

_oteryn_schema_list_contains() {
    local needle="$1" list="$2" item
    IFS=',' read -r -a items <<<"$list"
    for item in "${items[@]}"; do
        [[ "$item" == "$needle" ]] && return 0
    done
    return 1
}

_oteryn_write_schema_state_known() {
    local state_dir="$1" schema_id="$2" release_sha="$3"
    [[ "$schema_id" =~ ^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$ ]] || { echo "Cannot persist invalid schema identity." >&2; return 1; }
    [[ "$release_sha" =~ ^[0-9a-f]{40}$ ]] || { echo "Cannot persist schema state without exact release identity." >&2; return 1; }
    {
        printf 'SCHEMA_STATE=known\n'
        printf 'SCHEMA_COMPATIBILITY_ID=%s\n' "$schema_id"
        printf 'MIGRATION_TARGET_RELEASE_SHA=%s\n' "$release_sha"
    } >"$state_dir/schema-state.env.tmp"
    chmod 600 "$state_dir/schema-state.env.tmp"
    mv "$state_dir/schema-state.env.tmp" "$state_dir/schema-state.env"
}

_oteryn_known_schema_identity() {
    local state_dir="$1" schema_file="$state_dir/schema-state.env" schema_state schema_id
    [[ -f "$schema_file" ]] || { echo "Managed schema identity is missing; refusing migration-bearing deployment." >&2; return 1; }
    schema_state="$(_oteryn_read_state_key "$schema_file" SCHEMA_STATE)" || return 1
    schema_id="$(_oteryn_read_state_key "$schema_file" SCHEMA_COMPATIBILITY_ID)" || return 1
    [[ "$schema_state" == known ]] || { echo "Managed schema identity is not known; refusing migration-bearing deployment." >&2; return 1; }
    [[ "$schema_id" =~ ^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$ ]] || { echo "Managed schema identity is invalid; refusing migration-bearing deployment." >&2; return 1; }
    printf '%s\n' "$schema_id"
}

_oteryn_bootstrap_legacy_current_release() {
    local state_dir="$1" current_file="$state_dir/current-release.env" legacy_file="$state_dir/last-good.env"
    local old_platform old_gateway old_sha observed_schema table_count
    local -a legacy_images
    [[ ! -f "$current_file" ]] || return 0

    if [[ ! -f "$legacy_file" ]]; then
        table_count="$(command docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE" exec -T \
            -e MYSQL_PWD="$MARIADB_ROOT_PASSWORD" mariadb mariadb -uroot -N -e \
            "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema='$PLATFORM_DB_NAME';")" || {
            echo "Cannot determine whether Platform DB is fresh; refusing migration." >&2
            return 1
        }
        [[ "$table_count" =~ ^[0-9]+$ ]] || { echo "Cannot determine whether Platform DB is fresh; refusing migration." >&2; return 1; }
        if (( table_count > 0 )); then
            echo "Existing Platform DB has no managed application baseline; refusing migration before backup-capable baseline is proven." >&2
            return 1
        fi
        return 0
    fi

    mapfile -t legacy_images < <(bash -c 'set -euo pipefail; source "$1"; printf "%s\n%s\n" "${PLATFORM_IMAGE:-}" "${GATEWAY_IMAGE:-}"' bash "$legacy_file")
    old_platform="${legacy_images[0]:-}"
    old_gateway="${legacy_images[1]:-}"
    [[ -n "$old_platform" && -n "$old_gateway" ]] || {
        echo "Legacy running-release snapshot is incomplete; refusing migration." >&2
        return 1
    }
    old_sha="$(_oteryn_release_sha_for_images "$old_platform" "$old_gateway")" || return 1

    observed_schema="observed-${old_sha}"
    # A legacy snapshot predates the native-only cut and needs the retired legacy-engine
    # topology, so it is recorded as ineligible for image rollback.
    bash "$SCRIPT_DIR/release-state.sh" write "$current_file" "$old_sha" "$old_sha" "$old_sha" \
        "$observed_schema" "$observed_schema" "$old_platform" "$old_gateway" 0
    _oteryn_write_schema_state_known "$state_dir" "$observed_schema" "$old_sha"
}

_oteryn_marketplace_state_file() {
    printf '%s/marketplace.env\n' "$(_oteryn_deploy_state_dir)"
}

# Durable Marketplace state written by earlier releases also carries retired escrow
# account and character-transfer keys. They are dropped: ignored, never re-emitted
# and never a reason to block a deploy. Every other unknown key still fails closed.
OTERYN_RETIRED_MARKETPLACE_STATE_KEYS=(
    MARKETPLACE_ESCROW_CANARY_ACCOUNT_ID
    MARKETPLACE_ESCROW_CANARY_ACCOUNT_NAME
    MARKETPLACE_ESCROW_CANARY_ACCOUNT_CREATION_EPOCH
    CANARY_CHARACTER_TRANSFER_DB_USER
    CANARY_CHARACTER_TRANSFER_DB_PASSWORD
)

_oteryn_marketplace_non_terminal_auction_count() {
    local -a db=(command docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE" exec -T
        -e MYSQL_PWD="$MARIADB_ROOT_PASSWORD" mariadb mariadb -uroot -N -B -e)
    local table_count pending

    table_count="$("${db[@]}" "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema='$PLATFORM_DB_NAME' AND table_name='character_auctions';")" || return 1
    [[ "$table_count" =~ ^[0-9]+$ ]] || return 1
    if [[ "$table_count" == 0 ]]; then
        printf '0\n'
        return 0
    fi
    pending="$("${db[@]}" "SELECT COUNT(*) FROM \`$PLATFORM_DB_NAME\`.character_auctions WHERE status IN ('escrow_pending','active','settlement_pending','cancel_pending','recovery_required');")" || return 1
    [[ "$pending" =~ ^[0-9]+$ ]] || return 1
    printf '%s\n' "$pending"
}

# Reverse of _oteryn_quiesce_platform_db_consumers for a refused migration: starts only
# the services that were running before, then the scheduler if it was running.
_oteryn_restart_after_refused_migration() {
    local was_running="$1" service scheduler_id
    local -a base_compose=(command docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE")

    for service in platform internal-proxy gateway; do
        if grep -qx "$service" <<<"$was_running"; then
            "${base_compose[@]}" start "$service" >/dev/null || true
        fi
    done
    if [[ "${OTERYN_MARKETPLACE_SCHEDULER_WAS_RUNNING:-0}" == 1 ]]; then
        scheduler_id="$(_oteryn_marketplace_scheduler_id)" || return 0
        [[ -z "$scheduler_id" ]] || command docker start "$scheduler_id" >/dev/null || true
    fi
    return 0
}

# Rewrites a durable state file to the current format (MARKETPLACE_ENABLED=false
# only): the retired keys are dropped and any enabled runtime is disabled, because
# the transfer contract is gone. Either rewrite needs a verified zero
# count of non-terminal auctions (N > 0 is refused). When no count can be obtained
# (database not running), it fails closed unless the operator attests, with
# OTERYN_ATTEST_NO_PENDING_AUCTIONS=1, that no auction can be pending. Idempotent.
# With the argument "quiesce" (the deploy path), Platform, Gateway, the internal proxy
# and the scheduler are stopped before the count so no listing can be accepted between
# the count and the key removal; the deploy brings them back up afterwards.
_oteryn_migrate_legacy_marketplace_state() {
    local quiesce="${1:-}"
    local durable_file line key value enabled='' legacy=0 retired pending tmp

    durable_file="$(_oteryn_marketplace_state_file)"
    [[ -f "$durable_file" ]] || return 0

    while IFS= read -r line || [[ -n "$line" ]]; do
        line="${line%$'\r'}"
        [[ -z "$line" || "$line" =~ ^[[:space:]]*# ]] && continue
        [[ "$line" == *=* ]] || { echo "Invalid Marketplace state line." >&2; return 1; }
        key="${line%%=*}"
        value="${line#*=}"
        if [[ "$key" == MARKETPLACE_ENABLED ]]; then
            [[ -z "$enabled" ]] || { echo "Duplicate Marketplace state key: $key" >&2; return 1; }
            enabled="$value"
            continue
        fi
        for retired in "${OTERYN_RETIRED_MARKETPLACE_STATE_KEYS[@]}"; do
            if [[ "$key" == "$retired" ]]; then
                legacy=1
                continue 2
            fi
        done
        echo "Unexpected durable Marketplace state key: $key" >&2
        return 1
    done < "$durable_file"

    [[ "$legacy" == 1 || "$enabled" == true ]] || return 0
    [[ "$enabled" == true || "$enabled" == false ]] || {
        echo "Marketplace runtime state has invalid enabled flag." >&2
        return 1
    }

    # Any rewrite (retired keys or an enabled flag) disables the runtime, so it needs the drain proof.
    if [[ "$legacy" == 1 || "$enabled" == true ]]; then
        local was_running='' refused=''
        if [[ "$quiesce" == quiesce ]]; then
            was_running="$(command docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE" ps --services --status running 2>/dev/null | grep -E '^(platform|internal-proxy|gateway)$' || true)"
            _oteryn_quiesce_platform_db_consumers || { _oteryn_restart_after_refused_migration "$was_running" || true; return 1; }
        fi
        if pending="$(_oteryn_marketplace_non_terminal_auction_count 2>/dev/null)" && [[ "$pending" =~ ^[0-9]+$ ]]; then
            if [[ "$pending" -gt 0 ]]; then
                echo "Legacy Character Bazaar has $pending non-terminal auctions; drain them before disabling the Marketplace state." >&2
                refused=1
            fi
        elif [[ "${OTERYN_ATTEST_NO_PENDING_AUCTIONS:-}" != 1 ]]; then
            echo "Cannot prove the legacy Character Bazaar is drained; refusing to disable the Marketplace state. If no auction can be pending, set OTERYN_ATTEST_NO_PENDING_AUCTIONS=1 to attest that." >&2
            refused=1
        fi
        if [[ -n "$refused" ]]; then
            # Bring back exactly what the quiesce stopped (best effort; the refusal stays the result).
            if [[ "$quiesce" == quiesce ]]; then
                _oteryn_restart_after_refused_migration "$was_running" || true
            fi
            return 1
        fi
    fi

    tmp="$durable_file.tmp"
    ( umask 077; printf 'MARKETPLACE_ENABLED=false\n' >"$tmp" )
    chmod 600 "$tmp"
    mv "$tmp" "$durable_file"
    echo "Rewrote the durable Marketplace state as disabled without the retired keys."
}

_oteryn_load_marketplace_runtime_state() {
    local durable_file source_file line key value seen='|'
    local -a required=(
        MARKETPLACE_ENABLED
    )
    durable_file="$(_oteryn_marketplace_state_file)"
    source_file="$durable_file"
    if [[ -f "$ENV_FILE" ]] && grep -q '^MARKETPLACE_ENABLED=' "$ENV_FILE"; then
        source_file="$ENV_FILE"
    fi
    [[ -f "$source_file" ]] || return 2
    if [[ "$source_file" == "$durable_file" ]]; then
        _oteryn_migrate_legacy_marketplace_state || return 1
    fi

    unset MARKETPLACE_ENABLED

    while IFS= read -r line || [[ -n "$line" ]]; do
        line="${line%$'\r'}"
        [[ -z "$line" || "$line" =~ ^[[:space:]]*# ]] && continue
        [[ "$line" == *=* ]] || { echo "Invalid Marketplace state line." >&2; return 1; }
        key="${line%%=*}"
        value="${line#*=}"
        case "$key" in
            MARKETPLACE_ENABLED)
                if [[ "$seen" == *"|$key|"* ]]; then
                    echo "Duplicate Marketplace state key: $key" >&2
                    return 1
                fi
                seen="${seen}${key}|"
                printf -v "$key" '%s' "$value"
                export "$key"
                ;;
            *)
                [[ "$source_file" != "$durable_file" ]] || { echo "Unexpected durable Marketplace state key: $key" >&2; return 1; }
                ;;
        esac
    done < "$source_file"

    local name
    for name in "${required[@]}"; do
        [[ -n "${!name:-}" ]] || { echo "Marketplace runtime state is incomplete: $name" >&2; return 1; }
    done
    [[ "$MARKETPLACE_ENABLED" == true || "$MARKETPLACE_ENABLED" == false ]] || { echo "Marketplace runtime state has invalid enabled flag." >&2; return 1; }
}

_oteryn_marketplace_scheduler_id() {
    local ids count
    ids="$(command docker ps -a \
        --filter "label=com.docker.compose.project=${COMPOSE_PROJECT_NAME:-oteryn-staging}" \
        --filter 'label=com.docker.compose.service=marketplace-scheduler' \
        --format '{{.ID}}' | sed '/^$/d')" || return 1
    count="$(printf '%s\n' "$ids" | sed '/^$/d' | wc -l | tr -d ' ')"
    [[ "$count" -le 1 ]] || { echo "Expected at most one Marketplace scheduler container." >&2; return 1; }
    printf '%s\n' "$ids"
}

_oteryn_recreate_marketplace_runtime() {
    local marketplace_file="$DEPLOY_DIR/compose.marketplace.yml"
    local -a marketplace_compose=(command docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE" -f "$marketplace_file")
    [[ -f "$marketplace_file" ]] || { echo "Marketplace Compose file is missing." >&2; return 1; }
    _oteryn_load_marketplace_runtime_state || { echo "Cannot prove Marketplace runtime settings for runtime recreation." >&2; return 1; }
    [[ "$MARKETPLACE_ENABLED" == true ]] || { echo "Refusing to recreate Marketplace runtime while durable/effective state is disabled." >&2; return 1; }

    # From the ingress stop on, every failure leaves Platform, the internal proxy and Gateway removed
    # (fail closed): the callers may have had ingress running, and a stopped container would come
    # back through `restart: always`.
    if _oteryn_recreate_marketplace_runtime_gated "${marketplace_compose[@]}"; then
        return 0
    fi
    echo "Marketplace runtime recreation failed; removing the Platform, internal proxy and Gateway containers (fail closed)." >&2
    "${marketplace_compose[@]}" rm -s -f gateway internal-proxy platform >&2 || true
    return 1
}

_oteryn_recreate_marketplace_runtime_gated() {
    local platform_id scheduler_id platform_running scheduler_running platform_image scheduler_image
    local platform_enabled scheduler_enabled ingress
    local -a marketplace_compose=("$@")

    # Same order as every Platform route: ingress stays stopped until the recreated Platform passes the
    # trust gate and the self-check; only the ingress containers that were running before start again.
    ingress="$(oteryn_native_admission_ingress_stop "${marketplace_compose[@]}")" || return 1
    "${marketplace_compose[@]}" up -d --no-deps --force-recreate platform || return 1
    oteryn_publish_native_admission_trust_or_stop "${marketplace_compose[@]}" || return 1
    # The replacement Platform is the one that will serve: prove it can sign before ingress returns.
    oteryn_native_admission_self_check_or_stop "${marketplace_compose[@]}" || return 1
    # shellcheck disable=SC2086 # container ids, one per line
    oteryn_native_admission_ingress_start $ingress || return 1
    "${marketplace_compose[@]}" up -d --force-recreate marketplace-scheduler || return 1

    platform_id="$("${marketplace_compose[@]}" ps -a -q platform)" || return 1
    scheduler_id="$(_oteryn_marketplace_scheduler_id)" || return 1
    [[ -n "$platform_id" && -n "$scheduler_id" ]] || { echo "Marketplace Platform/scheduler runtime was not recreated." >&2; return 1; }

    platform_running="$(command docker inspect --format '{{.State.Running}}' "$platform_id")" || return 1
    scheduler_running="$(command docker inspect --format '{{.State.Running}}' "$scheduler_id")" || return 1
    platform_image="$(command docker inspect --format '{{.Config.Image}}' "$platform_id")" || return 1
    scheduler_image="$(command docker inspect --format '{{.Config.Image}}' "$scheduler_id")" || return 1
    platform_enabled="$(command docker inspect --format '{{range .Config.Env}}{{println .}}{{end}}' "$platform_id" | sed -n 's/^MARKETPLACE_ENABLED=//p' | head -n 1)"
    scheduler_enabled="$(command docker inspect --format '{{range .Config.Env}}{{println .}}{{end}}' "$scheduler_id" | sed -n 's/^MARKETPLACE_ENABLED=//p' | head -n 1)"

    [[ "$platform_running" == true && "$scheduler_running" == true ]] || { echo "Marketplace Platform/scheduler is not fully running after recreation." >&2; return 1; }
    [[ "$platform_image" == "$PLATFORM_IMAGE" ]] || { echo "Marketplace Platform image does not match the selected Platform image." >&2; return 1; }
    [[ "$scheduler_image" == "$PLATFORM_IMAGE" ]] || { echo "Marketplace scheduler image does not match the selected Platform image." >&2; return 1; }
    [[ "$platform_enabled" == true && "$scheduler_enabled" == true ]] || { echo "Marketplace runtime did not receive enabled settings." >&2; return 1; }
}

_oteryn_recreate_marketplace_scheduler() {
    # Compatibility name retained for the existing contract test; recreation now
    # covers both the browser-facing Platform container and scheduler.
    _oteryn_recreate_marketplace_runtime
}

_oteryn_quiesce_platform_db_consumers() {
    local scheduler_id scheduler_running
    local -a base_compose=(command docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE")

    OTERYN_MARKETPLACE_SCHEDULER_WAS_RUNNING=0
    export OTERYN_MARKETPLACE_SCHEDULER_WAS_RUNNING
    scheduler_id="$(_oteryn_marketplace_scheduler_id)" || return 1
    if [[ -n "$scheduler_id" ]]; then
        scheduler_running="$(command docker inspect --format '{{.State.Running}}' "$scheduler_id")" || return 1
        if [[ "$scheduler_running" == true ]]; then
            OTERYN_MARKETPLACE_SCHEDULER_WAS_RUNNING=1
            export OTERYN_MARKETPLACE_SCHEDULER_WAS_RUNNING
            command docker stop "$scheduler_id" >/dev/null
            scheduler_running="$(command docker inspect --format '{{.State.Running}}' "$scheduler_id")" || return 1
            [[ "$scheduler_running" == false ]] || {
                echo "Deployment rejected: marketplace-scheduler is still running before migration backup." >&2
                return 1
            }
        fi
    fi

    "${base_compose[@]}" stop platform gateway internal-proxy
}

_oteryn_reconcile_marketplace_scheduler_after_runtime_change() {
    local scheduler_id state_rc
    scheduler_id="$(_oteryn_marketplace_scheduler_id)" || return 1
    set +e
    _oteryn_load_marketplace_runtime_state
    state_rc=$?
    set -e
    if [[ "$state_rc" -eq 2 ]]; then
        [[ -z "$scheduler_id" ]] || { echo "Marketplace scheduler exists but no durable/effective Marketplace state is available." >&2; return 1; }
        return 0
    fi
    [[ "$state_rc" -eq 0 ]] || return "$state_rc"

    if [[ "$MARKETPLACE_ENABLED" == true ]]; then
        _oteryn_recreate_marketplace_scheduler || return 1
    elif [[ -n "$scheduler_id" ]]; then
        command docker rm -f "$scheduler_id" >/dev/null
    fi
}

_oteryn_restore_quiesced_consumers_after_migrate() {
    _oteryn_reconcile_marketplace_scheduler_after_runtime_change || return 1
    OTERYN_MARKETPLACE_SCHEDULER_WAS_RUNNING=0
    export OTERYN_MARKETPLACE_SCHEDULER_WAS_RUNNING
}

_oteryn_resume_candidate_if_safe() {
    local state_dir="$1" release_sha="$2"
    local candidate_file="$state_dir/candidate-release.env" schema_file="$state_dir/schema-state.env"
    local schema_state schema_id schema_target candidate_schema candidate_accepts
    local -a candidate_runtime

    [[ -f "$candidate_file" ]] || return 1
    bash "$SCRIPT_DIR/release-state.sh" validate "$candidate_file" || return 1
    [[ -f "$schema_file" ]] || {
        echo "Candidate resume rejected: managed schema state is missing." >&2
        return 1
    }

    mapfile -t candidate_runtime < <(bash -c '
        set -euo pipefail
        source "$1"
        printf "%s\n" \
            "$RELEASE_SHA" "$PLATFORM_SOURCE_SHA" "$GATEWAY_SOURCE_SHA" \
            "$PLATFORM_IMAGE" "$GATEWAY_IMAGE" \
            "${GAME_WORLD_ID:-}" "${GAME_WORLD_SLUG:-}" "${GAME_WORLD_NAME:-}" \
            "${GAME_WORLD_REGION:-}" "${GAME_WORLD_HOST:-}" "${GAME_WORLD_PORT:-}"
    ' bash "$candidate_file")

    [[ "${candidate_runtime[0]:-}" == "$release_sha" ]] || {
        echo "Deployment rejected: unresolved candidate release ${candidate_runtime[0]:-UNKNOWN} differs from requested release $release_sha." >&2
        return 1
    }
    [[ "${candidate_runtime[1]:-}" == "$PLATFORM_SOURCE_SHA" \
        && "${candidate_runtime[2]:-}" == "$GATEWAY_SOURCE_SHA" ]] || {
        echo "Candidate resume rejected: component source identity drifted." >&2
        return 1
    }
    [[ "${candidate_runtime[3]:-}" == "$PLATFORM_IMAGE" \
        && "${candidate_runtime[4]:-}" == "$GATEWAY_IMAGE" ]] || {
        echo "Candidate resume rejected: immutable runtime image identity drifted." >&2
        return 1
    }
    [[ "${candidate_runtime[5]:-}" == "${GAME_WORLD_ID:-}" \
        && "${candidate_runtime[6]:-}" == "${GAME_WORLD_SLUG:-}" \
        && "${candidate_runtime[7]:-}" == "${GAME_WORLD_NAME:-}" \
        && "${candidate_runtime[8]:-}" == "${GAME_WORLD_REGION:-}" \
        && "${candidate_runtime[9]:-}" == "${GAME_WORLD_HOST:-}" \
        && "${candidate_runtime[10]:-}" == "${GAME_WORLD_PORT:-}" ]] || {
        echo "Candidate resume rejected: staged world identity drifted." >&2
        return 1
    }

    schema_state="$(_oteryn_read_state_key "$schema_file" SCHEMA_STATE)" || return 1
    schema_id="$(_oteryn_read_state_key "$schema_file" SCHEMA_COMPATIBILITY_ID)" || return 1
    schema_target="$(_oteryn_read_state_key "$schema_file" MIGRATION_TARGET_RELEASE_SHA)" || return 1
    candidate_schema="$(_oteryn_read_state_key "$candidate_file" SCHEMA_COMPATIBILITY_ID)" || return 1
    candidate_accepts="$(_oteryn_read_state_key "$candidate_file" APP_ACCEPTS_SCHEMA_IDS)" || return 1

    [[ "$schema_state" == known && "$schema_target" == "$release_sha" && "$schema_id" == "$candidate_schema" ]] || {
        echo "Candidate resume rejected: prior migration is not proven complete for the exact candidate release." >&2
        return 1
    }
    _oteryn_schema_list_contains "$schema_id" "$candidate_accepts" || {
        echo "Candidate resume rejected: candidate application does not accept the proven schema." >&2
        return 1
    }

    OTERYN_SAME_RELEASE_REDEPLOY=1
    export OTERYN_SAME_RELEASE_REDEPLOY
    echo "Resuming exact candidate release $release_sha after a previously successful migration; preserving recovery evidence until health checks pass."
    return 0
}

_oteryn_before_platform_migrate() {
    local state_dir release_sha backup_dir backup_file backup_meta current_file old_sha old_schema
    state_dir="$(_oteryn_deploy_state_dir)"
    release_sha="$(_oteryn_release_sha)" || return 1

    # Drop retired durable Marketplace state keys before any schema change.
    if ! { [[ -f "$ENV_FILE" ]] && grep -q '^MARKETPLACE_ENABLED=' "$ENV_FILE"; }; then
        _oteryn_migrate_legacy_marketplace_state quiesce || return 1
    fi

    if [[ -f "$state_dir/candidate-release.env" ]]; then
        _oteryn_resume_candidate_if_safe "$state_dir" "$release_sha" || return 1
        return 0
    fi

    _oteryn_load_candidate_contract || return 1
    mkdir -p "$state_dir/backups"
    chmod 700 "$state_dir" "$state_dir/backups"

    _oteryn_bootstrap_legacy_current_release "$state_dir" || return 1

    bash "$SCRIPT_DIR/release-state.sh" write "$state_dir/candidate-release.env" "$release_sha" \
        "$PLATFORM_SOURCE_SHA" "$GATEWAY_SOURCE_SHA" \
        "$OTERYN_SCHEMA_COMPATIBILITY_ID" "$OTERYN_APP_ACCEPTS_SCHEMA_IDS" \
        "$PLATFORM_IMAGE" "$GATEWAY_IMAGE" 1

    current_file="$state_dir/current-release.env"
    if [[ -f "$current_file" ]]; then
        bash "$SCRIPT_DIR/release-state.sh" validate "$current_file"
        old_sha="$(_oteryn_read_state_key "$current_file" RELEASE_SHA)" || return 1
        old_schema="$(_oteryn_known_schema_identity "$state_dir")" || return 1

        if [[ "$old_sha" == "$release_sha" ]]; then
            _oteryn_schema_list_contains "$old_schema" "$OTERYN_APP_ACCEPTS_SCHEMA_IDS" || {
                echo "Same-release redeploy rejected: candidate application does not accept the known current schema." >&2
                return 1
            }
            OTERYN_SAME_RELEASE_REDEPLOY=1
            export OTERYN_SAME_RELEASE_REDEPLOY
            return 0
        fi

        cp "$current_file" "$state_dir/last-good-release.env.tmp"
        chmod 600 "$state_dir/last-good-release.env.tmp"
        mv "$state_dir/last-good-release.env.tmp" "$state_dir/last-good-release.env"

        _oteryn_quiesce_platform_db_consumers || return 1

        backup_dir="$state_dir/backups/${old_sha}-before-${release_sha}"
        mkdir -p "$backup_dir"
        chmod 700 "$backup_dir"
        backup_file="$backup_dir/platform.sql"
        command docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE" exec -T \
            -e MYSQL_PWD="$MARIADB_ROOT_PASSWORD" mariadb \
            mariadb-dump -uroot --single-transaction --routines --triggers --events "$PLATFORM_DB_NAME" >"$backup_file.tmp"
        chmod 600 "$backup_file.tmp"
        mv "$backup_file.tmp" "$backup_file"
        backup_meta="$backup_dir/evidence.env"
        {
            printf 'BACKUP_FROM_RELEASE_SHA=%s\n' "$old_sha"
            printf 'BACKUP_BEFORE_RELEASE_SHA=%s\n' "$release_sha"
            printf 'BACKUP_SCHEMA_COMPATIBILITY_ID=%s\n' "$old_schema"
            printf 'BACKUP_COMPOSE_PROJECT_NAME=%s\n' "${COMPOSE_PROJECT_NAME:-oteryn-staging}"
            printf 'BACKUP_PLATFORM_DB_NAME=%s\n' "$PLATFORM_DB_NAME"
            printf 'BACKUP_SHA256=%s\n' "$(sha256sum "$backup_file" | awk '{print $1}')"
        } >"$backup_meta.tmp"
        chmod 600 "$backup_meta.tmp"
        mv "$backup_meta.tmp" "$backup_meta"
    fi

    {
        printf 'SCHEMA_STATE=unknown\n'
        printf 'MIGRATION_TARGET_RELEASE_SHA=%s\n' "$release_sha"
    } >"$state_dir/schema-state.env.tmp"
    chmod 600 "$state_dir/schema-state.env.tmp"
    mv "$state_dir/schema-state.env.tmp" "$state_dir/schema-state.env"
}

_oteryn_after_platform_migrate() {
    local state_dir release_sha
    state_dir="$(_oteryn_deploy_state_dir)"
    release_sha="$(_oteryn_release_sha)" || return 1
    _oteryn_write_schema_state_known "$state_dir" "$OTERYN_SCHEMA_COMPATIBILITY_ID" "$release_sha"
    _oteryn_restore_quiesced_consumers_after_migrate || return 1
}

_oteryn_finalize_release_on_exit() {
    local rc="$?" state_dir
    [[ "$rc" -eq 0 ]] || return "$rc"
    [[ "$(basename -- "${0:-}")" == "deploy.sh" ]] || return 0
    state_dir="$(_oteryn_deploy_state_dir)"
    bash "$SCRIPT_DIR/release-state.sh" validate "$state_dir/candidate-release.env"
    cp "$state_dir/candidate-release.env" "$state_dir/current-release.env.tmp"
    chmod 600 "$state_dir/current-release.env.tmp"
    mv "$state_dir/current-release.env.tmp" "$state_dir/current-release.env"
    rm -f "$state_dir/candidate-release.env"
    return 0
}

docker() {
    local -a args=("$@")
    local i joined rc
    for i in "${!args[@]}"; do
        case "${args[$i]}" in
            alpine:3.22) args[$i]="$OTERYN_HEALTH_ALPINE_IMAGE" ;;
            python:3.12-alpine) args[$i]="$OTERYN_HEALTH_PYTHON_IMAGE" ;;
        esac
    done

    if [[ "$(basename -- "${0:-}")" == "deploy.sh" ]]; then
        joined=" ${args[*]} "
        if [[ "${OTERYN_MIGRATION_PREPARED:-0}" != 1 && "$joined" == *" up -d platform "* ]]; then
            _oteryn_before_platform_migrate || return 1
            command docker "${args[@]}"
            rc=$?
            [[ "$rc" -eq 0 ]] || return "$rc"
            OTERYN_MIGRATION_PREPARED=1
            export OTERYN_MIGRATION_PREPARED
            return 0
        fi
        if [[ "$joined" == *" exec -T platform php artisan migrate --force --no-interaction "* ]]; then
            [[ "${OTERYN_MIGRATION_PREPARED:-0}" == 1 ]] || _oteryn_before_platform_migrate || return 1
            if [[ "${OTERYN_SAME_RELEASE_REDEPLOY:-0}" == 1 ]]; then
                echo "Same-release redeploy: skipping migration; the exact current image already proved this schema transition."
                OTERYN_SAME_RELEASE_REDEPLOY=0
                OTERYN_MIGRATION_PREPARED=0
                export OTERYN_SAME_RELEASE_REDEPLOY OTERYN_MIGRATION_PREPARED
                _oteryn_reconcile_marketplace_scheduler_after_runtime_change || return 1
                return 0
            fi
            command docker "${args[@]}"
            rc=$?
            if [[ "$rc" -eq 0 ]]; then
                _oteryn_after_platform_migrate || return 1
                OTERYN_MIGRATION_PREPARED=0
                export OTERYN_MIGRATION_PREPARED
            fi
            return "$rc"
        fi
    fi
    command docker "${args[@]}"
}

# Owner decision D1006: with native admission switched on, every Platform (re)creation for a
# release, rollback or Marketplace switch trusts the PUBLIC half of the
# Platform issuer's current signing key in the deployment's own signing trust registry (MariaDB),
# after the key exists and before the signing self-check verifies against that registry. The
# command derives the public key from the loaded private key, refuses outside testing or
# preproduction, keeps every registry rule (a kid is bound to one key forever, revoked kids stay
# revoked, at most two trusted keys, high-water witness on every write) and is idempotent. It
# prints one JSON line with the trust scope, key id and public key only: the values Game's trust
# configuration needs.
# The witness directory lives on the platform_storage volume shared by Platform and the
# marketplace scheduler. Once it is configured, every identity security revocation writes through
# it, so it is created owner-only on every deploy, whatever the native admission switch says. A
# missing witness for a registry with trust history fails closed.
oteryn_publish_native_admission_trust() {
    local container_id="${1:-}"
    local flag output

    [[ -n "$container_id" ]] || { echo "Native admission trust publication needs the Platform container." >&2; return 1; }
    if ! flag="$(docker exec "$container_id" sh -ec '
        test -n "${GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY:-}"
        umask 077
        mkdir -p -- "$GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY"
        test ! -L "$GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY"
        chmod 700 -- "$GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY"
        test -O "$GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY"
        test "$(stat -c %a -- "$GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY")" = 700
        printf %s "${GAME_AUTH_NATIVE_ADMISSION_ENABLED:-}"
    ')"; then
        echo "Unable to prepare the native evidence witness directory (owner-only, not a link, owned by the Platform user) or read the Platform native admission flag." >&2
        return 1
    fi
    if [[ "$flag" != true ]]; then
        echo "Native admission is off; no signing key is trusted."
        return 0
    fi
    # Through the image entrypoint, exactly like the signing self-check: it exports the signing key id
    # and file path from the key volume, which a bare docker exec does not see.
    if ! output="$(docker exec "$container_id" /usr/local/bin/oteryn-platform-entrypoint \
        php artisan game-auth:native-admission:publish-trust --no-interaction 2>&1)"; then
        printf '%s\n' "$output" >&2
        echo "Native admission signing key trust publication refused; deploy stops before the signing self-check." >&2
        return 1
    fi
    if [[ ! "$output" =~ \"result\":\"(published|already_trusted)\" ]]; then
        echo "Native admission trust publication returned no trust result." >&2
        return 1
    fi
    echo "Native admission signing key trusted (PUBLIC values for Game's trust configuration): $output"
}

# NATIVE-LOGIN-E2E-1: publishes the Game world's native route record (game-auth:native-route:publish)
# as part of the deploy, driven by repository variables (no secret, no key). Opt-in:
# OTERYN_NATIVE_ROUTE_PUBLISH_ENABLED=true (repository variable OTERYN_STAGING_NATIVE_ROUTE_PUBLISH_ENABLED).
# When on, every required value must be present and well-formed or the deploy refuses (fail closed);
# nothing is guessed. Order: the Registry row must exist (game-auth:world:ensure ran), then
# game-auth:native-topology:issue (idempotent) issues the WorldId/ChannelId, which must equal the
# configured native World (OTERYN_STAGING_NATIVE_WORLD_ID, the D171 World), then the route is
# published and read back. Descriptor reference: Game deploy/synology-game/preprod-topology.toml
# ([scope] channel_key, [route] host, port, tls_server_name). Arguments: the route's Compose
# command words.
oteryn_native_route_publish_validate() {
    local missing=() name
    for name in OTERYN_NATIVE_CHANNEL_KEY OTERYN_NATIVE_ROUTE_HOST OTERYN_NATIVE_ROUTE_PORT OTERYN_NATIVE_ROUTE_TLS_SERVER_NAME GAME_AUTH_NATIVE_ADMISSION_UNVERIFIED_CHARACTER_WORLD_ID; do
        [[ -n "${!name:-}" ]] || missing+=("$name")
    done
    if ((${#missing[@]} > 0)); then
        echo "Native route publication is enabled but required values are missing: ${missing[*]} (repository variables OTERYN_STAGING_NATIVE_CHANNEL_KEY, OTERYN_STAGING_NATIVE_ROUTE_HOST, OTERYN_STAGING_NATIVE_ROUTE_PORT, OTERYN_STAGING_NATIVE_ROUTE_TLS_SERVER_NAME, OTERYN_STAGING_NATIVE_WORLD_ID)." >&2
        return 1
    fi
    if [[ "${OTERYN_NATIVE_ADMISSION_ENABLED:-false}" != true ]]; then
        echo "Native route publication needs native admission enabled (OTERYN_STAGING_NATIVE_ADMISSION_ENABLED=true)." >&2
        return 1
    fi
    if [[ ! "$OTERYN_NATIVE_CHANNEL_KEY" =~ ^[a-z0-9][a-z0-9-]{0,63}$ ]]; then
        echo "OTERYN_STAGING_NATIVE_CHANNEL_KEY must be a lower-case channel key of at most 64 characters (a-z, 0-9, hyphen)." >&2
        return 1
    fi
    if [[ ! "$OTERYN_NATIVE_ROUTE_PORT" =~ ^[1-9][0-9]{0,4}$ ]] || ((OTERYN_NATIVE_ROUTE_PORT > 65535)); then
        echo "OTERYN_STAGING_NATIVE_ROUTE_PORT must be a TCP port from 1 to 65535." >&2
        return 1
    fi
    if [[ ! "$OTERYN_NATIVE_ROUTE_HOST" =~ ^[A-Za-z0-9]([A-Za-z0-9.-]{0,251}[A-Za-z0-9])?$ ]]; then
        echo "OTERYN_STAGING_NATIVE_ROUTE_HOST must be a host name or IPv4 address without scheme, port or path." >&2
        return 1
    fi
    if [[ ! "$OTERYN_NATIVE_ROUTE_TLS_SERVER_NAME" =~ ^[A-Za-z0-9]([A-Za-z0-9.-]{0,251}[A-Za-z0-9])?$ ]]; then
        echo "OTERYN_STAGING_NATIVE_ROUTE_TLS_SERVER_NAME must be a DNS name the Game client verifies." >&2
        return 1
    fi
    case "${OTERYN_NATIVE_ROUTE_LOGIN_ENABLED:-true}" in
        true | false) ;;
        *) echo "OTERYN_STAGING_NATIVE_ROUTE_LOGIN_ENABLED must be true or false." >&2; return 1 ;;
    esac
    if [[ ! "${GAME_WORLD_ID:-}" =~ ^[1-9][0-9]*$ ]]; then
        echo "GAME_WORLD_ID (the Registry world row) must be a positive integer for native route publication." >&2
        return 1
    fi
}

oteryn_publish_native_route() {
    local issued published issued_world
    (($# > 0)) || { echo "Native route publication needs the Compose command." >&2; return 1; }
    case "${OTERYN_NATIVE_ROUTE_PUBLISH_ENABLED:-false}" in
        true) ;;
        false | '') echo "Native route publication is off (OTERYN_STAGING_NATIVE_ROUTE_PUBLISH_ENABLED is not true)."; return 0 ;;
        *) echo "OTERYN_STAGING_NATIVE_ROUTE_PUBLISH_ENABLED must be true or false." >&2; return 1 ;;
    esac
    oteryn_native_route_publish_validate || return 1

    if ! issued="$("$@" exec -T platform php artisan game-auth:native-topology:issue \
        --world-row-id="$GAME_WORLD_ID" --channel-key="$OTERYN_NATIVE_CHANNEL_KEY" --no-interaction 2>&1)"; then
        printf '%s\n' "$issued" >&2
        echo "Native topology issuance/readback failed; the native route was not published." >&2
        return 1
    fi
    issued_world="$(sed -n 's/.*"world_id":"\([^"]*\)".*/\1/p' <<<"$issued" | head -n1)"
    if [[ -z "$issued_world" ]]; then
        echo "Native topology issuance returned no WorldId; the native route was not published." >&2
        return 1
    fi
    if [[ "$issued_world" != "$GAME_AUTH_NATIVE_ADMISSION_UNVERIFIED_CHARACTER_WORLD_ID" ]]; then
        echo "The Registry issued native WorldId $issued_world, which differs from OTERYN_STAGING_NATIVE_WORLD_ID; set that repository variable to the issued WorldId (not a secret) and redeploy. The native route was not published." >&2
        return 1
    fi

    echo "Native topology (non-secret receipt for the Game topology descriptor [scope]): $issued"
    if ! published="$("$@" exec -T platform php artisan game-auth:native-route:publish \
        --world-row-id="$GAME_WORLD_ID" --channel-key="$OTERYN_NATIVE_CHANNEL_KEY" \
        --host="$OTERYN_NATIVE_ROUTE_HOST" --port="$OTERYN_NATIVE_ROUTE_PORT" \
        --tls-server-name="$OTERYN_NATIVE_ROUTE_TLS_SERVER_NAME" \
        --login-enabled="${OTERYN_NATIVE_ROUTE_LOGIN_ENABLED:-true}" --no-interaction 2>&1)"; then
        printf '%s\n' "$published" >&2
        echo "Native route publication refused; native login has no route record." >&2
        return 1
    fi
    if [[ ! "$published" =~ \"route_revision\":\"[^\"]+\" ]]; then
        echo "Native route publication returned no route revision." >&2
        return 1
    fi
    echo "Native route record published (non-secret receipt for the Game topology descriptor): $published"
}

# Native admission grants reach Platform only through the internal proxy and Gateway (ingress):
# NativeAdmissionGrantIssuer reads the trusted key without consulting the witness. So every route
# that (re)creates Platform follows one order: stop ingress, (re)create Platform, pass the trust
# gate below, and only then start ingress. No request can reach a Platform whose trust publication
# and witness are unproven, so no grant can be issued from ambiguous trust state.
#
# oteryn_native_admission_ingress_stop <compose command words>: prints the ids of the ingress
# containers that were running (start order), then stops Gateway and the internal proxy.
oteryn_native_admission_ingress_stop() {
    local service id
    local -a running=()
    (($# > 0)) || { echo "Native admission ingress stop needs the Compose command." >&2; return 1; }
    for service in internal-proxy gateway; do
        id="$("$@" ps -q "$service")" || return 1
        [[ -z "$id" ]] || running+=("$id")
    done
    "$@" stop gateway internal-proxy >&2 || return 1
    ((${#running[@]} == 0)) || printf '%s\n' "${running[@]}"
}

# Starts the given ingress container ids again (the output of oteryn_native_admission_ingress_stop,
# or a route's preserved ids). Only call it after the trust gate passed.
oteryn_native_admission_ingress_start() {
    local id
    local -a ids=()
    for id in "$@"; do
        [[ -z "$id" ]] || ids+=("$id")
    done
    ((${#ids[@]} == 0)) || command docker start "${ids[@]}" >/dev/null
}

# The trust gate. The arguments are the route's Compose command words (for example
# "${compose[@]}"); ingress must already be stopped. When witness preparation or publication fails,
# Platform, the internal proxy and Gateway are removed (fail closed): a stopped container with
# `restart: always` would come back on the next Docker daemon or NAS restart. The caller aborts
# before ingress, health-check, promotion or scheduler restart. Volumes are kept; a later deploy or
# rollback recreates the containers only through this gate.
oteryn_publish_native_admission_trust_or_stop() {
    (($# > 0)) || { echo "Native admission trust gate needs the Compose command." >&2; return 1; }
    if oteryn_publish_native_admission_trust "$("$@" ps -q platform)"; then
        return 0
    fi
    echo "Native admission trust gate failed; removing the Platform, internal proxy and Gateway containers (fail closed)." >&2
    "$@" rm -s -f gateway internal-proxy platform >&2 || \
        echo "Unable to remove Platform, internal proxy and Gateway after the failed trust gate." >&2
    return 1
}

# Self-check gate for the Platform that will really serve: oteryn_native_admission_self_check_or_stop
# <compose command words>. The running Platform container's own environment decides whether admission
# is on (a failed docker call is a gate failure, never "disabled"); when it is on the signing and
# projection self-check must pass. Any failure removes Platform, the internal proxy and Gateway (they
# are `restart: always`), so no restartable ingress stays behind an unproven Platform.
oteryn_native_admission_self_check_or_stop() {
    local id env_out
    (($# > 0)) || { echo "Native admission self-check gate needs the Compose command." >&2; return 1; }
    if id="$("$@" ps -q platform)" && [[ -n "$id" ]] \
        && env_out="$(command docker inspect --format '{{range .Config.Env}}{{println .}}{{end}}' "$id")" \
        && { ! grep -qx 'GAME_AUTH_NATIVE_ADMISSION_ENABLED=true' <<<"$env_out" || oteryn_run_native_admission_self_check "$id"; }; then
        return 0
    fi
    echo "Native admission self-check gate failed; removing the Platform, internal proxy and Gateway containers (fail closed)." >&2
    "$@" rm -s -f gateway internal-proxy platform >&2 || \
        echo "Unable to remove Platform, internal proxy and Gateway after the failed self-check gate." >&2
    return 1
}

# Failure restore of the previous Platform for the fast-deploy and Platform-reconcile traps:
# oteryn_restore_previous_platform_behind_trust_gate <label>, with the previous release's images
# exported. Same order as a release: ingress stopped, previous Platform recreated, trust gate, then
# the internal proxy and Gateway (started again, or recreated when a failed forward gate removed
# them) and the scheduler. Any failure leaves Platform, internal proxy and Gateway removed.
oteryn_restore_previous_platform_behind_trust_gate() {
    local label="$1"
    local -a base=(command docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE")
    if ! oteryn_native_admission_ingress_stop "${base[@]}" >/dev/null 2>&1; then
        "${base[@]}" rm -s -f gateway internal-proxy platform >/dev/null 2>&1 || true
        echo "$label could not stop the internal proxy and Gateway; it removed the runtime containers (fail closed)." >&2
        return 1
    fi
    "${base[@]}" up -d --no-deps --force-recreate platform >/dev/null 2>&1 || \
        echo "$label could not recreate the previous Platform container." >&2
    if ! oteryn_publish_native_admission_trust_or_stop "${base[@]}"; then
        echo "$label left Platform, internal proxy and Gateway removed: the previous Platform failed the native admission trust gate." >&2
        return 1
    fi
    local rc=0
    "${base[@]}" up -d --no-deps internal-proxy gateway >/dev/null 2>&1 || {
        echo "$label could not start the internal proxy and Gateway." >&2
        rc=1
    }
    _oteryn_reconcile_marketplace_scheduler_after_runtime_change >/dev/null 2>&1 || {
        echo "$label could not reconcile the previous Marketplace runtime." >&2
        rc=1
    }
    return "$rc"
}

# Native (protocol_version 2) login needs the Gateway branch and the Platform issuer switched
# together. A Gateway that accepts the branch while the issuer cannot sign would pass deploy with
# every login answering NATIVE_LOGIN_UNAVAILABLE. A login probe with an unknown ticket is rejected
# before the issuer touches its signing key, so it cannot prove issuance works; only the Platform
# signing self-check (oteryn_run_native_admission_self_check) does, and deploy and health-check
# both run it whenever the flag is true.
oteryn_assert_native_login_pairing() {
    local platform_flag="$1"
    local gateway_flag="$2"

    case "$platform_flag" in
        true | false) ;;
        *)
            echo "Platform native admission flag is missing or invalid; the deployment must set it explicitly." >&2
            return 1
            ;;
    esac
    if [[ "$gateway_flag" != "$platform_flag" ]]; then
        echo "Gateway native login (${gateway_flag:-unset}) does not match Platform native admission ($platform_flag)." >&2
        return 1
    fi
}

# Creates the issuer signing key volume content when it does not exist yet and proves the key id and
# public key can be read back. Prints only key ids and PUBLIC keys (for Game's trust config), never
# the private key. Fails closed.
oteryn_ensure_native_admission_key() {
    local script_dir="${SCRIPT_DIR:-$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)}"
    local output

    if ! output="$(PLATFORM_IMAGE="$PLATFORM_IMAGE" bash "$script_dir/generate-native-admission-key.sh" ensure)"; then
        echo "Native admission signing key custody is not usable; refusing to deploy." >&2
        return 1
    fi
    if ! grep -Eq '^current_key_id=[A-Za-z0-9._-]{1,64}$' <<<"$output" \
        || ! grep -Eq '^current_public_key=[A-Za-z0-9_-]{43}$' <<<"$output"; then
        echo "Native admission signing key custody did not report a current key id and public key." >&2
        return 1
    fi
    echo "Native admission signing key present; publish these PUBLIC values to Game's trust config:"
    printf '%s\n' "$output"
}

# Runs the Platform signing self-check in the given running Platform container: signs a throwaway
# grant with the configured key and verifies it against the trust registry. Exit 0 only when the
# issuer can really sign. The command prints one JSON line with the key id or one reason line, never
# key material.
oteryn_run_native_admission_self_check() {
    local container_id="$1"
    local output

    if [[ -z "$container_id" ]]; then
        echo "Native admission self-check needs the running Platform container." >&2
        return 1
    fi
    if ! output="$(docker exec "$container_id" /usr/local/bin/oteryn-platform-entrypoint \
        php artisan game-auth:native-admission:self-check --no-interaction 2>&1)"; then
        echo "Native admission signing self-check failed; refusing to enable native game login: ${output}" >&2
        return 1
    fi
    echo "Native admission signing self-check passed: ${output}"

    # The deploy gate only proves the identity variables are non-empty. Prove the running Platform really
    # accepts them with the application's own settings logic (valid JSON, scope format, no overlap with
    # another purpose's identity): a null result means ingestion answers 503 and no scope routes.
    if ! docker exec "$container_id" /usr/local/bin/oteryn-platform-entrypoint php -r '
        require "vendor/autoload.php";
        $app = require "bootstrap/app.php";
        $app->make(Illuminate\Contracts\Console\Kernel::class)->bootstrap();
        exit(App\GameAuth\NativeRuntimeStatus\NativeRuntimeStatusSettings::current() !== null
            && App\GameAuth\NativeRuntimeStatus\NativeScopeAssignmentSettings::current() !== null ? 0 : 1);
    ' >/dev/null 2>&1; then
        echo "Native admission projection identities are invalid in the running Platform (runtime status or scope assignment settings are not usable); refusing to enable native game login." >&2
        return 1
    fi
}

# Validates the projection settings native login needs besides a signing key: the native World UUIDv7
# for the D171 fallback and the Game runtime-status and scope-assignment identities that make a scope
# routable. Reads the loaded environment; prints the reason to stderr and fails when incomplete.
oteryn_native_admission_projection_ready() {
    if [[ ! "${GAME_AUTH_NATIVE_ADMISSION_UNVERIFIED_CHARACTER_WORLD_ID:-}" =~ ^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$ ]]; then
        echo "Native admission needs OTERYN_STAGING_NATIVE_WORLD_ID (the native World UUIDv7; set the repository variable or the value in deploy/synology/native-defaults.env)." >&2
        return 1
    fi
    if [[ "${GAME_AUTH_NATIVE_RUNTIME_STATUS_ENABLED:-}" != true || -z "${GAME_AUTH_NATIVE_RUNTIME_STATUS_IDENTITIES:-}" \
        || "${GAME_AUTH_NATIVE_SCOPE_ASSIGNMENT_ENABLED:-}" != true || -z "${GAME_AUTH_NATIVE_SCOPE_ASSIGNMENT_IDENTITIES:-}" ]]; then
        echo "Native admission needs OTERYN_STAGING_NATIVE_RUNTIME_STATUS_IDENTITIES and OTERYN_STAGING_NATIVE_SCOPE_ASSIGNMENT_IDENTITIES (repository variables or deploy/synology/native-defaults.env); without them no scope is routable and every login would fail." >&2
        return 1
    fi
}

# Prints the native admission flag a run of the given Platform image may use: true only when the
# environment asks for native admission AND the image can both run the self-check command and export
# the signing keys from the key volume (entrypoint load_native_admission_keys). An image that has the
# command but predates the key export would fail the self-check, so rollback and previous-candidate
# recovery of such a release run with native login off.
oteryn_native_admission_flag_for_image() {
    local image="$1"

    if [[ "${OTERYN_NATIVE_ADMISSION_ENABLED:-false}" == true ]] \
        && oteryn_native_admission_projection_ready 2>/dev/null \
        && docker run --rm --network none --entrypoint grep "$image" -q load_native_admission_keys \
            /usr/local/bin/oteryn-platform-entrypoint 2>/dev/null \
        && docker run --rm --network none --entrypoint php "$image" artisan list --raw 2>/dev/null \
            | awk '/^game-auth:native-admission:self-check /{a=1} /^game-auth:native-admission:publish-trust /{b=1} END{exit !(a&&b)}'; then
        echo true
    else
        echo false
    fi
}

# Pins the compatibility flag for an image into a bounded runtime env file, so every Compose command
# that reads that file (including the Marketplace reconciler, which builds its own command from
# OTERYN_ENV_FILE) recreates Platform and Gateway with the same flag.
oteryn_pin_native_admission_flag() {
    local env_file="$1" image="$2" flag
    flag="$(oteryn_native_admission_flag_for_image "$image")"
    { grep -v '^OTERYN_NATIVE_ADMISSION_ENABLED=' "$env_file" || true; printf 'OTERYN_NATIVE_ADMISSION_ENABLED=%s\n' "$flag"; } >"$env_file.pin"
    chmod 600 "$env_file.pin"
    mv "$env_file.pin" "$env_file"
}

trap _oteryn_finalize_release_on_exit EXIT
