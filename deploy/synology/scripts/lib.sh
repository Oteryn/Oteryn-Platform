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
    revision="$(_oteryn_release_sha_for_images "$PLATFORM_IMAGE" "$GATEWAY_IMAGE")" || return 1
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
    local old_platform old_gateway old_canary old_sha observed_schema table_count
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

    mapfile -t legacy_images < <(bash -c 'set -euo pipefail; source "$1"; printf "%s\n%s\n%s\n" "${PLATFORM_IMAGE:-}" "${GATEWAY_IMAGE:-}" "${CANARY_IMAGE:-}"' bash "$legacy_file")
    old_platform="${legacy_images[0]:-}"
    old_gateway="${legacy_images[1]:-}"
    old_canary="${legacy_images[2]:-}"
    [[ -n "$old_platform" && -n "$old_gateway" && -n "$old_canary" ]] || {
        echo "Legacy running-release snapshot is incomplete; refusing migration." >&2
        return 1
    }
    old_sha="$(_oteryn_release_sha_for_images "$old_platform" "$old_gateway")" || return 1

    observed_schema="observed-${old_sha}"
    bash "$SCRIPT_DIR/release-state.sh" write "$current_file" "$old_sha" \
        "$observed_schema" "$observed_schema" "$old_platform" "$old_gateway" "$old_canary" 1
    _oteryn_write_schema_state_known "$state_dir" "$observed_schema" "$old_sha"
}

_oteryn_marketplace_state_file() {
    printf '%s/marketplace.env\n' "$(_oteryn_deploy_state_dir)"
}

_oteryn_load_marketplace_runtime_state() {
    local durable_file source_file line key value seen='|'
    local -a required=(
        MARKETPLACE_ENABLED
        MARKETPLACE_ESCROW_CANARY_ACCOUNT_ID
        MARKETPLACE_ESCROW_CANARY_ACCOUNT_NAME
        MARKETPLACE_ESCROW_CANARY_ACCOUNT_CREATION_EPOCH
        CANARY_CHARACTER_TRANSFER_DB_USER
        CANARY_CHARACTER_TRANSFER_DB_PASSWORD
    )
    durable_file="$(_oteryn_marketplace_state_file)"
    source_file="$durable_file"
    if [[ -f "$ENV_FILE" ]] && grep -q '^MARKETPLACE_ENABLED=' "$ENV_FILE"; then
        source_file="$ENV_FILE"
    fi
    [[ -f "$source_file" ]] || return 2

    unset MARKETPLACE_ENABLED MARKETPLACE_ESCROW_CANARY_ACCOUNT_ID MARKETPLACE_ESCROW_CANARY_ACCOUNT_NAME
    unset MARKETPLACE_ESCROW_CANARY_ACCOUNT_CREATION_EPOCH CANARY_CHARACTER_TRANSFER_DB_USER CANARY_CHARACTER_TRANSFER_DB_PASSWORD

    while IFS= read -r line || [[ -n "$line" ]]; do
        line="${line%$'\r'}"
        [[ -z "$line" || "$line" =~ ^[[:space:]]*# ]] && continue
        [[ "$line" == *=* ]] || { echo "Invalid Marketplace state line." >&2; return 1; }
        key="${line%%=*}"
        value="${line#*=}"
        case "$key" in
            MARKETPLACE_ENABLED|MARKETPLACE_ESCROW_CANARY_ACCOUNT_ID|MARKETPLACE_ESCROW_CANARY_ACCOUNT_NAME|MARKETPLACE_ESCROW_CANARY_ACCOUNT_CREATION_EPOCH|CANARY_CHARACTER_TRANSFER_DB_USER|CANARY_CHARACTER_TRANSFER_DB_PASSWORD)
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
    if [[ "$MARKETPLACE_ENABLED" == true ]]; then
        [[ "$MARKETPLACE_ESCROW_CANARY_ACCOUNT_ID" =~ ^[1-9][0-9]*$ ]] || { echo "Enabled Marketplace runtime state has invalid escrow id." >&2; return 1; }
        [[ -n "$CANARY_CHARACTER_TRANSFER_DB_USER" && -n "$CANARY_CHARACTER_TRANSFER_DB_PASSWORD" ]] || { echo "Enabled Marketplace runtime state lacks transfer credentials." >&2; return 1; }
    fi
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

_oteryn_recreate_marketplace_scheduler() {
    local marketplace_file="$DEPLOY_DIR/compose.marketplace.yml" scheduler_id scheduler_running scheduler_image
    local -a marketplace_compose=(command docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE" -f "$marketplace_file")
    [[ -f "$marketplace_file" ]] || { echo "Marketplace Compose file is missing." >&2; return 1; }
    _oteryn_load_marketplace_runtime_state || { echo "Cannot prove Marketplace runtime settings for scheduler recreation." >&2; return 1; }
    [[ "$MARKETPLACE_ENABLED" == true ]] || { echo "Refusing to recreate Marketplace scheduler while durable/effective state is disabled." >&2; return 1; }
    "${marketplace_compose[@]}" up -d --force-recreate marketplace-scheduler
    scheduler_id="$(_oteryn_marketplace_scheduler_id)" || return 1
    [[ -n "$scheduler_id" ]] || { echo "Marketplace scheduler was not recreated." >&2; return 1; }
    scheduler_running="$(command docker inspect --format '{{.State.Running}}' "$scheduler_id")" || return 1
    scheduler_image="$(command docker inspect --format '{{.Config.Image}}' "$scheduler_id")" || return 1
    [[ "$scheduler_running" == true ]] || { echo "Marketplace scheduler is not running after recreation." >&2; return 1; }
    [[ "$scheduler_image" == "$PLATFORM_IMAGE" ]] || { echo "Marketplace scheduler image does not match the selected Platform runtime image." >&2; return 1; }
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

_oteryn_restore_quiesced_consumers_after_migrate() {
    if [[ "${OTERYN_MARKETPLACE_SCHEDULER_WAS_RUNNING:-0}" == 1 ]]; then
        _oteryn_recreate_marketplace_scheduler || return 1
        OTERYN_MARKETPLACE_SCHEDULER_WAS_RUNNING=0
        export OTERYN_MARKETPLACE_SCHEDULER_WAS_RUNNING
    fi
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

_oteryn_before_platform_migrate() {
    local state_dir release_sha backup_dir backup_file backup_meta current_file old_sha old_schema
    state_dir="$(_oteryn_deploy_state_dir)"
    _oteryn_load_candidate_contract || return 1
    release_sha="$(_oteryn_release_sha)" || return 1
    mkdir -p "$state_dir/backups"
    chmod 700 "$state_dir" "$state_dir/backups"

    _oteryn_bootstrap_legacy_current_release "$state_dir" || return 1

    bash "$SCRIPT_DIR/release-state.sh" write "$state_dir/candidate-release.env" "$release_sha" \
        "$OTERYN_SCHEMA_COMPATIBILITY_ID" "$OTERYN_APP_ACCEPTS_SCHEMA_IDS" \
        "$PLATFORM_IMAGE" "$GATEWAY_IMAGE" "$CANARY_IMAGE" 1

    current_file="$state_dir/current-release.env"
    if [[ -f "$current_file" ]]; then
        bash "$SCRIPT_DIR/release-state.sh" validate "$current_file"
        cp "$current_file" "$state_dir/last-good-release.env.tmp"
        chmod 600 "$state_dir/last-good-release.env.tmp"
        mv "$state_dir/last-good-release.env.tmp" "$state_dir/last-good-release.env"
        old_sha="$(_oteryn_read_state_key "$current_file" RELEASE_SHA)" || return 1
        old_schema="$(_oteryn_known_schema_identity "$state_dir")" || return 1

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

trap _oteryn_finalize_release_on_exit EXIT