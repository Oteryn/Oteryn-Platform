#!/usr/bin/env bash

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

    if [[ "$(basename -- "${0:-}")" == "deploy.sh" && -f "$DEPLOY_DIR/release-contract.env" ]]; then
        # shellcheck disable=SC1090
        source "$DEPLOY_DIR/release-contract.env"
        export OTERYN_MIGRATION_POLICY OTERYN_SCHEMA_COMPATIBILITY_ID OTERYN_APP_ACCEPTS_SCHEMA_IDS
        if [[ "$OTERYN_MIGRATION_POLICY" != "expand-contract" ]]; then
            echo "Synology staging migration policy must remain expand-contract." >&2
            return 1
        fi
    fi

    # Character Bazaar Staging Control loads both a partial durable state file
    # and the complete ephemeral deployment .env. Apply the public-origin
    # migration only to the complete .env; the state file intentionally carries
    # Marketplace keys only and must not be treated as deployment configuration.
    if [[ "${GITHUB_WORKFLOW:-}" == "Character Bazaar Staging Control" \
        && "$(basename -- "$env_file")" == ".env" ]]; then
        case "${APP_URL:-}" in
            https://oteryn.molehill.cloud|http://127.0.0.1:8000) ;;
            *)
                echo "Character Bazaar public staging APP_URL must be the canonical origin or the exact legacy loopback value." >&2
                return 1
                ;;
        esac
        case "${SESSION_SECURE_COOKIE:-}" in
            true|false|'') ;;
            *)
                echo "Character Bazaar public staging SESSION_SECURE_COOKIE must be boolean." >&2
                return 1
                ;;
        esac

        APP_URL=https://oteryn.molehill.cloud
        SESSION_SECURE_COOKIE=true
        export APP_URL SESSION_SECURE_COOKIE
    fi
}

_oteryn_deploy_state_dir() {
    printf '%s\n' "${OTERYN_STATE_DIR:-/var/lib/oteryn-staging-state}"
}

_oteryn_release_sha() {
    local sha="${OTERYN_RELEASE_SHA:-${GITHUB_SHA:-}}"
    if [[ -z "$sha" ]] && command -v git >/dev/null 2>&1; then
        sha="$(git -C "$REPO_ROOT" rev-parse HEAD 2>/dev/null || true)"
    fi
    [[ "$sha" =~ ^[0-9a-f]{40}$ ]] || {
        echo "Cannot prove exact application release SHA; refusing migration-bearing deployment." >&2
        return 1
    }
    printf '%s\n' "$sha"
}

_oteryn_before_platform_migrate() {
    local state_dir release_sha backup_dir backup_file backup_meta current_file
    state_dir="$(_oteryn_deploy_state_dir)"
    release_sha="$(_oteryn_release_sha)"
    mkdir -p "$state_dir/backups"
    chmod 700 "$state_dir" "$state_dir/backups"
    current_file="$state_dir/current-release.env"

    if [[ -f "$current_file" ]]; then
        bash "$SCRIPT_DIR/release-state.sh" validate "$current_file"
        cp "$current_file" "$state_dir/last-good-release.env.tmp"
        chmod 600 "$state_dir/last-good-release.env.tmp"
        mv "$state_dir/last-good-release.env.tmp" "$state_dir/last-good-release.env"

        # Backup is deliberately taken before migrate and tied to both sides of
        # the transition. It is recovery evidence, not an automatic rollback.
        # shellcheck disable=SC1090
        source "$current_file"
        backup_dir="$state_dir/backups/${RELEASE_SHA}-before-${release_sha}"
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
            printf 'BACKUP_FROM_RELEASE_SHA=%s\n' "$RELEASE_SHA"
            printf 'BACKUP_BEFORE_RELEASE_SHA=%s\n' "$release_sha"
            printf 'BACKUP_SCHEMA_COMPATIBILITY_ID=%s\n' "$SCHEMA_COMPATIBILITY_ID"
            printf 'BACKUP_SHA256=%s\n' "$(sha256sum "$backup_file" | awk '{print $1}')"
        } >"$backup_meta.tmp"
        chmod 600 "$backup_meta.tmp"
        mv "$backup_meta.tmp" "$backup_meta"
    fi

    # Unknown is persisted before invoking migrate so a crash, timeout or
    # transport loss cannot accidentally authorize image rollback.
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
    local rc="$?" state_dir release_sha
    [[ "$rc" -eq 0 ]] || return "$rc"
    [[ "$(basename -- "${0:-}")" == "deploy.sh" ]] || return 0
    state_dir="$(_oteryn_deploy_state_dir)"
    release_sha="$(_oteryn_release_sha)"
    bash "$SCRIPT_DIR/release-state.sh" write "$state_dir/current-release.env" "$release_sha" \
        "$OTERYN_SCHEMA_COMPATIBILITY_ID" "$OTERYN_APP_ACCEPTS_SCHEMA_IDS" \
        "$PLATFORM_IMAGE" "$GATEWAY_IMAGE" "$CANARY_IMAGE" 1
    return 0
}

# All current health probes still execute unchanged, but mutable historical
# aliases are translated at the single Docker invocation boundary to repository-
# pinned immutable identities. Regression tests forbid any unhandled helper tag.
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
            if [[ "$rc" -eq 0 ]]; then
                _oteryn_after_platform_migrate
            fi
            return "$rc"
        fi
    fi
    command docker "${args[@]}"
}

trap _oteryn_finalize_release_on_exit EXIT
