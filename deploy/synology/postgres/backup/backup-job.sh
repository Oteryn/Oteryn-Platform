#!/bin/bash
# Platform PostgreSQL backup jobs (docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md P5, runbook
# docs/operations/PLATFORM_POSTGRES_BACKUP_PITR.md). The postgres-backup sidecar (compose.yml) runs
# `backup-job.sh service`; backup-postgres.sh on the NAS runs the other modes through `docker exec`.
#
# NAS spool (the postgres_backup volume, mounted at $SPOOL, never inside postgres_data):
#   wal/<segment>.gz{,.sha256}   continuous WAL archive written by archive-wal.sh (archive_command)
#   base/<stamp>/                daily physical base backup by the backup role (pg_basebackup, REPLICATION):
#                                base.tar.gz, pg_wal.tar.gz, backup_manifest (SHA-256 per file),
#                                SHA256SUMS, backup.info (written last: the completion marker)
#   dump/<stamp>/                daily logical dump by the read-only dump role (pg_dump -Fc):
#                                oteryn_platform.dump, oteryn_platform.toc, SHA256SUMS, dump.info
#   status/                      heartbeat, <job>.last results, the job lock
# Retention: the newest 7 base backups and dumps plus the newest of each of the 4 most recent older ISO
# weeks. WAL older than the start of the oldest retained daily base backup is removed, so PITR reaches
# back to that backup; a weekly backup restores to its own end (it carries its WAL in pg_wal.tar.gz).
#
# Modes: service | loop | credentials | base | dump | ensure | prune | preflight | status | retained <kind>
set -euo pipefail
umask 077
export LC_ALL=C TZ=UTC

SPOOL="${OTERYN_BACKUP_SPOOL:-/var/lib/oteryn-postgres-backup}"
PG_HOST="${OTERYN_BACKUP_PGHOST:-postgres}"
PG_CA="${OTERYN_BACKUP_CA:-/run/oteryn-postgres-ca/ca.crt}"
DATABASE=oteryn_platform
BACKUP_ROLE=oteryn_platform_backup
DUMP_ROLE=oteryn_platform_dump
BACKUP_HOUR_UTC="${OTERYN_BACKUP_HOUR_UTC:-2}"
INTERVAL_SECONDS="${OTERYN_BACKUP_INTERVAL_SECONDS:-300}"
KEEP_DAILY="${OTERYN_BACKUP_KEEP_DAILY:-7}"
KEEP_WEEKLY="${OTERYN_BACKUP_KEEP_WEEKLY:-4}"
# Free space the spool must keep before a backup starts (preflight): room for one more base backup and
# one more dump at the uncompressed database size each, plus a day of WAL, and never below MIN_FREE_MB.
MIN_FREE_MB="${OTERYN_BACKUP_MIN_FREE_MB:-2048}"
WAL_RESERVE_MB="${OTERYN_BACKUP_WAL_RESERVE_MB:-1024}"
RETRY_BACKOFF_SECONDS=3600

log() { printf '%s oteryn-postgres-backup %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$*" >&2; }
die() { log "error: $*"; exit 1; }

# Decimal integers of at most seven digits; a leading zero (08) is read as decimal, never octal.
decimal_setting() {
    local name="$1" value="${!1}"
    [[ "$value" =~ ^[0-9]{1,7}$ ]] || die "$name must be a decimal integer of at most seven digits"
    printf -v "$name" '%d' "$((10#$value))"
}
for setting in BACKUP_HOUR_UTC INTERVAL_SECONDS KEEP_DAILY KEEP_WEEKLY MIN_FREE_MB WAL_RESERVE_MB; do
    decimal_setting "$setting"
done
(( BACKUP_HOUR_UTC <= 23 && KEEP_DAILY >= 1 && KEEP_DAILY <= 366 && KEEP_WEEKLY <= 520 \
    && INTERVAL_SECONDS >= 10 && INTERVAL_SECONDS <= 3600 )) || die "backup settings are out of range"

stamp_epoch() {
    [[ "$1" =~ ^([0-9]{4})([0-9]{2})([0-9]{2})T([0-9]{2})([0-9]{2})([0-9]{2})Z$ ]] || return 1
    date -u -d "${BASH_REMATCH[1]}-${BASH_REMATCH[2]}-${BASH_REMATCH[3]} ${BASH_REMATCH[4]}:${BASH_REMATCH[5]}:${BASH_REMATCH[6]}" +%s
}

iso_week() { date -u -d "@$(stamp_epoch "$1")" +%G-W%V; }

info_file() { [[ "$1" == base ]] && echo backup.info || echo dump.info; }

info_value() { sed -n "s/^$2=//p" "$1" | head -n 1; }

# Complete backups of one kind, newest first (a directory counts only once its info file exists).
complete_backups() {
    local kind="$1" info dir
    info="$(info_file "$kind")"
    [[ -d "$SPOOL/$kind" ]] || return 0
    for dir in "$SPOOL/$kind"/*Z; do
        [[ -d "$dir" && -f "$dir/$info" && "${dir##*/}" =~ ^[0-9]{8}T[0-9]{6}Z$ ]] && printf '%s\n' "${dir##*/}"
    done | sort -r
}

# BASHPID, not $$: the heartbeat subshell writes beside the loop. prune removes a temporary file a killed
# writer left behind.
write_atomic() {
    local file="$1" tmp
    tmp="$1.tmp.$BASHPID"
    cat > "$tmp"
    mv -f -- "$tmp" "$file"
}

JOB_RECORDED=0
record_status() {
    local job="$1" result="$2" detail="$3"
    JOB_RECORDED=1
    printf 'result=%s\nepoch=%s\ntime=%s\ndetail=%s\n' "$result" "$(date +%s)" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$detail" \
        | write_atomic "$SPOOL/status/$job.last"
}

conninfo() {
    local role="$1" database="${2:-}"
    printf 'host=%s port=5432 user=%s sslmode=verify-full sslrootcert=%s connect_timeout=10 application_name=oteryn-postgres-backup' \
        "$PG_HOST" "$role" "$PG_CA"
    [[ -z "$database" ]] || printf ' dbname=%s' "$database"
}

# The role passwords reach libpq through a 0600 password file in the container's own filesystem (never
# the spool, never an argument or the container environment). backup-postgres.sh writes it with
# `credentials`, which reads the backup role password and then the dump role password from stdin.
PGPASS_FILE="${OTERYN_BACKUP_PGPASS_FILE:-/tmp/oteryn-postgres-backup.pgpass}"

write_credentials() {
    local backup_password dump_password
    IFS= read -r backup_password || true
    IFS= read -r dump_password || true
    [[ "$backup_password" =~ ^[A-Za-z0-9_.-]{24,}$ && "$dump_password" =~ ^[A-Za-z0-9_.-]{24,}$ ]] \
        || die "credentials expects the backup and dump role passwords, one per line"
    printf '%s:5432:replication:%s:%s\n%s:5432:%s:%s:%s\n' \
        "$PG_HOST" "$BACKUP_ROLE" "$backup_password" "$PG_HOST" "$DATABASE" "$DUMP_ROLE" "$dump_password" \
        | write_atomic "$PGPASS_FILE"
    chmod 0600 "$PGPASS_FILE"
}

use_credentials() {
    if [[ ! -s "$PGPASS_FILE" ]]; then
        log "backup credentials are not provisioned (backup-postgres.sh reconcile writes them)"
        return 1
    fi
    export PGPASSFILE="$PGPASS_FILE"
}

dump_sql() { psql -X -q -t -A -w -v ON_ERROR_STOP=1 -d "$(conninfo "$DUMP_ROLE" "$DATABASE")" -c "$1"; }

spool_free_mb() { df -Pm -- "$SPOOL" | awk 'NR == 2 { print $4 }'; }

require_spool() {
    local dir
    for dir in "$SPOOL" "$SPOOL/wal" "$SPOOL/base" "$SPOOL/dump" "$SPOOL/status"; do
        [[ -d "$dir" && -w "$dir" ]] || die "spool directory $dir is missing or not writable"
    done
}

take_lock() {
    exec 9> "$SPOOL/status/job.lock"
    flock -w 7200 9 || die "another backup job holds the lock"
}

# Prints database_mb, required_free_mb, spool_free_mb and result; exits 1 when the spool lacks room.
preflight() {
    local db_bytes db_mb required free
    require_spool
    use_credentials || { printf 'result=credentials_missing\n'; return 1; }
    if ! db_bytes="$(dump_sql 'SELECT pg_database_size(current_database())')" || [[ ! "$db_bytes" =~ ^[0-9]+$ ]]; then
        printf 'result=database_unreachable\n'
        log "cannot read the database size as $DUMP_ROLE"
        return 1
    fi
    db_mb=$(( (db_bytes + 1048575) / 1048576 ))
    required=$(( 2 * (db_mb + 64) + WAL_RESERVE_MB ))
    (( required >= MIN_FREE_MB )) || required="$MIN_FREE_MB"
    free="$(spool_free_mb)"
    [[ "$free" =~ ^[0-9]+$ ]] || die "cannot read the spool free space"
    printf 'database_mb=%s\nrequired_free_mb=%s\nspool_free_mb=%s\n' "$db_mb" "$required" "$free"
    if (( free < required )); then
        printf 'result=refused\n'
        log "refused: the NAS spool has ${free} MiB free, ${required} MiB required (2 x (database ${db_mb} MiB + 64) + ${WAL_RESERVE_MB} MiB WAL, minimum ${MIN_FREE_MB} MiB)"
        return 1
    fi
    printf 'result=ok\n'
}

# Runs preflight before a job; records why it refused (refused | database_unreachable | credentials_missing).
preflight_or_refuse() {
    local output
    if ! output="$(preflight)"; then
        record_status "$1" refused "$(sed -n 's/^result=//p' <<< "$output" | head -n 1)"
        exit 1
    fi
    # preflight ran in a subshell: export the password file to this job as well.
    use_credentials
}

# Removes the partial directory and records the failure unless the job reached its end.
job_trap() {
    local status=$?
    local job="$1" work="$2"
    if [[ -n "$work" && -e "$work" ]]; then
        rm -rf -- "$work"
    fi
    if (( status != 0 && JOB_RECORDED == 0 )); then
        record_status "$job" failed "exit $status (see the postgres-backup container log)"
    fi
}

run_base() {
    local stamp final work label start_wal start_time cluster_bytes end_epoch system_identifier
    require_spool
    take_lock
    trap "job_trap base ''" EXIT
    preflight_or_refuse base
    stamp="$(date -u +%Y%m%dT%H%M%SZ)"
    final="$SPOOL/base/$stamp"
    work="$SPOOL/base/.$stamp.partial"
    [[ ! -e "$final" ]] || die "base backup $stamp already exists"
    # shellcheck disable=SC2064 # the paths are fixed for this job
    trap "job_trap base '$work'" EXIT
    mkdir -- "$work"
    log "base backup $stamp starting"
    pg_basebackup -w -d "$(conninfo "$BACKUP_ROLE")" -D "$work" -Ft -z -X stream -c fast \
        --manifest-checksums=SHA256 -l "oteryn-$stamp"
    gzip -t -- "$work/base.tar.gz" "$work/pg_wal.tar.gz"
    label="$(tar -xzOf "$work/base.tar.gz" backup_label)"
    start_wal="$(sed -n 's/^START WAL LOCATION: .* (file \([0-9A-F]\{24\}\))$/\1/p' <<< "$label")"
    start_time="$(sed -n 's/^START TIME: //p' <<< "$label")"
    [[ "$start_wal" =~ ^[0-9A-F]{24}$ ]] || die "backup_label has no start WAL file"
    cluster_bytes="$(grep -o '"Size": *[0-9]*' "$work/backup_manifest" | awk -F: '{ s += $2 } END { print s + 0 }')"
    # The cluster the backup belongs to: a re-initialised server is another cluster (backup-postgres.sh check).
    system_identifier="$(grep -o '"System-Identifier": *[0-9]*' "$work/backup_manifest" | grep -o '[0-9]*$' | head -n 1)"
    [[ "$system_identifier" =~ ^[0-9]{1,20}$ ]] || die "backup_manifest has no system identifier"
    end_epoch=$(( $(date +%s) + 1 ))
    (cd "$work" && sha256sum base.tar.gz pg_wal.tar.gz backup_manifest > SHA256SUMS)
    sync -- "$work"/*
    printf 'format=1\nkind=base\nstamp=%s\nsystem_identifier=%s\nstart_wal=%s\nstart_time=%s\nend_epoch=%s\nend_time=%s\ncluster_bytes=%s\narchive_bytes=%s\n' \
        "$stamp" "$system_identifier" "$start_wal" "$start_time" "$end_epoch" "$(date -u -d "@$end_epoch" +%Y-%m-%dT%H:%M:%SZ)" \
        "$cluster_bytes" "$(du -sb -- "$work" | awk '{ print $1 }')" > "$work/backup.info"
    sync -- "$work/backup.info" "$work"
    mv -- "$work" "$final"
    sync -- "$SPOOL/base"
    record_status base ok "$stamp start_wal=$start_wal"
    log "base backup $stamp complete (start WAL $start_wal)"
}

run_dump() {
    local stamp final work end_epoch
    require_spool
    take_lock
    trap "job_trap dump ''" EXIT
    preflight_or_refuse dump
    stamp="$(date -u +%Y%m%dT%H%M%SZ)"
    final="$SPOOL/dump/$stamp"
    work="$SPOOL/dump/.$stamp.partial"
    [[ ! -e "$final" ]] || die "dump $stamp already exists"
    # shellcheck disable=SC2064 # the paths are fixed for this job
    trap "job_trap dump '$work'" EXIT
    mkdir -- "$work"
    log "logical dump $stamp starting"
    pg_dump -w -d "$(conninfo "$DUMP_ROLE" "$DATABASE")" -Fc -Z 6 -f "$work/oteryn_platform.dump"
    pg_restore -l "$work/oteryn_platform.dump" > "$work/oteryn_platform.toc"
    end_epoch=$(( $(date +%s) + 1 ))
    (cd "$work" && sha256sum oteryn_platform.dump oteryn_platform.toc > SHA256SUMS)
    sync -- "$work"/*
    printf 'format=1\nkind=dump\nstamp=%s\nrole=%s\nend_epoch=%s\nend_time=%s\narchive_bytes=%s\n' \
        "$stamp" "$DUMP_ROLE" "$end_epoch" "$(date -u -d "@$end_epoch" +%Y-%m-%dT%H:%M:%SZ)" \
        "$(du -sb -- "$work" | awk '{ print $1 }')" > "$work/dump.info"
    sync -- "$work/dump.info" "$work"
    mv -- "$work" "$final"
    sync -- "$SPOOL/dump"
    record_status dump ok "$stamp"
    log "logical dump $stamp complete"
}

# Prints the stamps retention keeps for one kind: the newest KEEP_DAILY, then the newest of each of the
# KEEP_WEEKLY most recent older ISO weeks.
retained() {
    local kind="$1" index=0 stamp week weeks=' ' kept_weeks=0
    while IFS= read -r stamp; do
        [[ -n "$stamp" ]] || continue
        if (( index < KEEP_DAILY )); then
            printf '%s\n' "$stamp"
        else
            week="$(iso_week "$stamp")"
            if [[ "$weeks" != *" $week "* ]] && (( kept_weeks < KEEP_WEEKLY )); then
                weeks+="$week "
                kept_weeks=$((kept_weeks + 1))
                printf '%s\n' "$stamp"
            fi
        fi
        index=$((index + 1))
    done < <(complete_backups "$kind")
}

# The WAL file every retained daily base backup needs: the start WAL of the oldest of them.
wal_cutoff() {
    local oldest
    oldest="$(complete_backups base | head -n "$KEEP_DAILY" | tail -n 1)"
    [[ -n "$oldest" ]] || return 0
    info_value "$SPOOL/base/$oldest/backup.info" start_wal
}

prune_unlocked() {
    local kind stamp keep cutoff file name removed=0
    for kind in base dump; do
        keep=" $(retained "$kind" | tr '\n' ' ') "
        while IFS= read -r stamp; do
            [[ -n "$stamp" && "$keep" != *" $stamp "* ]] || continue
            rm -rf -- "${SPOOL:?}/$kind/$stamp"
            log "retention removed $kind $stamp"
        done < <(complete_backups "$kind")
        # Interrupted jobs: nothing else runs while the job lock is held.
        find "$SPOOL/$kind" -mindepth 1 -maxdepth 1 -name '.*.partial' -exec rm -rf -- {} +
    done
    cutoff="$(wal_cutoff)"
    if [[ "$cutoff" =~ ^[0-9A-F]{24}$ ]]; then
        for file in "$SPOOL/wal"/*.gz; do
            [[ -f "$file" ]] || continue
            name="${file##*/}"
            name="${name%.gz}"
            # Timeline history files stay; segments compare by log and segment number, not timeline.
            [[ "$name" =~ ^[0-9A-F]{24}(\.partial|\.[0-9A-F]{8}\.backup)?$ ]] || continue
            if [[ "${name:8:16}" < "${cutoff:8:16}" ]]; then
                rm -f -- "$file" "$file.sha256"
                removed=$((removed + 1))
            fi
        done
    fi
    find "$SPOOL/wal" -maxdepth 1 -name '.*.tmp.*' -mmin +60 -delete
    find "$SPOOL/status" -maxdepth 1 -name '*.tmp.*' -mmin +60 -delete
    record_status prune ok "wal_removed=$removed cutoff=${cutoff:-none}"
    log "retention kept base [$(retained base | tr '\n' ' ')] dump [$(retained dump | tr '\n' ' ')]; removed $removed WAL files before ${cutoff:-none}"
}

run_prune() {
    require_spool
    take_lock
    prune_unlocked
}

newest_age() {
    local newest
    newest="$(complete_backups "$1" | head -n 1)"
    [[ -n "$newest" ]] || { echo none; return 0; }
    echo $(( $(date +%s) - $(stamp_epoch "$newest") ))
}

# Due once a day at BACKUP_HOUR_UTC, or whenever the newest is missing or 25 hours old.
backup_due() {
    local age
    age="$(newest_age "$1")"
    [[ "$age" == none ]] && return 0
    (( age >= 90000 )) && return 0
    (( age >= 72000 && 10#$(date -u +%H) == BACKUP_HOUR_UTC ))
}

backing_off() {
    local file="$SPOOL/status/$1.last" result epoch
    [[ -f "$file" ]] || return 1
    result="$(info_value "$file" result)"
    epoch="$(info_value "$file" epoch)"
    [[ "$result" != ok && "$epoch" =~ ^[0-9]+$ ]] && (( $(date +%s) - epoch < RETRY_BACKOFF_SECONDS ))
}

run_loop() {
    local kind self heartbeat_pid
    self="$(readlink -f -- "$0")"
    require_spool
    # The heartbeat (the container healthcheck) runs beside the loop: a base backup of a large database
    # may take longer than the healthcheck window. It dies with the loop, which is the container's PID 1.
    (
        trap 'exit 0' TERM
        while :; do
            date +%s | write_atomic "$SPOOL/status/heartbeat"
            sleep 60 &
            wait $! || true
        done
    ) &
    heartbeat_pid=$!
    trap 'log "stopping"; kill "$heartbeat_pid" 2>/dev/null; exit 0' TERM INT
    log "service started: daily base backup and dump at ${BACKUP_HOUR_UTC}:00 UTC, check every ${INTERVAL_SECONDS}s"
    while :; do
        kill -0 "$heartbeat_pid" 2>/dev/null || die "the heartbeat writer stopped"
        for kind in base dump; do
            if backup_due "$kind" && ! backing_off "$kind"; then
                # Each job is its own process, so errexit and its cleanup trap apply.
                if bash "$self" "$kind"; then
                    bash "$self" prune || log "retention failed"
                else
                    log "$kind backup failed; next attempt in at least ${RETRY_BACKOFF_SECONDS}s"
                fi
            fi
        done
        sleep "$INTERVAL_SECONDS" &
        wait $! || true
    done
}

# Brings both backups to at most 24 hours old (deploy reconcile), then applies retention.
run_ensure() {
    local kind age self
    self="$(readlink -f -- "$0")"
    for kind in base dump; do
        age="$(newest_age "$kind")"
        if [[ "$age" == none ]] || (( age >= 86400 )); then
            bash "$self" "$kind"
        fi
    done
    bash "$self" prune
}

print_status() {
    local kind newest file heartbeat
    for kind in base dump; do
        newest="$(complete_backups "$kind" | head -n 1)"
        printf '%s_newest=%s\n%s_age_seconds=%s\n%s_count=%s\n' "$kind" "${newest:-none}" "$kind" "$(newest_age "$kind")" \
            "$kind" "$(complete_backups "$kind" | grep -c . || true)"
        if [[ "$kind" == base && -n "$newest" ]]; then
            printf 'base_system_identifier=%s\n' "$(info_value "$SPOOL/base/$newest/backup.info" system_identifier)"
        fi
        file="$SPOOL/status/$kind.last"
        [[ -f "$file" ]] && printf '%s_last_result=%s\n' "$kind" "$(info_value "$file" result)"
    done
    printf 'wal_files=%s\n' "$(find "$SPOOL/wal" -maxdepth 1 -name '*.gz' | grep -c . || true)"
    printf 'wal_cutoff=%s\n' "$(wal_cutoff)"
    heartbeat="$(cat "$SPOOL/status/heartbeat" 2>/dev/null || echo 0)"
    printf 'heartbeat_age_seconds=%s\n' "$(( $(date +%s) - heartbeat ))"
    preflight || true
}

mode="${1:-}"
# Jobs run as postgres (the spool owner); the image's gosu drops root. Without gosu (the contract tests)
# a job runs as the calling user.
if [[ "$mode" != service && "$(id -u)" == 0 ]] && command -v gosu > /dev/null; then
    exec gosu postgres bash "$0" "$@"
fi

case "$mode" in
    service)
        [[ "$(id -u)" == 0 ]] || die "service mode starts as root to prepare the spool"
        install -d -o postgres -g postgres -m 0700 "$SPOOL" "$SPOOL/wal" "$SPOOL/base" "$SPOOL/dump" "$SPOOL/status"
        chown postgres:postgres "$SPOOL" "$SPOOL/wal" "$SPOOL/base" "$SPOOL/dump" "$SPOOL/status"
        chmod 0700 "$SPOOL" "$SPOOL/wal" "$SPOOL/base" "$SPOOL/dump" "$SPOOL/status"
        # The healthcheck reads the heartbeat; one left by a previous container must not report this one healthy.
        rm -f -- "$SPOOL/status/heartbeat"
        # Run a private copy: a deploy replaces the staged script under a running loop otherwise.
        install -m 0755 -- "$0" /tmp/oteryn-backup-job.sh
        exec gosu postgres bash /tmp/oteryn-backup-job.sh loop
        ;;
    loop) run_loop ;;
    base) run_base ;;
    dump) run_dump ;;
    prune) run_prune ;;
    ensure) run_ensure ;;
    credentials) write_credentials ;;
    preflight) preflight ;;
    status) print_status ;;
    retained)
        [[ "${2:-}" == base || "${2:-}" == dump ]] || die "retained expects base or dump"
        retained "$2"
        ;;
    *)
        echo "usage: backup-job.sh service|loop|credentials|base|dump|ensure|prune|preflight|status|retained <kind>" >&2
        exit 2
        ;;
esac
