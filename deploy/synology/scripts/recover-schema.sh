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

if [[ $# -ne 1 ]]; then
    echo "usage: $0 BACKUP_EVIDENCE_ENV" >&2
    exit 2
fi

evidence_file="$1"
[[ -f "$evidence_file" ]] || { echo "Recovery evidence does not exist: $evidence_file" >&2; exit 1; }
case "$(cd -- "$(dirname -- "$evidence_file")" && pwd)/$(basename -- "$evidence_file")" in
    "$state_dir"/backups/*/evidence.env) ;;
    *) echo "Recovery evidence must be a managed staging backup under $state_dir/backups." >&2; exit 1 ;;
esac

# Evidence is data, never shell input. Parse the exact generated allowlist so a
# malformed file cannot override PLATFORM_DB_NAME or any other operational value.
unset BACKUP_FROM_RELEASE_SHA BACKUP_BEFORE_RELEASE_SHA BACKUP_SCHEMA_COMPATIBILITY_ID
unset BACKUP_COMPOSE_PROJECT_NAME BACKUP_PLATFORM_DB_NAME BACKUP_SHA256
seen='|'
while IFS= read -r line || [[ -n "$line" ]]; do
    line="${line%$'\r'}"
    [[ -z "$line" ]] && continue
    [[ "$line" == *=* ]] || { echo "Recovery evidence contains an invalid line." >&2; exit 1; }
    key="${line%%=*}"
    value="${line#*=}"
    case "$key" in
        BACKUP_FROM_RELEASE_SHA|BACKUP_BEFORE_RELEASE_SHA|BACKUP_SCHEMA_COMPATIBILITY_ID|BACKUP_COMPOSE_PROJECT_NAME|BACKUP_PLATFORM_DB_NAME|BACKUP_SHA256) ;;
        *) echo "Recovery evidence contains unexpected key: $key" >&2; exit 1 ;;
    esac
    if [[ "$seen" == *"|$key|"* ]]; then
        echo "Recovery evidence contains duplicate key: $key" >&2
        exit 1
    fi
    seen="${seen}${key}|"
    printf -v "$key" '%s' "$value"
done < "$evidence_file"

for name in \
    BACKUP_FROM_RELEASE_SHA BACKUP_BEFORE_RELEASE_SHA BACKUP_SCHEMA_COMPATIBILITY_ID \
    BACKUP_COMPOSE_PROJECT_NAME BACKUP_PLATFORM_DB_NAME BACKUP_SHA256; do
    [[ -n "${!name:-}" ]] || { echo "Recovery evidence is incomplete: $name" >&2; exit 1; }
done
[[ "$BACKUP_FROM_RELEASE_SHA" =~ ^[0-9a-f]{40}$ ]] || { echo "Invalid backup source release SHA" >&2; exit 1; }
[[ "$BACKUP_BEFORE_RELEASE_SHA" =~ ^[0-9a-f]{40}$ ]] || { echo "Invalid backup target release SHA" >&2; exit 1; }
[[ "$BACKUP_SCHEMA_COMPATIBILITY_ID" =~ ^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$ ]] || { echo "Invalid backup schema compatibility identity" >&2; exit 1; }
[[ "$BACKUP_COMPOSE_PROJECT_NAME" =~ ^[A-Za-z0-9][A-Za-z0-9_.-]{0,127}$ ]] || { echo "Invalid backup Compose project identity" >&2; exit 1; }
[[ "$BACKUP_PLATFORM_DB_NAME" =~ ^[A-Za-z0-9_][A-Za-z0-9_.-]{0,63}$ ]] || { echo "Invalid backup Platform database identity" >&2; exit 1; }
[[ "$BACKUP_SHA256" =~ ^[0-9a-f]{64}$ ]] || { echo "Invalid backup digest" >&2; exit 1; }

effective_compose_project="${COMPOSE_PROJECT_NAME:-oteryn-staging}"
[[ "$BACKUP_COMPOSE_PROJECT_NAME" == "$effective_compose_project" ]] || {
    echo "Recovery rejected: backup Compose project does not match the configured staging target." >&2
    exit 1
}
[[ "$BACKUP_PLATFORM_DB_NAME" == "$PLATFORM_DB_NAME" ]] || {
    echo "Recovery rejected: backup database does not match the configured Platform database target." >&2
    exit 1
}

backup_file="$(dirname -- "$evidence_file")/platform.sql"
[[ -s "$backup_file" ]] || { echo "Recovery backup is missing or empty." >&2; exit 1; }
actual_sha="$(sha256sum "$backup_file" | awk '{print $1}')"
[[ "$actual_sha" == "$BACKUP_SHA256" ]] || { echo "Recovery backup digest mismatch." >&2; exit 1; }

candidate_file="$state_dir/candidate-release.env"
last_good_file="$state_dir/last-good-release.env"
schema_file="$state_dir/schema-state.env"
for file in "$candidate_file" "$last_good_file" "$schema_file"; do
    [[ -f "$file" ]] || { echo "Recovery rejected: required state is missing: $file" >&2; exit 1; }
done

bash "$SCRIPT_DIR/release-state.sh" validate "$candidate_file"
bash "$SCRIPT_DIR/release-state.sh" validate "$last_good_file"
candidate_sha="$(_oteryn_read_state_key "$candidate_file" RELEASE_SHA)"
last_good_sha="$(_oteryn_read_state_key "$last_good_file" RELEASE_SHA)"
[[ "$candidate_sha" == "$BACKUP_BEFORE_RELEASE_SHA" ]] || { echo "Recovery rejected: backup target does not match candidate release." >&2; exit 1; }
[[ "$last_good_sha" == "$BACKUP_FROM_RELEASE_SHA" ]] || { echo "Recovery rejected: backup source does not match last-good release." >&2; exit 1; }

# The backup records the schema that actually existed at dump time. The last-good
# application's primary schema identity may differ after a compatible image-only
# rollback, so prove acceptance rather than asserting equality with its primary ID.
bash "$SCRIPT_DIR/release-state.sh" compatible-schema \
    "$BACKUP_SCHEMA_COMPATIBILITY_ID" "$last_good_file" "$candidate_sha"

schema_target="$(_oteryn_read_state_key "$schema_file" MIGRATION_TARGET_RELEASE_SHA)"
[[ "$schema_target" == "$BACKUP_BEFORE_RELEASE_SHA" ]] || { echo "Recovery rejected: schema state is not tied to this failed candidate." >&2; exit 1; }

compose=(docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE")

# Persist uncertainty only after all immutable evidence, release identity and
# destructive target checks have passed.
{
    printf 'SCHEMA_STATE=unknown\n'
    printf 'MIGRATION_TARGET_RELEASE_SHA=%s\n' "$BACKUP_BEFORE_RELEASE_SHA"
} >"$schema_file.tmp"
chmod 600 "$schema_file.tmp"
mv "$schema_file.tmp" "$schema_file"

# Stop every known Platform DB consumer before recreation/import. Discover the
# optional scheduler by Compose ownership labels so no partial env interpolation
# can alter its configuration merely to stop it.
marketplace_scheduler_id="$(_oteryn_marketplace_scheduler_id)"
if [[ -n "$marketplace_scheduler_id" ]]; then
    command docker stop "$marketplace_scheduler_id" >/dev/null
    marketplace_scheduler_running="$(command docker inspect --format '{{.State.Running}}' "$marketplace_scheduler_id")"
    [[ "$marketplace_scheduler_running" == false ]] || {
        echo "Recovery rejected: marketplace-scheduler is still running." >&2
        exit 1
    }
fi
"${compose[@]}" stop platform gateway internal-proxy

# This is an explicit destructive staging recovery. It never runs implicitly from
# deploy.sh or rollback.sh. The dump is restored into the staging Platform DB only.
"${compose[@]}" exec -T -e MYSQL_PWD="$MARIADB_ROOT_PASSWORD" mariadb \
    mariadb -uroot -e "DROP DATABASE IF EXISTS \`$PLATFORM_DB_NAME\`; CREATE DATABASE \`$PLATFORM_DB_NAME\` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;"
"${compose[@]}" exec -T -e MYSQL_PWD="$MARIADB_ROOT_PASSWORD" mariadb \
    mariadb -uroot "$PLATFORM_DB_NAME" <"$backup_file"

_oteryn_write_schema_state_known "$state_dir" "$BACKUP_SCHEMA_COMPATIBILITY_ID" "$BACKUP_FROM_RELEASE_SHA"

# Keep candidate-release.env until rollback.sh succeeds. It is the durable proof
# of which failed application transition the restored schema just recovered from.
echo "Staging Platform database restored from verified pre-migration backup. Runtime images were not changed; run rollback.sh separately after compatibility validation."
