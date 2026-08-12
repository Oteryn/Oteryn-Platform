#!/usr/bin/env bash

# Docker Hub index digests verified 2026-08-12. Health probes keep their exact
# historical behavior while the Docker boundary substitutes immutable identities.
OTERYN_HEALTH_ALPINE_IMAGE='alpine@sha256:14358309a308569c32bdc37e2e0e9694be33a9d99e68afb0f5ff33cc1f695dce'
OTERYN_HEALTH_PYTHON_IMAGE='python@sha256:6d43704baacd1bfbe7c295d7f13079d5d8104ed33568873133f8fc69980419df'

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
}

_oteryn_deploy_state_dir() { printf '%s\n' "${OTERYN_STATE_DIR:-/var/lib/oteryn-staging-state}"; }

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
        echo "Platform/Gateway OCI application revisions disagree; refusing deployment." >&2
        return 1
    }
    printf '%s\n' "$platform_revision"
}

_oteryn_release_sha() {
    local explicit_sha="${OTERYN_RELEASE_SHA:-}" revision
    revision="$(_oteryn_release_sha_for_images "$PLATFORM_IMAGE" "$GATEWAY_IMAGE")"
    if [[ -n "$explicit_sha" && "$explicit_sha" != "$revision" ]]; then
        echo "OTERYN_RELEASE_SHA disagrees with runtime OCI application revision." >&2
        return 1
    fi
    if [[ "${GATEWAY_VERSION:-}" =~ ^sha-([0-9a-f]{40})$ && "${BASH_REMATCH[1]}" != "$revision" ]]; then
        echo "GATEWAY_VERSION disagrees with runtime OCI application revision." >&2
        return 1
    fi
    printf '%s\n' "$revision"
}

_oteryn_contract_from_platform_image() {
    local platform_image="$1" payload line key value
    local policy='' schema='' accepts=''
    payload="$(command docker run --rm --entrypoint cat "$platform_image" /var/www/html/deploy/synology/release-contract.env 2>/dev/null)" || {
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
    IFS=$'\t' read -r OTERYN_MIGRATION_POLICY OTERYN_SCHEMA_COMPATIBILITY_ID OTERYN_APP_ACCEPTS_SCHEMA_IDS \
        < <(_oteryn_contract_from_platform_image "$PLATFORM_IMAGE")
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

_oteryn_bootstrap_legacy_current_release() {
    local state_dir="$1" current_file="$state_dir/current-release.env" legacy_file="$state_dir/last-good.env"
    local old_platform old_gateway old_canary old_sha observed_schema table_count
    [[ ! -f "$current_file" ]] || return 0

    if [[ ! -f "$legacy_file" ]]; then
        table_count="$(command docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE" exec -T \
            -e MYSQL_PWD="$MARIADB_ROOT_PASSWORD" mariadb mariadb -uroot -N -e \
            "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema='$PLATFORM_DB_NAME';")"
        [[ "$table_count" =~ ^[0-9]+$ ]] || { echo "Cannot determine whether Platform DB is fresh; refusing migration." >&2; return 1; }
        if (( table_count > 0 )); then
            echo "Existing Platform DB has no managed application baseline; refusing migration before backup-capable baseline is proven." >&2
            return 1
        fi
        return 0
    fi

    # last-good.env is generated locally by deploy.sh using printf %q from the
    # exact running containers before any pull can move mutable tags.
    unset PLATFORM_IMAGE GATEWAY_IMAGE CANARY_IMAGE
    # shellcheck disable=SC1090
    source "$legacy_file"
    old_platform="${PLATFORM_IMAGE:-}"
    old_gateway="${GATEWAY_IMAGE:-}"
    old_canary="${CANARY_IMAGE:-}"
    [[ -n "$old_platform" && -n "$old_gateway" && -n "$old_canary" ]] || {
        echo "Legacy running-release snapshot is incomplete; refusing migration." >&2
        return 1
    }
    old_sha="$(_oteryn_release_sha_for_images "$old_platform" "$old_gateway")"

    # This synthetic identity is not an unverified image contract. It names the
    # exact pre-migration DB state that the observed old application is running
    # against at bootstrap time. A backup of that exact state is created below
    # before any migration, so recovery can truthfully return to this identity.
    observed_schema="observed-${old_sha}"
    bash "$SCRIPT_DIR/release-state.sh" write "$current_file" "$old_sha" \
        "$observed_schema" "$observed_schema" "$old_platform" "$old_gateway" "$old_canary" 1
}

_oteryn_before_platform_migrate() {
    local state_dir release_sha backup_dir backup_file backup_meta current_file old_sha old_schema
    state_dir="$(_oteryn_deploy_state_dir)"
    _oteryn_load_candidate_contract
    release_sha="$(_oteryn_release_sha)"
    mkdir -p "$state_dir/backups"
    chmod 700 "$state_dir" "$state_dir/backups"

    _oteryn_bootstrap_legacy_current_release "$state_dir"

    bash "$SCRIPT_DIR/release-state.sh" write "$state_dir/candidate-release.env" "$release_sha" \
        "$OTERYN_SCHEMA_COMPATIBILITY_ID" "$OTERYN_APP_ACCEPTS_SCHEMA_IDS" \
        "$PLATFORM_IMAGE" "$GATEWAY_IMAGE" "$CANARY_IMAGE" 1

    current_file="$state_dir/current-release.env"
    if [[ -f "$current_file" ]]; then
        bash "$SCRIPT_DIR/release-state.sh" validate "$current_file"
        cp "$current_file" "$state_dir/last-good-release.env.tmp"
        chmod 600 "$state_dir/last-good-release.env.tmp"
        mv "$state_dir/last-good-release.env.tmp" "$state_dir/last-good-release.env"
        old_sha="$(_oteryn_read_state_key "$current_file" RELEASE_SHA)"
        old_schema="$(_oteryn_read_state_key "$current_file" SCHEMA_COMPATIBILITY_ID)"

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
    release_sha="$(_oteryn_release_sha)"
    {
        printf 'SCHEMA_STATE=known\n'
        printf 'SCHEMA_COMPATIBILITY_ID=%s\n' "$OTERYN_SCHEMA_COMPATIBILITY_ID"
        printf 'MIGRATION_TARGET_RELEASE_SHA=%s\n' "$release_sha"
    } >"$state_dir/schema-state.env.tmp"
    chmod 600 "$state_dir/schema-state.env.tmp"
    mv "$state_dir/schema-state.env.tmp" "$state_dir/schema-state.env"
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
    local i
    for i in "${!args[@]}"; do
        case "${args[$i]}" in
            alpine:3.22) args[$i]="$OTERYN_HEALTH_ALPINE_IMAGE" ;;
            python:3.12-alpine) args[$i]="$OTERYN_HEALTH_PYTHON_IMAGE" ;;
        esac
    done

    if [[ "$(basename -- "${0:-}")" == "deploy.sh" ]]; then
        local joined=" ${args[*]} "
        if [[ "$joined" == *" exec -T platform php artisan migrate --force --no-interaction "* ]]; then
            _oteryn_before_platform_migrate
            command docker "${args[@]}"
            local rc=$?
            if [[ "$rc" -eq 0 ]]; then _oteryn_after_platform_migrate; fi
            return "$rc"
        fi
    fi
    command docker "${args[@]}"
}

trap _oteryn_finalize_release_on_exit EXIT
