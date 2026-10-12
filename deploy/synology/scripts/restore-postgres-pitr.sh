#!/usr/bin/env bash
# Platform PostgreSQL point-in-time restore drill (docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md P5,
# runbook docs/operations/PLATFORM_POSTGRES_BACKUP_PITR.md). Restores a base backup from the NAS spool plus
# archived WAL into a scratch container with its own anonymous data volume and no network, verifies it and
# discards it. The live server, its volumes and the spool are only read (plus the marker rows below).
#
#   restore-postgres-pitr.sh                      probe drill (default): writes marker A into table
#                                                 oteryn_pitr_drill of the `postgres` maintenance database,
#                                                 records target time T and the row count of every table of
#                                                 oteryn_platform in one snapshot, writes marker B after T,
#                                                 forces the WAL segment into the archive, then restores to T
#   restore-postgres-pitr.sh --target-time 'YYYY-MM-DD HH:MM:SS[.ffffff]'   restore to that UTC time
#                                                 (in the past; marker C is written and archived after it,
#                                                 because recovery stops only at a commit past the target)
#   restore-postgres-pitr.sh --target immediate --backup <stamp>           a (weekly) base backup alone
#   Options: --skip-platform-checks (no Platform image on this host; never in the staging drill)
#            --evidence-dir DIR (default $OTERYN_STATE_DIR/pitr-drills when OTERYN_STATE_DIR is set)
#
# Checks on the restored instance: the backup's SHA256SUMS and pg_verifybackup (manifest checksums and
# WAL), promotion at the target, markers (A present, B absent), row counts equal to the reference
# snapshot, the wallet ledger invariant, the migrations head, then with the running Platform image
# `migrate:status` (nothing pending) and `production:verify-configuration`. It reports the restore time
# against RTO 4h and the data-loss bound against RPO 24h. Containers are found by Compose labels of
# COMPOSE_PROJECT_NAME (default oteryn-staging). Nothing here removes a named volume: the scratch
# container runs with --rm, which removes it and its anonymous data volume when it is stopped.
set -euo pipefail
export LC_ALL=C

PROJECT="${COMPOSE_PROJECT_NAME:-oteryn-staging}"
SPOOL=/var/lib/oteryn-postgres-backup
DRILL=/tmp/oteryn-drill
DATA=/var/lib/postgresql/data
RTO_SECONDS=14400
RPO_SECONDS=86400
ARCHIVE_WAIT_SECONDS="${OTERYN_ARCHIVE_WAIT_SECONDS:-180}"
# How long the scratch instance waits for a running backup job before it takes the spool lock.
LOCK_WAIT_SECONDS="${OTERYN_DRILL_LOCK_WAIT_SECONDS:-7200}"
LOCKED_MARKER=/tmp/oteryn-drill.locked
# Room for WAL replay beyond the restored cluster on the scratch data volume.
WAL_MARGIN_MB="${OTERYN_DRILL_WAL_MARGIN_MB:-1024}"
DRILL_APP_URL="${OTERYN_DRILL_APP_URL:-https://oteryn.molehill.cloud}"
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
RESTORE_WAL="$SCRIPT_DIR/../postgres/backup/restore-wal.sh"

target=probe
target_time=''
backup_stamp=''
platform_checks=true
evidence_dir="${OTERYN_STATE_DIR:+$OTERYN_STATE_DIR/pitr-drills}"

usage() {
    echo "usage: restore-postgres-pitr.sh [--target-time 'YYYY-MM-DD HH:MM:SS[.ffffff]' | --target immediate --backup STAMP]" >&2
    echo "                                [--skip-platform-checks] [--evidence-dir DIR]" >&2
    exit 2
}

while (( $# )); do
    case "$1" in
        --target-time) [[ "$target" == probe ]] || usage; target="time"; target_time="${2:-}"; (( $# >= 2 )) || usage; shift 2 ;;
        --target) [[ "${2:-}" == immediate && "$target" == probe ]] || usage; target=immediate; shift 2 ;;
        --backup) backup_stamp="${2:-}"; (( $# >= 2 )) || usage; shift 2 ;;
        --skip-platform-checks) platform_checks=false; shift ;;
        --evidence-dir) evidence_dir="${2:-}"; [[ -n "$evidence_dir" ]] || usage; shift 2 ;;
        *) usage ;;
    esac
done
if [[ "$target" == time ]]; then
    [[ "$target_time" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}\ [0-9]{2}:[0-9]{2}:[0-9]{2}(\.[0-9]{1,6})?$ ]] || usage
fi
if [[ "$target" == immediate ]]; then
    [[ "$backup_stamp" =~ ^[0-9]{8}T[0-9]{6}Z$ ]] || usage
elif [[ -n "$backup_stamp" ]]; then
    usage
fi
[[ -f "$RESTORE_WAL" ]] || { echo "Missing $RESTORE_WAL" >&2; exit 1; }
# The project name reaches Docker label filters: Compose project names only.
[[ "$PROJECT" =~ ^[a-z0-9][a-z0-9_-]{0,62}$ ]] || { echo "COMPOSE_PROJECT_NAME must be a Compose project name" >&2; exit 2; }
[[ "$WAL_MARGIN_MB" =~ ^[0-9]{1,7}$ && "$ARCHIVE_WAIT_SECONDS" =~ ^[1-9][0-9]{0,5}$ && "$LOCK_WAIT_SECONDS" =~ ^[1-9][0-9]{0,5}$ ]] \
    || usage
WAL_MARGIN_MB=$((10#$WAL_MARGIN_MB))

started_epoch="$(date +%s)"
stamp="$(date -u +%Y%m%dT%H%M%SZ)"
scratch="oteryn-pitr-drill-$stamp"
scratch_volume=''
scratch_created=false
declare -A evidence=()
evidence_order=()

log() { printf '[pitr-drill] %s\n' "$*"; }

# PID 1 of the scratch container. It holds a shared lock on the spool's job lock (backup-job.sh take_lock)
# for the life of the container, so no base backup, dump or retention run of the sidecar changes the spool
# while the drill chooses a base backup, replays its WAL and checks the newest dump. A backup job waits for
# the drill (at most its own lock timeout). Arguments: spool, lock wait seconds, marker file.
# Exit 3: the spool has no job lock (no backup job ever ran); exit 4: a job held the lock too long.
# shellcheck disable=SC2016 # expanded by the scratch container's bash
# The trap is set before the marker, and the wait runs in short steps: a TERM that arrives just before a
# `wait` is handled when that step ends, never lost behind an endless sleep.
SCRATCH_MAIN='exec 9< "$1/status/job.lock" || exit 3
flock -s -w "$2" 9 || exit 4
trap "exit 0" TERM
: > "$3"
while :; do
    sleep 5 9<&- &
    wait $!
done'

record() {
    [[ -n "${evidence[$1]+set}" ]] || evidence_order+=("$1")
    evidence[$1]="$2"
}
fail() {
    record result failed
    record failure "$*"
    echo "PITR drill failed: $*" >&2
    exit 1
}

cleanup() {
    local status=$? deadline
    if [[ "$scratch_created" == true ]]; then
        if (( status != 0 )); then
            docker exec "$scratch" tail -n 40 "$DRILL/server.log" >&2 2>/dev/null || true
        fi
        docker stop -t 30 "$scratch" > /dev/null 2>&1 || true
        deadline=$((SECONDS + 60))
        while docker container inspect "$scratch" > /dev/null 2>&1 && (( SECONDS < deadline )); do
            sleep 1
        done
        if docker container inspect "$scratch" > /dev/null 2>&1; then
            echo "Scratch container $scratch is still present; remove it by hand (docker stop $scratch)." >&2
            status=1
        elif [[ -n "$scratch_volume" ]] && docker volume inspect "$scratch_volume" > /dev/null 2>&1; then
            echo "Scratch data volume $scratch_volume is still present (anonymous volume of the drill)." >&2
            status=1
        else
            log "scratch instance discarded"
        fi
    fi
    write_evidence || true
    exit "$status"
}
trap cleanup EXIT
trap 'exit 143' TERM INT

write_evidence() {
    local key file summary=''
    (( ${#evidence_order[@]} )) || return 0
    for key in "${evidence_order[@]}"; do
        summary+="$(printf '%s=%s' "$(tr '[:lower:]' '[:upper:]' <<< "$key")" "${evidence[$key]//$'\n'/ }")"$'\n'
    done
    printf '%s' "$summary"
    if [[ -n "$evidence_dir" ]]; then
        mkdir -p -- "$evidence_dir/$stamp"
        file="$evidence_dir/$stamp/evidence.env"
        (umask 077; printf '%s' "$summary" > "$file")
        log "evidence written to $file"
    fi
    if [[ -n "${GITHUB_STEP_SUMMARY:-}" ]]; then
        {
            echo "### Platform PostgreSQL PITR drill $stamp"
            echo
            echo '| Key | Value |'
            echo '| --- | --- |'
            for key in "${evidence_order[@]}"; do
                printf '| %s | %s |\n' "$key" "${evidence[$key]//|//}"
            done
        } >> "$GITHUB_STEP_SUMMARY"
    fi
}

find_service() {
    local ids
    ids="$(docker ps -q --filter "label=com.docker.compose.project=$PROJECT" --filter "label=com.docker.compose.service=$1")"
    [[ "$(grep -c . <<< "$ids" || true)" -le 1 ]] || fail "more than one running $1 container in project $PROJECT"
    printf '%s' "$ids"
}

live_sql() {
    docker exec -i -u postgres "$pg" psql -X -q -t -A -v ON_ERROR_STOP=1 -d "$1"
}

drill_sh() {
    docker exec -i -u postgres "$scratch" bash -c "$1"
}

drill_sql() {
    docker exec -i -u postgres "$scratch" psql -X -q -t -A -v ON_ERROR_STOP=1 -h "$DRILL" -p 5432 -U postgres -d "$1"
}

# One line per table of schema public of oteryn_platform: name=count, then the migrations head.
REFERENCE_SQL="
SELECT 'C|' || coalesce(string_agg(format('%s=%s', c.relname,
         (xpath('/row/c/text()', query_to_xml(format('SELECT count(*) AS c FROM public.%I', c.relname), false, true, '')))[1]::text),
         ',' ORDER BY c.relname), '')
  FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
 WHERE n.nspname = 'public' AND c.relkind IN ('r', 'p');
SELECT 'M|' || CASE WHEN to_regclass('public.migrations') IS NULL THEN 'absent'
    ELSE (xpath('/row/h/text()', query_to_xml(
        'SELECT count(*) || '':'' || coalesce((SELECT migration FROM public.migrations ORDER BY id DESC LIMIT 1), ''none'') AS h FROM public.migrations',
        false, true, '')))[1]::text END;
"
LEDGER_SQL="
SELECT CASE WHEN to_regclass('public.wallet_accounts') IS NULL OR to_regclass('public.wallet_ledger_entries') IS NULL
    THEN 'absent'
    ELSE (xpath('/row/m/text()', query_to_xml(
        'SELECT count(*) AS m FROM public.wallet_accounts a
           LEFT JOIN (SELECT identity_id, sum(available_delta) AS av, sum(reserved_delta) AS rv
                        FROM public.wallet_ledger_entries GROUP BY identity_id) l USING (identity_id)
          WHERE a.available_balance <> coalesce(l.av, 0) OR a.reserved_balance <> coalesce(l.rv, 0)',
        false, true, '')))[1]::text END;
"

pg="$(find_service postgres)"
[[ -n "$pg" ]] || fail "no running postgres container in Compose project $PROJECT"
image="$(docker inspect --format '{{.Config.Image}}' "$pg")"
spool_volume="$(docker inspect --format '{{range .Mounts}}{{if eq .Destination "'"$SPOOL"'"}}{{.Name}}{{end}}{{end}}' "$pg")"
[[ -n "$spool_volume" ]] || fail "the postgres container mounts no backup spool at $SPOOL"
# The name reaches a --mount option list: a Docker volume name only, never extra options.
[[ "$spool_volume" =~ ^[A-Za-z0-9][A-Za-z0-9_.-]{0,254}$ ]] || fail "unexpected backup spool volume name"
record drill "$stamp"
record project "$PROJECT"
record target_mode "$target"
record postgres_image "$image"
live_system_identifier="$(live_sql postgres <<< 'SELECT system_identifier FROM pg_control_system()')"
[[ "$live_system_identifier" =~ ^[0-9]{1,20}$ ]] || fail "cannot read the system identifier of the live server"

[[ "$(live_sql postgres <<< "SELECT current_setting('archive_mode')")" == on ]] || fail "archive_mode is not on"
IFS='|' read -r archive_timeout last_archived last_failed ready_age <<< "$(live_sql postgres <<'SQL'
SELECT extract(epoch FROM current_setting('archive_timeout')::interval)::bigint,
       coalesce(extract(epoch FROM a.last_archived_time)::bigint, 0),
       coalesce(extract(epoch FROM a.last_failed_time)::bigint, 0),
       coalesce((SELECT extract(epoch FROM now() - min(s.modification))::bigint
                 FROM pg_ls_archive_statusdir() s WHERE s.name LIKE '%.ready'), 0)
  FROM pg_stat_archiver a;
SQL
)"
[[ "$archive_timeout" =~ ^[0-9]{1,9}$ && "$last_archived" =~ ^[0-9]{1,12}$ && "$last_failed" =~ ^[0-9]{1,12}$ \
    && "$ready_age" =~ ^-?[0-9]{1,12}$ ]] || fail "cannot read pg_stat_archiver of the live server"
archiver_healthy=false
if (( last_failed <= last_archived && ready_age <= 600 )); then
    archiver_healthy=true
fi
record archiver_healthy "$archiver_healthy"

reference_counts=''
reference_migrations=''
drill_id="$stamp-$$"

# Marker rows live in table oteryn_pitr_drill of the `postgres` maintenance database, never in oteryn_platform.
write_marker() {
    live_sql postgres <<SQL > /dev/null || fail "cannot write drill marker $1"
SET client_min_messages = warning;
CREATE TABLE IF NOT EXISTS public.oteryn_pitr_drill (
    id bigserial PRIMARY KEY,
    drill text NOT NULL,
    marker text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT clock_timestamp()
);
INSERT INTO public.oteryn_pitr_drill (drill, marker) VALUES ('$drill_id', '$1');
SQL
}

# Recovery to a time stops at the first commit after it; without one in the archive it ends before the
# target. So a commit after the target is written, its WAL segment switched out and its archiving awaited.
archive_commit_after_target() {
    local segment deadline
    write_marker "$1"
    segment="$(live_sql postgres <<< 'SELECT pg_walfile_name(pg_switch_wal())')"
    [[ "$segment" =~ ^[0-9A-F]{24}$ ]] || fail "unexpected WAL segment $segment"
    deadline=$((SECONDS + ARCHIVE_WAIT_SECONDS))
    until [[ "$(live_sql postgres <<< "SELECT coalesce(substr(last_archived_wal, 1, 24) >= '$segment', false) FROM pg_stat_archiver")" == t ]]; do
        (( SECONDS < deadline )) || fail "WAL segment $segment was not archived within ${ARCHIVE_WAIT_SECONDS}s"
        sleep 2
    done
    record forced_segment "$segment"
}

if [[ "$target" == probe ]]; then
    log "probe: marker A, reference snapshot, marker B"
    write_marker A
    sleep 1
    reference="$(live_sql oteryn_platform <<SQL
BEGIN ISOLATION LEVEL REPEATABLE READ;
SELECT 'T|' || to_char(t AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US') || '|' || floor(extract(epoch FROM t))::bigint
  FROM (SELECT clock_timestamp() AS t) x;
$REFERENCE_SQL
COMMIT;
SQL
)" || fail "cannot take the reference snapshot"
    IFS='|' read -r _ target_time target_epoch <<< "$(grep '^T|' <<< "$reference")"
    [[ "$target_epoch" =~ ^[0-9]{1,12}$ ]] || fail "cannot read the reference snapshot time"
    reference_counts="$(sed -n 's/^C|//p' <<< "$reference")"
    reference_migrations="$(sed -n 's/^M|//p' <<< "$reference")"
    sleep 1
    archive_commit_after_target B
elif [[ "$target" == time ]]; then
    target_epoch="$(date -u -d "$target_time" +%s)" || fail "cannot parse --target-time"
    (( target_epoch < $(date +%s) )) || fail "--target-time must be in the past"
    archive_commit_after_target C
fi
if [[ "$target" != immediate ]]; then
    record target_time "$target_time UTC"
fi

log "starting scratch instance $scratch (no network, anonymous data volume, spool read-only)"
docker create --name "$scratch" --rm --label oteryn.pitr-drill=true --network none --user postgres \
    --mount "type=volume,source=$spool_volume,target=$SPOOL,readonly" \
    --mount "type=volume,target=$DATA" \
    --entrypoint bash "$image" -c "$SCRATCH_MAIN" oteryn-pitr-drill "$SPOOL" "$LOCK_WAIT_SECONDS" "$LOCKED_MARKER" > /dev/null \
    || fail "cannot create the scratch container"
scratch_created=true
docker start "$scratch" > /dev/null || fail "cannot start the scratch container"
# The container removes itself (--rm) when its lock holder exits: an empty name is no error here.
scratch_volume="$(docker inspect --format '{{range .Mounts}}{{if eq .Destination "'"$DATA"'"}}{{.Name}}{{end}}{{end}}' "$scratch" 2> /dev/null)" || true
log "waiting for the spool job lock (a running backup job finishes first, at most ${LOCK_WAIT_SECONDS}s)"
deadline=$((SECONDS + LOCK_WAIT_SECONDS + 60))
until drill_sh "test -f $LOCKED_MARKER" 2> /dev/null; do
    [[ "$(docker inspect --format '{{.State.Running}}' "$scratch" 2> /dev/null)" == true ]] \
        || fail "the scratch instance could not take the spool job lock (no backup job ever ran, or one held it over ${LOCK_WAIT_SECONDS}s)"
    (( SECONDS < deadline )) || fail "the scratch instance did not take the spool job lock within ${LOCK_WAIT_SECONDS}s"
    sleep 2
done
record spool_lock "shared job lock held from base backup choice to the end of the drill"
restore_started="$(date +%s)"

backups="$(drill_sh "for f in $SPOOL/base/*Z/backup.info; do [ -f \"\$f\" ] || continue; printf '%s %s %s %s\n' \"\$(sed -n 's/^stamp=//p' \"\$f\")\" \"\$(sed -n 's/^end_epoch=//p' \"\$f\")\" \"\$(sed -n 's/^cluster_bytes=//p' \"\$f\")\" \"\$(sed -n 's/^system_identifier=//p' \"\$f\")\"; done | sort -r")"
chosen=''
cluster_bytes=0
# Spool content is data: a value of an unexpected shape never reaches a command line or arithmetic.
while read -r candidate end_epoch bytes system_identifier; do
    [[ "$candidate" =~ ^[0-9]{8}T[0-9]{6}Z$ && "$end_epoch" =~ ^[0-9]{1,12}$ && "$bytes" =~ ^[0-9]{1,15}$ ]] || continue
    # Only base backups of the live cluster (a re-initialised server is another cluster).
    [[ "$system_identifier" == "$live_system_identifier" ]] || continue
    if [[ "$target" == immediate ]]; then
        [[ "$candidate" == "$backup_stamp" ]] || continue
    elif (( end_epoch > target_epoch )); then
        continue
    fi
    chosen="$candidate"
    cluster_bytes="$bytes"
    base_end_epoch="$end_epoch"
    break
done <<< "$backups"
[[ -n "$chosen" ]] || fail "no complete base backup in the spool ends before the target (run: backup-postgres.sh run base)"
record base_backup "$chosen"
log "base backup $chosen"

free_kb="$(drill_sh "df -Pk $DATA | awk 'NR == 2 { print \$4 }'")"
[[ "$free_kb" =~ ^[0-9]{1,15}$ ]] || fail "cannot read the free space of the scratch data volume"
required_kb=$(( cluster_bytes / 1024 + WAL_MARGIN_MB * 1024 ))
(( free_kb >= required_kb )) \
    || fail "scratch volume has $((free_kb / 1024)) MiB free, $((required_kb / 1024)) MiB required (cluster plus ${WAL_MARGIN_MB} MiB of WAL)"

drill_sh "
set -euo pipefail
cd $SPOOL/base/$chosen
sha256sum --quiet -c SHA256SUMS
chmod 0700 $DATA
tar -xzf base.tar.gz -C $DATA
tar -xzf pg_wal.tar.gz -C $DATA/pg_wal
pg_verifybackup -q -m $SPOOL/base/$chosen/backup_manifest $DATA
# ALTER SYSTEM settings of the live server (archive_command, listen_addresses) would override the drill
# configuration below: the scratch instance runs with the drill configuration alone.
: > $DATA/postgresql.auto.conf
" || fail "base backup $chosen failed its checksums or pg_verifybackup"
record base_verified "SHA256SUMS and pg_verifybackup passed"

if [[ "$target" == immediate ]]; then
    recovery_target="recovery_target = 'immediate'"
else
    recovery_target="recovery_target_time = '$target_time+00'"
fi
drill_sh "mkdir -p $DRILL && cat > $DRILL/restore-wal.sh" < "$RESTORE_WAL"
drill_sh "cat > $DRILL/pg_hba.conf" <<'HBA'
# Scratch drill instance only: no network namespace beyond loopback.
local all all              trust
host  all all 127.0.0.1/32 trust
HBA
drill_sh "cat > $DRILL/postgresql.conf" <<CONF
hba_file = '$DRILL/pg_hba.conf'
ident_file = '$DATA/pg_ident.conf'
listen_addresses = '127.0.0.1'
port = 5432
unix_socket_directories = '$DRILL'
max_connections = 50
max_wal_senders = 3
ssl = off
hot_standby = off
shared_buffers = 128MB
wal_level = replica
archive_mode = off
timezone = 'UTC'
log_timezone = 'UTC'
log_destination = 'stderr'
logging_collector = off
restore_command = 'bash $DRILL/restore-wal.sh "%f" "%p"'
$recovery_target
recovery_target_action = 'promote'
CONF
log "recovering to ${target_time:-the end of $chosen}"
drill_sh "touch $DATA/recovery.signal && pg_ctl -D $DATA -o '-c config_file=$DRILL/postgresql.conf' -l $DRILL/server.log -w -t $RTO_SECONDS start > /dev/null" \
    || fail "recovery did not reach the target (server log below)"
# pg_ctl can return before the end of recovery; the server exits (FATAL) when the archive ends before
# the target, which fails the drill at once instead of waiting for a promotion that never comes.
deadline=$((restore_started + RTO_SECONDS))
until [[ "$(drill_sql postgres <<< 'SELECT pg_is_in_recovery()' 2>/dev/null)" == f ]]; do
    drill_sh "pg_ctl -D $DATA status > /dev/null" || fail "recovery stopped before the target (server log below)"
    (( $(date +%s) < deadline )) || fail "the scratch instance was not promoted within the RTO of ${RTO_SECONDS}s"
    sleep 2
done
restored_epoch="$(date +%s)"
restore_seconds=$((restored_epoch - restore_started))
record restore_seconds "$restore_seconds"
record replayed_to "$(drill_sql postgres <<< "SELECT coalesce(to_char(pg_last_xact_replay_timestamp() AT TIME ZONE 'UTC', 'YYYY-MM-DD HH24:MI:SS.US'), 'none')") UTC"
log "promoted after ${restore_seconds}s"

checks_passed=0
if [[ "$target" == probe ]]; then
    markers="$(drill_sql postgres <<< "SELECT count(*) FILTER (WHERE marker = 'A') || ':' || count(*) FILTER (WHERE marker = 'B') FROM public.oteryn_pitr_drill WHERE drill = '$drill_id'")"
    [[ "$markers" == 1:0 ]] || fail "markers A:B are $markers after restore to T, expected 1:0"
    record markers "A present, B absent"
    checks_passed=$((checks_passed + 1))
fi

restored="$(drill_sql oteryn_platform <<< "$REFERENCE_SQL")" || fail "cannot read the restored oteryn_platform database"
restored_counts="$(sed -n 's/^C|//p' <<< "$restored")"
restored_migrations="$(sed -n 's/^M|//p' <<< "$restored")"
record tables "$(tr ',' '\n' <<< "$restored_counts" | grep -c . || true)"
record migrations_head "$restored_migrations"
if [[ "$target" == probe ]]; then
    [[ "$restored_counts" == "$reference_counts" ]] || fail "row counts differ from the reference snapshot at T"
    [[ "$restored_migrations" == "$reference_migrations" ]] \
        || fail "migrations head $restored_migrations differs from the reference $reference_migrations"
    record row_counts "equal to the reference snapshot"
    checks_passed=$((checks_passed + 2))
fi

ledger="$(drill_sql oteryn_platform <<< "$LEDGER_SQL")" || fail "cannot check the wallet ledger invariant"
if [[ "$ledger" == absent ]]; then
    record ledger_invariant not_applicable
else
    [[ "$ledger" == 0 ]] || fail "$ledger wallet accounts differ from the sum of their ledger entries"
    record ledger_invariant "0 mismatched accounts"
    checks_passed=$((checks_passed + 1))
fi

dump_result="$(drill_sh "
newest=\$(ls -1d $SPOOL/dump/*Z 2>/dev/null | sort -r | while read -r d; do [ -f \"\$d/dump.info\" ] && { echo \"\$d\"; break; }; done)
[ -n \"\$newest\" ] || { echo none; exit 0; }
cd \"\$newest\" && sha256sum --quiet -c SHA256SUMS && pg_restore -l oteryn_platform.dump > /dev/null && echo \"\${newest##*/}\"
")" || fail "the newest logical dump failed its checksum or cannot be listed"
[[ "$dump_result" != none ]] || fail "no complete logical dump in the spool"
[[ "$dump_result" =~ ^[0-9]{8}T[0-9]{6}Z$ ]] || fail "unexpected logical dump directory name in the spool"
record dump_verified "$dump_result (SHA256SUMS and pg_restore -l)"
checks_passed=$((checks_passed + 1))

if [[ "$platform_checks" == true ]]; then
    platform="$(find_service platform)"
    [[ -n "$platform" ]] || fail "no running platform container to take the image from (or pass --skip-platform-checks)"
    platform_image="$(docker inspect --format '{{.Config.Image}}' "$platform")"
    engine=mariadb
    if docker inspect --format '{{range .Config.Env}}{{println .}}{{end}}' "$platform" | grep -qx 'DB_CONNECTION=pgsql'; then
        engine=postgres
    fi
    record platform_image "$platform_image"
    record platform_engine "$engine"
    # Throwaway production-shaped configuration: an ephemeral key, no secret, nothing reachable.
    app_key="base64:$(head -c 32 /dev/urandom | base64)"
    platform_artisan() {
        docker run --rm --network "container:$scratch" --entrypoint php -w /var/www/html \
            -e APP_ENV=production -e APP_DEBUG=false -e "APP_KEY=$app_key" -e "APP_URL=$DRILL_APP_URL" \
            -e LOG_CHANNEL=stderr -e LOG_LEVEL=warning -e MARKETPLACE_ENABLED=false -e PAYMENTS_ENABLED=false \
            -e DB_CONNECTION=pgsql -e DB_HOST=127.0.0.1 -e DB_PORT=5432 -e DB_DATABASE=oteryn_platform \
            -e DB_USERNAME=oteryn_platform_owner -e DB_PASSWORD=drill-trust-auth-not-a-secret -e DB_SSLMODE=disable \
            -e SESSION_SECURE_COOKIE=true -e SESSION_HTTP_ONLY=true -e MAIL_MAILER=smtp \
            -e MAIL_HOST=smtp.oteryn.molehill.cloud -e MAIL_PORT=587 -e MAIL_FROM_ADDRESS=pitr-drill@oteryn.molehill.cloud \
            -e "MAIL_FROM_NAME=Oteryn PITR drill" \
            "$platform_image" artisan "$@" --no-interaction
    }
    if [[ "$restored_migrations" == absent && "$engine" == mariadb ]]; then
        record migrate_status "not_applicable (engine mariadb: the PostgreSQL database holds no Platform schema yet)"
    else
        set +e
        platform_artisan migrate:status --pending=3 > /dev/null
        status=$?
        set -e
        case "$status" in
            0) record migrate_status "no pending migration for $platform_image" ;;
            3) fail "the restored database has pending migrations for the running Platform image" ;;
            *) fail "migrate:status failed on the restored database (exit $status)" ;;
        esac
        checks_passed=$((checks_passed + 1))
    fi
    platform_artisan production:verify-configuration > /dev/null \
        || fail "production:verify-configuration failed against the restored instance"
    record verify_configuration passed
    checks_passed=$((checks_passed + 1))
else
    record platform_checks "skipped (--skip-platform-checks)"
fi

if [[ "$archiver_healthy" == true && "$target" != immediate ]]; then
    data_loss_bound="$archive_timeout"
    record data_loss_basis "archive_timeout of a healthy archiver"
else
    data_loss_bound=$(( started_epoch - base_end_epoch ))
    record data_loss_basis "age of base backup $chosen (archiver unhealthy or base-only restore)"
fi
record data_loss_bound_seconds "$data_loss_bound"
record rpo_seconds "$RPO_SECONDS"
record rto_seconds "$RTO_SECONDS"
(( restore_seconds <= RTO_SECONDS )) || fail "restore took ${restore_seconds}s, above the RTO of ${RTO_SECONDS}s"
(( data_loss_bound <= RPO_SECONDS )) || fail "data-loss bound ${data_loss_bound}s is above the RPO of ${RPO_SECONDS}s"
record checks_passed "$checks_passed"
record total_seconds "$(( $(date +%s) - started_epoch ))"
record result passed
log "PITR drill passed: restored to ${target_time:-$chosen} in ${restore_seconds}s, data-loss bound ${data_loss_bound}s"
