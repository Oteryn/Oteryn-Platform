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
current_file="$state_dir/current-release.env"
legacy_file="$state_dir/last-good.env"
last_good_file="$state_dir/last-good-release.env"
candidate_file="$state_dir/candidate-release.env"

# A surviving candidate marks an incomplete prior transition. Refuse before the
# lib.sh migration hook can rewrite candidate metadata or reuse its backup path.
if [[ -f "$candidate_file" ]]; then
    bash "$SCRIPT_DIR/release-state.sh" validate "$candidate_file"
    candidate_sha="$(_oteryn_read_state_key "$candidate_file" RELEASE_SHA)"
    echo "Deployment rejected: unresolved candidate release $candidate_sha still owns recovery evidence." >&2
    exit 1
fi

# A managed or provable legacy release already owns the pre-migration baseline.
if [[ -f "$current_file" || -f "$legacy_file" || -f "$last_good_file" ]]; then
    exit 0
fi

mkdir -p "$state_dir/backups"
chmod 700 "$state_dir" "$state_dir/backups"

table_count="$(command docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE" exec -T \
    -e MYSQL_PWD="$MARIADB_ROOT_PASSWORD" mariadb mariadb -uroot -N -e \
    "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema='$PLATFORM_DB_NAME';")" || {
    echo "Cannot prove the fresh Platform DB baseline; refusing migration." >&2
    exit 1
}
[[ "$table_count" =~ ^[0-9]+$ ]] || { echo "Cannot prove the fresh Platform DB baseline; refusing migration." >&2; exit 1; }
if (( table_count != 0 )); then
    echo "Fresh baseline rejected: Platform DB is non-empty and no managed application baseline exists." >&2
    exit 1
fi

# No Platform DB consumer may exist while the empty baseline is captured.
for service in platform gateway internal-proxy; do
    container_id="$(command docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE" ps -q "$service" 2>/dev/null || true)"
    [[ -z "$container_id" ]] || { echo "Fresh baseline rejected: unexpected running application service exists: $service" >&2; exit 1; }
done
scheduler_id="$(_oteryn_marketplace_scheduler_id)"
[[ -z "$scheduler_id" ]] || { echo "Fresh baseline rejected: Marketplace scheduler exists without a managed application baseline." >&2; exit 1; }

release_sha="$(_oteryn_release_sha)" || exit 1
backup_dir="$state_dir/backups/fresh-empty-before-${release_sha}"
backup_file="$backup_dir/platform.sql"
evidence_file="$backup_dir/evidence.env"
mkdir -p "$backup_dir"
chmod 700 "$backup_dir"

command docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE" exec -T \
    -e MYSQL_PWD="$MARIADB_ROOT_PASSWORD" mariadb \
    mariadb-dump -uroot --single-transaction --routines --triggers --events "$PLATFORM_DB_NAME" >"$backup_file.tmp"
chmod 600 "$backup_file.tmp"
mv "$backup_file.tmp" "$backup_file"

# Re-prove emptiness after the dump so concurrent or unexpected writes cannot be
# promoted as a recoverable fresh baseline.
table_count_after="$(command docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE" exec -T \
    -e MYSQL_PWD="$MARIADB_ROOT_PASSWORD" mariadb mariadb -uroot -N -e \
    "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema='$PLATFORM_DB_NAME';")"
[[ "$table_count_after" == 0 ]] || { echo "Fresh baseline changed while being captured; refusing migration." >&2; exit 1; }

{
    printf 'BACKUP_BASELINE_KIND=fresh-empty\n'
    printf 'BACKUP_BEFORE_RELEASE_SHA=%s\n' "$release_sha"
    printf 'BACKUP_SCHEMA_COMPATIBILITY_ID=fresh-empty\n'
    printf 'BACKUP_COMPOSE_PROJECT_NAME=%s\n' "${COMPOSE_PROJECT_NAME:-oteryn-staging}"
    printf 'BACKUP_PLATFORM_DB_NAME=%s\n' "$PLATFORM_DB_NAME"
    printf 'BACKUP_SHA256=%s\n' "$(sha256sum "$backup_file" | awk '{print $1}')"
} >"$evidence_file.tmp"
chmod 600 "$evidence_file.tmp"
mv "$evidence_file.tmp" "$evidence_file"

_oteryn_write_schema_state_known "$state_dir" fresh-empty "$release_sha"
echo "Verified fresh empty Platform DB recovery baseline captured before first managed migration."
