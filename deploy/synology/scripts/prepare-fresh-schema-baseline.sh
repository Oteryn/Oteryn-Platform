#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd)"
ENV_FILE="${OTERYN_ENV_FILE:-$DEPLOY_DIR/.env}"
COMPOSE_FILE="$DEPLOY_DIR/compose.yml"

# shellcheck source=deploy/synology/scripts/lib.sh
source "$SCRIPT_DIR/lib.sh"
load_oteryn_env_file "$ENV_FILE"
# All five PostgreSQL secrets on the postgres engine (none or a partial set otherwise refused), before any change.
oteryn_require_postgres_secrets_for_engine || exit 1

state_dir="${OTERYN_STATE_DIR:-/var/lib/oteryn-staging-state}"
current_file="$state_dir/current-release.env"
legacy_file="$state_dir/last-good.env"
last_good_file="$state_dir/last-good-release.env"
candidate_file="$state_dir/candidate-release.env"

# A surviving candidate normally marks an incomplete prior transition. The only
# resumable case is the same exact immutable candidate after its migration has
# already been durably proven known. lib.sh re-validates the same boundary before
# Platform starts and preserves candidate recovery evidence until health passes.
if [[ -f "$candidate_file" ]]; then
    release_sha="$(_oteryn_release_sha)" || exit 1
    _oteryn_resume_candidate_if_safe "$state_dir" "$release_sha" || exit 1
    exit 0
fi

# A managed or provable legacy release already owns the pre-migration baseline.
if [[ -f "$current_file" || -f "$legacy_file" || -f "$last_good_file" ]]; then
    exit 0
fi

mkdir -p "$state_dir/backups"
chmod 700 "$state_dir" "$state_dir/backups"

# The Platform database must hold no relation outside the system schemas, counted by the PostgreSQL
# superuser (local peer, no password).
platform_db_compose=(command docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE")
platform_db_table_count() {
    oteryn_postgres_relation_count "${platform_db_compose[@]}"
}

table_count="$(platform_db_table_count)" || {
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
backup_file="$backup_dir/platform.dump"
evidence_file="$backup_dir/evidence.env"
mkdir -p "$backup_dir"
chmod 700 "$backup_dir"

# pg_dump -Fc by the read-only dump role over TLS, validated with pg_restore --list (lib.sh).
oteryn_postgres_dump_platform "${platform_db_compose[@]}" "$backup_file" || exit 1

# Re-prove emptiness after the dump so concurrent or unexpected writes cannot be
# promoted as a recoverable fresh baseline.
table_count_after="$(platform_db_table_count)"
[[ "$table_count_after" == 0 ]] || { echo "Fresh baseline changed while being captured; refusing migration." >&2; exit 1; }

{
    # A baseline without this key is MariaDB-era; recover-schema.sh refuses it.
    printf 'BACKUP_DB_ENGINE=postgres\n'
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
