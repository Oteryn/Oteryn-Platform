#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd)"
ENV_FILE="${OTERYN_ENV_FILE:-$DEPLOY_DIR/.env}"
COMPOSE_FILE="$DEPLOY_DIR/compose.yml"
MARKETPLACE_COMPOSE_FILE="$DEPLOY_DIR/compose.marketplace.yml"
state_dir="${OTERYN_STATE_DIR:-/var/lib/oteryn-staging-state}"

# shellcheck source=deploy/synology/scripts/lib.sh
source "$SCRIPT_DIR/lib.sh"
load_oteryn_env_file "$ENV_FILE"

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

# shellcheck disable=SC1090
source "$evidence_file"
for name in BACKUP_FROM_RELEASE_SHA BACKUP_BEFORE_RELEASE_SHA BACKUP_SCHEMA_COMPATIBILITY_ID BACKUP_SHA256; do
    [[ -n "${!name:-}" ]] || { echo "Recovery evidence is incomplete: $name" >&2; exit 1; }
done
[[ "$BACKUP_FROM_RELEASE_SHA" =~ ^[0-9a-f]{40}$ ]] || { echo "Invalid backup source release SHA" >&2; exit 1; }
[[ "$BACKUP_BEFORE_RELEASE_SHA" =~ ^[0-9a-f]{40}$ ]] || { echo "Invalid backup target release SHA" >&2; exit 1; }
[[ "$BACKUP_SHA256" =~ ^[0-9a-f]{64}$ ]] || { echo "Invalid backup digest" >&2; exit 1; }

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

# The evidence must describe exactly the transition that is currently unsafe.
# shellcheck disable=SC1090
source "$candidate_file"
[[ "$RELEASE_SHA" == "$BACKUP_BEFORE_RELEASE_SHA" ]] || { echo "Recovery rejected: backup target does not match candidate release." >&2; exit 1; }
# shellcheck disable=SC1090
source "$last_good_file"
[[ "$RELEASE_SHA" == "$BACKUP_FROM_RELEASE_SHA" ]] || { echo "Recovery rejected: backup source does not match last-good release." >&2; exit 1; }
[[ "$SCHEMA_COMPATIBILITY_ID" == "$BACKUP_SCHEMA_COMPATIBILITY_ID" ]] || { echo "Recovery rejected: backup schema identity does not match last-good release." >&2; exit 1; }
# shellcheck disable=SC1090
source "$schema_file"
[[ "${MIGRATION_TARGET_RELEASE_SHA:-}" == "$BACKUP_BEFORE_RELEASE_SHA" ]] || { echo "Recovery rejected: schema state is not tied to this failed candidate." >&2; exit 1; }

compose=(docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE")
marketplace_compose=(docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE" -f "$MARKETPLACE_COMPOSE_FILE")

# Persist uncertainty before any destructive database operation. Any failure
# from this point onward leaves rollback fail-closed until a complete restore
# has established and persisted a known schema identity.
{
    printf 'SCHEMA_STATE=unknown\n'
    printf 'MIGRATION_TARGET_RELEASE_SHA=%s\n' "$BACKUP_BEFORE_RELEASE_SHA"
} >"$schema_file.tmp"
chmod 600 "$schema_file.tmp"
mv "$schema_file.tmp" "$schema_file"

# Stop every known Platform DB consumer before recreation/import. Marketplace is
# optional, but if its scheduler container exists it must stop successfully.
marketplace_scheduler_id="$("${marketplace_compose[@]}" ps -a -q marketplace-scheduler 2>/dev/null || true)"
if [[ -n "$marketplace_scheduler_id" ]]; then
    "${marketplace_compose[@]}" stop marketplace-scheduler
    marketplace_scheduler_running="$(docker inspect --format '{{.State.Running}}' "$marketplace_scheduler_id")"
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

{
    printf 'SCHEMA_STATE=known\n'
    printf 'SCHEMA_COMPATIBILITY_ID=%s\n' "$BACKUP_SCHEMA_COMPATIBILITY_ID"
    printf 'MIGRATION_TARGET_RELEASE_SHA=%s\n' "$BACKUP_FROM_RELEASE_SHA"
} >"$schema_file.tmp"
chmod 600 "$schema_file.tmp"
mv "$schema_file.tmp" "$schema_file"

# Keep candidate-release.env until rollback.sh succeeds. It is the durable proof
# of which failed application transition the restored schema just recovered from.
echo "Staging Platform database restored from verified pre-migration backup. Runtime images were not changed; run rollback.sh separately after compatibility validation."
