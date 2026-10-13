#!/usr/bin/env bash
# Platform PostgreSQL backups and WAL archive on the NAS (docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md
# P5, runbook docs/operations/PLATFORM_POSTGRES_BACKUP_PITR.md).
#
#   backup-postgres.sh reconcile   deploy.sh, after oteryn_reconcile_postgres: start the postgres-backup
#                                  sidecar, provision its role credentials, refuse when the NAS spool lacks
#                                  the required free space, prove a forced WAL segment reaches the spool,
#                                  bring the base backup and dump to at most 24 hours old, then `check`.
#                                  The backups run detached in the sidecar; the deploy waits for them at
#                                  most OTERYN_BACKUP_ENSURE_BUDGET_SECONDS (default 300). Past that budget
#                                  they finish in the background and the reconcile accepts a degraded check.
#   backup-postgres.sh check       health-check.sh: sidecar running and healthy, archive_mode on (exit 1
#                                  otherwise); no archive failure backlog, base backup and dump of the live
#                                  cluster at most 26 hours old, spool room (exit 3, "degraded", otherwise).
#                                  reconcile treats both as failures; health-check.sh warns on 3, so a
#                                  stale backup never blocks a rollback or a candidate resolution.
#   backup-postgres.sh run <job>   one sidecar job now: base | dump | prune | preflight | status.
#
# Without the PostgreSQL secrets (oteryn_postgres_secret_state absent) postgres is not started and every
# mode is a no-op. The spool is the postgres_backup volume; nothing here removes a volume or a backup
# outside the sidecar's retention.
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd)"
ENV_FILE="${OTERYN_ENV_FILE:-$DEPLOY_DIR/.env}"
COMPOSE_FILE="$DEPLOY_DIR/compose.yml"

# shellcheck source=deploy/synology/scripts/lib.sh
source "$SCRIPT_DIR/lib.sh"
load_oteryn_env_file "$ENV_FILE"

compose=(docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE" --profile postgres)
JOB=(bash /bootstrap/postgres-backup/backup-job.sh)
SPOOL=/var/lib/oteryn-postgres-backup
MAX_BACKUP_AGE_SECONDS="${OTERYN_BACKUP_MAX_AGE_SECONDS:-93600}"
MAX_READY_AGE_SECONDS="${OTERYN_ARCHIVE_MAX_READY_AGE_SECONDS:-600}"
READY_TIMEOUT_SECONDS="${OTERYN_BACKUP_READY_TIMEOUT_SECONDS:-120}"
ARCHIVE_TIMEOUT_SECONDS="${OTERYN_ARCHIVE_WAIT_SECONDS:-180}"
# How long a deploy waits for the base backup and dump it starts (the deploy job has 30 minutes in all).
ENSURE_BUDGET_SECONDS="${OTERYN_BACKUP_ENSURE_BUDGET_SECONDS:-300}"
ENSURE_POLL_SECONDS=5

fail() {
    echo "PostgreSQL backup: $*" >&2
    exit 1
}

for value in "$MAX_BACKUP_AGE_SECONDS" "$MAX_READY_AGE_SECONDS" "$READY_TIMEOUT_SECONDS" "$ARCHIVE_TIMEOUT_SECONDS" \
    "$ENSURE_BUDGET_SECONDS"; do
    [[ "$value" =~ ^[1-9][0-9]{0,6}$ ]] || fail "timeouts and ages must be positive decimal integers of at most seven digits"
done

secrets_configured() {
    local state
    state="$(oteryn_postgres_secret_state)" || exit 1
    [[ "$state" == configured ]]
}

container_of() {
    "${compose[@]}" ps -q "$1"
}

superuser_sql() {
    "${compose[@]}" exec -T -u postgres postgres psql -X -q -t -A -v ON_ERROR_STOP=1 -d postgres -c "$1"
}

sidecar_job() {
    "${compose[@]}" exec -T -u postgres postgres-backup "${JOB[@]}" "$@"
}

wait_sidecar_healthy() {
    local container deadline status
    deadline=$((SECONDS + READY_TIMEOUT_SECONDS))
    while :; do
        container="$(container_of postgres-backup)"
        status=missing
        if [[ -n "$container" ]]; then
            status="$(docker inspect --format '{{if .State.Running}}{{if .State.Health}}{{.State.Health.Status}}{{else}}none{{end}}{{else}}stopped{{end}}' "$container")"
        fi
        [[ "$status" == healthy ]] && return 0
        if (( SECONDS >= deadline )); then
            "${compose[@]}" logs --no-color --tail 50 postgres-backup >&2 || true
            fail "the postgres-backup sidecar is $status, not healthy"
        fi
        sleep 3
    done
}

status_value() { sed -n "s/^$1=//p" <<< "$2" | head -n 1; }

# Starts `ensure` detached in the sidecar and waits for it at most ENSURE_BUDGET_SECONDS. A base backup
# can wait up to two hours for the job lock and pg_basebackup or pg_dump has no time limit, so the deploy
# never waits for them unbounded. The job writes "<token> <exit status>" to status/ensure.result when it
# ends and logs to status/ensure.log. Returns 0 when it succeeded within the budget and 1 when it is
# still running after the budget; a failed job fails the reconcile.
ensure_within_budget() {
    local token result deadline
    token="$(date -u +%Y%m%dT%H%M%SZ)-$$-$RANDOM"
    # shellcheck disable=SC2016  # the inner script expands its own positional arguments in the sidecar
    "${compose[@]}" exec -d -u postgres postgres-backup bash -c '
        status=0
        "${@:4}" ensure > "$3" 2>&1 || status=$?
        printf "%s %s\n" "$1" "$status" > "$2.tmp" && mv -f -- "$2.tmp" "$2"
    ' oteryn-ensure "$token" "$SPOOL/status/ensure.result" "$SPOOL/status/ensure.log" "${JOB[@]}" \
        || fail "cannot start the base backup and logical dump in the postgres-backup sidecar"
    deadline=$((SECONDS + ENSURE_BUDGET_SECONDS))
    while :; do
        result="$("${compose[@]}" exec -T -u postgres postgres-backup cat "$SPOOL/status/ensure.result" 2>/dev/null || true)"
        if [[ "$result" == "$token "* ]]; then
            [[ "$result" == "$token 0" ]] && return 0
            "${compose[@]}" exec -T -u postgres postgres-backup tail -n 40 "$SPOOL/status/ensure.log" >&2 2>/dev/null || true
            fail "the base backup or logical dump failed (ensure exit ${result#"$token "}; log above and in $SPOOL/status/ensure.log)"
        fi
        (( SECONDS < deadline )) || return 1
        sleep "$ENSURE_POLL_SECONDS"
    done
}

check() {
    local container output mode last_archived last_failed ready_age status age kind free required live_id spool_id
    local -a degraded=()

    container="$(container_of postgres-backup)"
    [[ -n "$container" && "$(docker inspect --format '{{.State.Running}}' "$container")" == true ]] \
        || fail "Service is not running: postgres-backup"
    [[ "$(docker inspect --format '{{len .HostConfig.PortBindings}}' "$container")" == 0 ]] \
        || fail "the postgres-backup sidecar must publish no host port"
    wait_sidecar_healthy

    output="$(superuser_sql "SELECT current_setting('archive_mode'),
            coalesce(extract(epoch FROM a.last_archived_time)::bigint, 0),
            coalesce(extract(epoch FROM a.last_failed_time)::bigint, 0),
            coalesce((SELECT extract(epoch FROM now() - min(s.modification))::bigint
                      FROM pg_ls_archive_statusdir() s WHERE s.name LIKE '%.ready'), 0)
        FROM pg_stat_archiver a")" || fail "cannot read pg_stat_archiver"
    IFS='|' read -r mode last_archived last_failed ready_age <<< "$output"
    [[ "$mode" == on ]] || fail "archive_mode is $mode, not on"
    [[ "$last_archived" =~ ^[0-9]+$ && "$last_failed" =~ ^[0-9]+$ && "$ready_age" =~ ^-?[0-9]+$ ]] \
        || fail "unexpected pg_stat_archiver values"
    if (( last_failed > last_archived )); then
        degraded+=("the latest WAL archive attempt failed (pg_stat_archiver; see the postgres container log)")
    fi
    (( ready_age <= MAX_READY_AGE_SECONDS )) \
        || degraded+=("a WAL segment has waited ${ready_age}s for archiving (limit ${MAX_READY_AGE_SECONDS}s)")

    status="$(sidecar_job status)" || fail "the backup status job failed"
    for kind in base dump; do
        age="$(status_value "${kind}_age_seconds" "$status")"
        if [[ ! "$age" =~ ^[0-9]+$ ]]; then
            degraded+=("no complete $kind backup exists in the NAS spool")
        elif (( age > MAX_BACKUP_AGE_SECONDS )); then
            degraded+=("the newest $kind backup is ${age}s old (limit ${MAX_BACKUP_AGE_SECONDS}s)")
        fi
    done
    # A re-initialised server is another cluster: its WAL names collide with the archived ones and the
    # spool's base backups cannot restore it (runbook: retire the spool content of the old cluster).
    live_id="$(superuser_sql 'SELECT system_identifier FROM pg_control_system()')" || fail "cannot read the system identifier"
    spool_id="$(status_value base_system_identifier "$status")"
    if [[ -n "$spool_id" && "$spool_id" != "$live_id" ]]; then
        degraded+=("the newest base backup belongs to cluster $spool_id, the live server is cluster $live_id")
    fi
    free="$(status_value spool_free_mb "$status")"
    required="$(status_value required_free_mb "$status")"
    if [[ "$(status_value result "$status")" != ok ]]; then
        degraded+=("the NAS spool preflight is $(status_value result "$status") (${free:-?} MiB free, ${required:-?} MiB required)")
    fi
    if (( ${#degraded[@]} )); then
        printf 'PostgreSQL backup degraded: %s\n' "${degraded[@]}" >&2
        echo "PostgreSQL backup: see docs/operations/PLATFORM_POSTGRES_BACKUP_PITR.md" >&2
        exit 3
    fi
    echo "Verified backups: WAL archive current, base $(status_value base_newest "$status"), dump $(status_value dump_newest "$status"), spool ${free} MiB free of ${required} MiB required"
}

# Starts the sidecar, or recreates it when the staged job script differs from the private copy its
# loop runs (backup-job.sh service), so every deploy runs the reviewed script.
start_sidecar() {
    "${compose[@]}" up -d --no-deps postgres-backup || fail "cannot start the postgres-backup sidecar"
    # Healthy means the loop of this container wrote its heartbeat, so its private copy exists.
    wait_sidecar_healthy
    if ! "${compose[@]}" exec -T postgres-backup cmp -s /bootstrap/postgres-backup/backup-job.sh /tmp/oteryn-backup-job.sh; then
        echo "PostgreSQL backup: the job script changed; recreating the postgres-backup sidecar"
        "${compose[@]}" up -d --no-deps --force-recreate postgres-backup || fail "cannot recreate the postgres-backup sidecar"
        wait_sidecar_healthy
    fi
}

reconcile() {
    local output segment deadline archived result live_id spool_id
    start_sidecar

    # The role passwords travel on standard input into a 0600 file inside the sidecar.
    printf '%s\n%s\n' "$PLATFORM_POSTGRES_BACKUP_PASSWORD" "$PLATFORM_POSTGRES_DUMP_PASSWORD" \
        | sidecar_job credentials || fail "cannot provision the backup role credentials"

    if ! output="$(sidecar_job preflight)"; then
        printf '%s\n' "$output" >&2
        result="$(status_value result "$output")"
        case "$result" in
            refused) fail "refusing: the NAS spool (postgres_backup volume) lacks the free space a backup needs; see docs/operations/PLATFORM_POSTGRES_BACKUP_PITR.md" ;;
            database_unreachable) fail "the dump role cannot read the database size over verify-full TLS" ;;
            credentials_missing) fail "the backup role credentials are not provisioned in the sidecar" ;;
            *) fail "the spool preflight failed (${result:-no result}); see the postgres-backup container log" ;;
        esac
    fi
    echo "PostgreSQL backup spool preflight: $(tr '\n' ' ' <<< "$output")"

    # A re-initialised server is another cluster: archive-wal.sh refuses its colliding WAL names.
    live_id="$(superuser_sql 'SELECT system_identifier FROM pg_control_system()')" || fail "cannot read the system identifier"
    spool_id="$(status_value base_system_identifier "$(sidecar_job status)")"
    if [[ -n "$spool_id" && "$spool_id" != "$live_id" ]]; then
        fail "the NAS spool holds backups of cluster $spool_id, the live server is cluster $live_id; retire the old spool content (docs/operations/PLATFORM_POSTGRES_BACKUP_PITR.md, Another cluster)"
    fi

    [[ "$(superuser_sql "SELECT current_setting('archive_mode')")" == on ]] || fail "archive_mode is not on"
    superuser_sql 'SELECT pg_current_xact_id()' > /dev/null || fail "cannot write a WAL record"
    segment="$(superuser_sql 'SELECT pg_walfile_name(pg_switch_wal())')" || fail "pg_switch_wal failed"
    [[ "$segment" =~ ^[0-9A-F]{24}$ ]] || fail "unexpected WAL segment name: $segment"
    deadline=$((SECONDS + ARCHIVE_TIMEOUT_SECONDS))
    until "${compose[@]}" exec -T -u postgres postgres-backup test -s "$SPOOL/wal/$segment.gz" 2>/dev/null; do
        if (( SECONDS >= deadline )); then
            archived="$(superuser_sql 'SELECT coalesce(last_archived_wal, '"'none'"'), failed_count, coalesce(last_failed_wal, '"'none'"') FROM pg_stat_archiver' || true)"
            fail "forced WAL segment $segment did not reach the spool within ${ARCHIVE_TIMEOUT_SECONDS}s (archiver: $archived)"
        fi
        sleep 2
    done
    echo "Verified WAL archive: forced segment $segment is in the NAS spool"

    if ensure_within_budget; then
        # Strict at deploy: a degraded backup (exit 3) fails the reconcile like a structural failure.
        check
        return
    fi
    # Still running after the budget: the jobs hold the spool job lock and finish in the sidecar. The deploy
    # goes on with the backups reported degraded (health check warning; the next deploy takes the full path
    # and reconciles again). A structural failure still fails the reconcile.
    echo "PostgreSQL backup: the base backup or logical dump is still running in the postgres-backup sidecar after ${ENSURE_BUDGET_SECONDS}s; it finishes in the background (log: $SPOOL/status/ensure.log)." >&2
    # A separate process, so errexit stays in force inside the check.
    local status=0
    OTERYN_ENV_FILE="$ENV_FILE" bash "$SCRIPT_DIR/backup-postgres.sh" check || status=$?
    (( status == 0 || status == 3 )) || exit "$status"
}

mode="${1:-}"
case "$mode" in
    reconcile|check|run) ;;
    *) fail "usage: backup-postgres.sh reconcile | check | run <base|dump|prune|preflight|status>" ;;
esac

if ! secrets_configured; then
    echo "PostgreSQL secrets are not configured; backups are not running."
    exit 0
fi

case "$mode" in
    reconcile) reconcile ;;
    check) check ;;
    run)
        job="${2:-}"
        case "$job" in
            base|dump|prune|preflight|status) sidecar_job "$job" ;;
            *) fail "run expects base, dump, prune, preflight or status" ;;
        esac
        ;;
esac
