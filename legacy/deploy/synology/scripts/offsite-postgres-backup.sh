#!/usr/bin/env bash
# Encrypted off-NAS copy of the Platform PostgreSQL backup spool (ADR 0046 §7, review 2026-10-12 F-02,
# runbook docs/operations/PLATFORM_POSTGRES_BACKUP_PITR.md "Off-NAS copy"). The spool (base backups, WAL
# archive, logical dumps of the postgres_backup volume) is copied with restic, which encrypts and
# authenticates every object on the NAS before it leaves (AES-256 + Poly1305): the destination stores
# ciphertext only. The destination is configuration: any restic repository URL (s3:, b2:, azure:, gs:,
# sftp:, rest:). Neither a provider nor a secret is in the repository.
#
#   offsite-postgres-backup.sh init     create the encrypted repository at the destination (once)
#   offsite-postgres-backup.sh push     snapshot the spool to the destination (DSM Task Scheduler, every
#                                       minute: off-NAS lag = archive_timeout + schedule + push time)
#   offsite-postgres-backup.sh forget   retention: thin the snapshots and prune unreferenced data (daily)
#   offsite-postgres-backup.sh check    newest snapshot at the destination within the target RPO, then
#                                       restic check with a random data subset read back (weekly)
#   offsite-postgres-backup.sh status   the last local push result, without the network (exit 3 if stale)
#   offsite-postgres-backup.sh restore VOLUME [--snapshot ID]
#                                       restore a snapshot into a new Docker volume shaped like the spool,
#                                       for restore-postgres-pitr.sh --offline --spool-volume VOLUME
#   offsite-postgres-backup.sh drill [--snapshot ID] [--skip-platform-checks]
#                                       restore drill from the off-NAS copy: restore into a scratch volume,
#                                       restore-postgres-pitr.sh --offline --target latest from it, evidence
#                                       against RTO 4h (monthly). The scratch volume is the anonymous volume
#                                       of a --rm holder container: stopping the holder discards it, and
#                                       nothing here removes a volume (the restore VOLUME mode keeps its own)
#
# Configuration: an owner-maintained KEY=VALUE file outside the repository and outside the spool, named by
# OTERYN_OFFSITE_CONFIG (template deploy/synology/offsite-backup.env.example). It names the repository URL,
# the encryption password file and the provider credentials file; those two files are root-only (mode 0600
# or stricter) and the password also has a copy outside the NAS (without it the copy cannot be read).
# Exit status: 0 ok, 1 failure, 2 usage or configuration, 3 degraded (stale copy). Containers are found by
# Compose labels of COMPOSE_PROJECT_NAME (default oteryn-staging). Runs as root (Docker on the NAS).
set -euo pipefail
export LC_ALL=C

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=deploy/synology/scripts/lib.sh
source "$SCRIPT_DIR/lib.sh"

# restic 0.18.0, pinned by digest like every image on the NAS.
RESTIC_IMAGE='restic/restic:0.18.0@sha256:4cf4a61ef9786f4de53e9de8c8f5c040f33830eb0a10bf3d614410ee2fcb6120'
SPOOL=/var/lib/oteryn-postgres-backup
# The postgres image's postgres user owns the spool (backup-job.sh, archive-wal.sh).
SPOOL_UID=999
RTO_SECONDS=14400
TAG=postgres-spool
PITR="$SCRIPT_DIR/restore-postgres-pitr.sh"

usage() {
    echo "usage: offsite-postgres-backup.sh init | push | forget | check | status" >&2
    echo "       offsite-postgres-backup.sh restore VOLUME [--snapshot ID]" >&2
    echo "       offsite-postgres-backup.sh drill [--snapshot ID] [--skip-platform-checks]" >&2
    exit 2
}

log() { printf '%s offsite-backup %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$*" >&2; }
fail() { log "error: $*"; exit 1; }
config_error() { log "configuration: $*"; exit 2; }

mode="${1:-}"
(( $# )) && shift
case "$mode" in
    init|push|forget|check|status|restore|drill) ;;
    *) usage ;;
esac

restore_volume=''
snapshot=latest
pitr_args=()
if [[ "$mode" == restore ]]; then
    restore_volume="${1:-}"
    (( $# )) && shift
    [[ "$restore_volume" =~ ^[A-Za-z0-9][A-Za-z0-9_.-]{0,254}$ ]] || usage
fi
while (( $# )); do
    case "$1" in
        --snapshot) [[ "$mode" == restore || "$mode" == drill ]] || usage; snapshot="${2:-}"; (( $# >= 2 )) || usage; shift 2 ;;
        --skip-platform-checks) [[ "$mode" == drill ]] || usage; pitr_args+=(--skip-platform-checks); shift ;;
        *) usage ;;
    esac
done
# A restic snapshot ID (8 to 64 hex digits) or latest; it reaches the restic command line.
[[ "$snapshot" == latest || "$snapshot" =~ ^[0-9a-f]{8,64}$ ]] || usage

PROJECT="${COMPOSE_PROJECT_NAME:-oteryn-staging}"
[[ "$PROJECT" =~ ^[a-z0-9][a-z0-9_-]{0,62}$ ]] || config_error "COMPOSE_PROJECT_NAME must be a Compose project name"

[[ -n "${OTERYN_OFFSITE_CONFIG:-}" ]] || config_error "OTERYN_OFFSITE_CONFIG names no configuration file"
# <variable> <path>: a root-only secret file, never a link, never readable by group or others.
secret_file() {
    local mode
    [[ "$2" == /* && "$2" != *[[:space:],]* ]] || config_error "$1 must be an absolute path without spaces or commas"
    [[ -f "$2" && ! -L "$2" ]] || config_error "$1: $2 is not a regular file"
    [[ "$(stat -c %u -- "$2")" == "$(id -u)" ]] || config_error "$1: $2 is not owned by $(id -un)"
    mode="$(stat -c %a -- "$2")"
    [[ "$mode" =~ ^[0-7]?[0-7]00$ ]] || config_error "$1: $2 has mode $mode, expected 0600 or stricter"
}
config_file="$OTERYN_OFFSITE_CONFIG"
# The script runs as root: the configuration is root-only too, and sets only its own keys, never a global
# of this script (RESTIC_IMAGE, PITR, ...) or of the environment.
secret_file OTERYN_OFFSITE_CONFIG "$config_file"
CONFIG_KEYS=" OTERYN_OFFSITE_REPOSITORY OTERYN_OFFSITE_PASSWORD_FILE OTERYN_OFFSITE_CREDENTIALS_FILE OTERYN_OFFSITE_STATE_DIR"
CONFIG_KEYS+=" OTERYN_OFFSITE_TARGET_RPO_SECONDS OTERYN_OFFSITE_KEEP_WITHIN_HOURLY OTERYN_OFFSITE_KEEP_WITHIN_DAILY"
CONFIG_KEYS+=" OTERYN_OFFSITE_KEEP_WITHIN_WEEKLY OTERYN_OFFSITE_CHECK_SUBSET OTERYN_OFFSITE_LOCK_RETRY OTERYN_OFFSITE_SPOOL_VOLUME "
while IFS= read -r config_line || [[ -n "$config_line" ]]; do
    config_line="${config_line%$'\r'}"
    [[ -z "$config_line" || "$config_line" =~ ^[[:space:]]*# ]] && continue
    [[ "$CONFIG_KEYS" == *" ${config_line%%=*} "* ]] || config_error "$config_file sets ${config_line%%=*}, which is not an OTERYN_OFFSITE_ setting"
done < "$config_file"
load_oteryn_env_file "$config_file" || config_error "cannot read $config_file"

REPOSITORY="${OTERYN_OFFSITE_REPOSITORY:-}"
PASSWORD_FILE="${OTERYN_OFFSITE_PASSWORD_FILE:-}"
CREDENTIALS_FILE="${OTERYN_OFFSITE_CREDENTIALS_FILE:-}"
STATE_DIR="${OTERYN_OFFSITE_STATE_DIR:-}"
TARGET_RPO_SECONDS="${OTERYN_OFFSITE_TARGET_RPO_SECONDS:-300}"
KEEP_WITHIN_HOURLY="${OTERYN_OFFSITE_KEEP_WITHIN_HOURLY:-48h}"
KEEP_WITHIN_DAILY="${OTERYN_OFFSITE_KEEP_WITHIN_DAILY:-14d}"
KEEP_WITHIN_WEEKLY="${OTERYN_OFFSITE_KEEP_WITHIN_WEEKLY:-56d}"
CHECK_SUBSET="${OTERYN_OFFSITE_CHECK_SUBSET:-5%}"
LOCK_RETRY="${OTERYN_OFFSITE_LOCK_RETRY:-5m}"
SPOOL_VOLUME="${OTERYN_OFFSITE_SPOOL_VOLUME:-}"

# A restic URL with a scheme (or an absolute local path, for tests and a mounted second disk). Credentials
# never ride in the URL: they belong to the credentials file (RESTIC_REST_USERNAME, AWS_*, B2_*, ...).
[[ "$REPOSITORY" =~ ^(s3|b2|azure|gs|sftp|rest|swift):[^[:space:]]+$ || "$REPOSITORY" =~ ^/[^[:space:]]+$ ]] \
    || config_error "OTERYN_OFFSITE_REPOSITORY must be a restic repository URL (s3:, b2:, azure:, gs:, sftp:, rest:, swift:)"
[[ ! "$REPOSITORY" =~ ://[^/@]*:[^/@]*@ ]] \
    || config_error "OTERYN_OFFSITE_REPOSITORY carries a password: put it in the credentials file"
# A NAS-local copy (owner decision D-3) lives on its own shared folder: never in the Docker data root, where
# the live database and the spool volumes are.
[[ ! "$REPOSITORY" =~ ^(/volume[0-9]+/@docker|/var/lib/docker)(/|$) ]] \
    || config_error "OTERYN_OFFSITE_REPOSITORY is inside the Docker data root: use a separate shared folder"
[[ "$TARGET_RPO_SECONDS" =~ ^[1-9][0-9]{0,6}$ ]] || config_error "OTERYN_OFFSITE_TARGET_RPO_SECONDS must be a positive integer"
for value in "$KEEP_WITHIN_HOURLY" "$KEEP_WITHIN_DAILY" "$KEEP_WITHIN_WEEKLY"; do
    [[ "$value" =~ ^([1-9][0-9]{0,3}[ymdh])+$ ]] || config_error "retention durations are restic durations such as 48h, 14d or 56d"
done
[[ "$CHECK_SUBSET" =~ ^((100|[1-9][0-9]?(\.[0-9]+)?)%|[1-9][0-9]{0,3}/[1-9][0-9]{0,3})$ ]] \
    || config_error "OTERYN_OFFSITE_CHECK_SUBSET must be a percentage such as 5% or n/t"
[[ "$LOCK_RETRY" =~ ^[1-9][0-9]{0,3}[smh]$ ]] || config_error "OTERYN_OFFSITE_LOCK_RETRY must be a duration such as 5m"
[[ -z "$SPOOL_VOLUME" || "$SPOOL_VOLUME" =~ ^[A-Za-z0-9][A-Za-z0-9_.-]{0,254}$ ]] \
    || config_error "OTERYN_OFFSITE_SPOOL_VOLUME must be a Docker volume name"
[[ "$STATE_DIR" == /* && "$STATE_DIR" != *[[:space:]]* ]] || config_error "OTERYN_OFFSITE_STATE_DIR must be an absolute path"

secret_file OTERYN_OFFSITE_PASSWORD_FILE "$PASSWORD_FILE"
[[ -z "$CREDENTIALS_FILE" ]] || secret_file OTERYN_OFFSITE_CREDENTIALS_FILE "$CREDENTIALS_FILE"
[[ "$(head -c 64 -- "$PASSWORD_FILE" | tr -d '\n' | wc -c)" -ge 32 ]] \
    || config_error "OTERYN_OFFSITE_PASSWORD_FILE holds fewer than 32 characters"

mkdir -p -- "$STATE_DIR/status" "$STATE_DIR/cache" "$STATE_DIR/drills"
chmod 0700 -- "$STATE_DIR"
STATUS_FILE="$STATE_DIR/status/push.last"

# The spool volume: configured, or the one the postgres-backup (or postgres) container mounts at $SPOOL.
spool_volume() {
    local container service ids
    if [[ -n "$SPOOL_VOLUME" ]]; then
        printf '%s' "$SPOOL_VOLUME"
        return 0
    fi
    for service in postgres-backup postgres; do
        ids="$(docker ps -a -q --filter "label=com.docker.compose.project=$PROJECT" --filter "label=com.docker.compose.service=$service")"
        container="$(head -n 1 <<< "$ids")"
        [[ -n "$container" ]] || continue
        docker inspect --format '{{range .Mounts}}{{if eq .Destination "'"$SPOOL"'"}}{{.Name}}{{end}}{{end}}' "$container"
        return 0
    done
}

# <docker run options...> -- <restic arguments...>: one restic run with the repository, the password file
# (read-only, never in the environment) and the provider credentials. Root in the container with only the
# capabilities a run needs; the spool and the password are mounted read-only.
restic_run() {
    local -a opts=()
    while [[ "$1" != -- ]]; do opts+=("$1"); shift; done
    shift
    local -a creds=()
    [[ -z "$CREDENTIALS_FILE" ]] || creds=(--env-file "$CREDENTIALS_FILE")
    local -a repo_mount=()
    # A local repository (a mounted second disk): the path itself, the same inside the container.
    [[ "$REPOSITORY" != /* ]] || repo_mount=(--mount "type=bind,source=$REPOSITORY,target=$REPOSITORY")
    docker run --rm -i --cap-drop ALL --security-opt no-new-privileges \
        --mount "type=bind,source=$PASSWORD_FILE,target=/run/oteryn/restic-password,readonly" \
        --mount "type=bind,source=$STATE_DIR/cache,target=/cache" \
        "${repo_mount[@]}" "${creds[@]}" \
        -e RESTIC_REPOSITORY="$REPOSITORY" -e RESTIC_PASSWORD_FILE=/run/oteryn/restic-password \
        -e RESTIC_CACHE_DIR=/cache \
        "${opts[@]}" "$RESTIC_IMAGE" "$@"
}

write_status() {
    local tmp="$STATUS_FILE.tmp.$$"
    printf 'result=%s\nepoch=%s\ntime=%s\nsnapshot=%s\nduration_seconds=%s\nspool_volume=%s\n' "$@" > "$tmp"
    mv -f -- "$tmp" "$STATUS_FILE"
}

status_value() { sed -n "s/^$1=//p" "$STATUS_FILE" 2>/dev/null | head -n 1; }

run_init() {
    local rc=0
    [[ "$REPOSITORY" != /* ]] || mkdir -p -- "$REPOSITORY"
    restic_run -- cat config > /dev/null 2>&1 || rc=$?
    if (( rc == 0 )); then
        log "repository already initialised at the destination"
        return 0
    fi
    # 10: no repository at that location (restic 0.17 and later). Anything else (wrong password 12, network)
    # must not end in a second repository beside an unreadable one.
    (( rc == 10 )) || fail "cannot read the destination (restic exit $rc): check the password and credentials files"
    restic_run -- init > /dev/null
    log "encrypted repository created at the destination; store the password file off the NAS now"
}

run_push() {
    local volume started rc output snapshot_id attempt
    volume="$(spool_volume)"
    [[ "$volume" =~ ^[A-Za-z0-9][A-Za-z0-9_.-]{0,254}$ ]] \
        || fail "no backup spool volume (no postgres-backup container in project $PROJECT, or set OTERYN_OFFSITE_SPOOL_VOLUME)"
    # A push still running (a slow upload) keeps its slot: this run is skipped, not queued, and the freshness
    # checks report the lag.
    exec 8> "$STATE_DIR/status/push.lock"
    if ! flock -n 8; then
        log "previous push still running; skipped"
        return 0
    fi
    started="$(date +%s)"
    # No spool job lock: every file of the spool appears by an atomic rename (WAL .tmp files, .partial
    # backup directories, both excluded) and only retention removes files, under its own job lock. A file
    # that retention removes while restic reads it ends the snapshot as incomplete (exit 3); it was in the
    # previous snapshots already, and one immediate retry takes a complete one. A push never waits behind a
    # long base backup, so the WAL keeps leaving the NAS during it.
    for attempt in 1 2; do
        rc=0
        output="$(restic_run --cap-add DAC_READ_SEARCH \
            --mount "type=volume,source=$volume,target=/spool,readonly" -- \
            backup --json --quiet --retry-lock "$LOCK_RETRY" --host "$PROJECT" --tag "$TAG" \
            --exclude '.*.partial' --exclude '.*.tmp.*' \
            /spool/base /spool/wal /spool/dump)" || rc=$?
        (( rc == 3 && attempt == 1 )) || break
        log "spool changed during the snapshot (retention); retrying once"
    done
    if (( rc != 0 )); then
        write_status failed "$(date +%s)" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" none "$(( $(date +%s) - started ))" "$volume"
        fail "restic backup exited $rc"
    fi
    snapshot_id="$(grep '"message_type":"summary"' <<< "$output" | grep -o '"snapshot_id":"[0-9a-f]*"' | cut -d '"' -f 4 | tail -n 1)"
    [[ "$snapshot_id" =~ ^[0-9a-f]{8,64}$ ]] || fail "restic backup reported no snapshot"
    write_status ok "$started" "$(date -u -d "@$started" +%Y-%m-%dT%H:%M:%SZ)" "$snapshot_id" "$(( $(date +%s) - started ))" "$volume"
    log "snapshot ${snapshot_id:0:8} at the destination ($(( $(date +%s) - started ))s)"
}

run_forget() {
    # Snapshots of this host and tag only. Every snapshot is the whole spool (7 days of WAL and the base
    # backups it needs), so thinning snapshots keeps point-in-time recovery for each kept one; restic
    # deduplicates, the destination stores each base backup and WAL segment once.
    restic_run -- forget --retry-lock "$LOCK_RETRY" --host "$PROJECT" --tag "$TAG" --group-by host,tags \
        --keep-last 1 --keep-within-hourly "$KEEP_WITHIN_HOURLY" --keep-within-daily "$KEEP_WITHIN_DAILY" \
        --keep-within-weekly "$KEEP_WITHIN_WEEKLY" --prune --quiet \
        || fail "restic forget --prune failed"
    log "retention applied (hourly $KEEP_WITHIN_HOURLY, daily $KEEP_WITHIN_DAILY, weekly $KEEP_WITHIN_WEEKLY)"
}

# The newest snapshot of this host at the destination: "<epoch> <id>", read from the destination itself.
latest_snapshot() {
    local json time id
    json="$(restic_run -- snapshots --json --latest 1 --host "$PROJECT" --tag "$TAG")" || return 1
    time="$(grep -o '"time":"[^"]*"' <<< "$json" | head -n 1 | cut -d '"' -f 4)"
    id="$(grep -o '"id":"[0-9a-f]*"' <<< "$json" | head -n 1 | cut -d '"' -f 4)"
    [[ -n "$time" && "$id" =~ ^[0-9a-f]{8,64}$ ]] || return 1
    printf '%s %s\n' "$(date -u -d "$time" +%s)" "$id"
}

run_check() {
    local newest epoch id age degraded=false
    newest="$(latest_snapshot)" || fail "no snapshot of $PROJECT at the destination (or the destination is unreadable)"
    read -r epoch id <<< "$newest"
    age=$(( $(date +%s) - epoch ))
    if (( age > TARGET_RPO_SECONDS )); then
        log "degraded: the newest snapshot ${id:0:8} is ${age}s old, above the target RPO of ${TARGET_RPO_SECONDS}s"
        degraded=true
    else
        log "newest snapshot ${id:0:8} is ${age}s old (target RPO ${TARGET_RPO_SECONDS}s)"
    fi
    restic_run -- check --retry-lock "$LOCK_RETRY" --read-data-subset "$CHECK_SUBSET" > /dev/null \
        || fail "restic check failed: the copy at the destination is damaged"
    log "restic check passed (structure and $CHECK_SUBSET of the data read back and authenticated)"
    [[ "$degraded" == false ]] || exit 3
}

run_status() {
    local result epoch age
    [[ -f "$STATUS_FILE" ]] || { log "degraded: no push recorded yet"; exit 3; }
    result="$(status_value result)"
    epoch="$(status_value epoch)"
    [[ "$epoch" =~ ^[0-9]{1,12}$ ]] || fail "unreadable $STATUS_FILE"
    age=$(( $(date +%s) - epoch ))
    cat -- "$STATUS_FILE"
    echo "age_seconds=$age"
    echo "target_rpo_seconds=$TARGET_RPO_SECONDS"
    if [[ "$result" != ok ]]; then
        log "degraded: the last push failed"
        exit 3
    fi
    if (( age > TARGET_RPO_SECONDS )); then
        log "degraded: the last successful push started ${age}s ago, above the target RPO of ${TARGET_RPO_SECONDS}s"
        exit 3
    fi
}

# <volume> <snapshot>: restore into an empty volume, verify every restored file against the snapshot, then
# give it the spool's job lock (the scratch drill instance takes it shared; status/ is not copied).
restore_into() {
    local volume="$1" snap="$2"
    restic_run --cap-add CHOWN --cap-add DAC_OVERRIDE --cap-add FOWNER --cap-add DAC_READ_SEARCH \
        --mount "type=volume,source=$volume,target=/restore" -- \
        restore "$snap:/spool" --host "$PROJECT" --tag "$TAG" --target /restore --verify --quiet \
        || fail "restic restore of $snap into $volume failed or did not verify"
    docker run --rm --cap-drop ALL --cap-add CHOWN --cap-add DAC_OVERRIDE --security-opt no-new-privileges \
        --mount "type=volume,source=$volume,target=/restore" --entrypoint sh "$RESTIC_IMAGE" -c \
        "set -e; [ -d /restore/base ] && [ -d /restore/wal ]; mkdir -p -m 0700 /restore/status; : > /restore/status/job.lock; chown -R $SPOOL_UID:$SPOOL_UID /restore/status" \
        || fail "restored volume $volume has no base/ and wal/, or its job lock cannot be created"
}

run_restore() {
    ! docker volume inspect "$restore_volume" > /dev/null 2>&1 || fail "Docker volume $restore_volume already exists"
    docker volume create --label oteryn.offsite-restore=true "$restore_volume" > /dev/null
    restore_into "$restore_volume" "$snapshot"
    log "snapshot $snapshot restored into volume $restore_volume; next: restore-postgres-pitr.sh --offline --spool-volume $restore_volume"
}

run_drill() {
    # Globals, not locals: the EXIT trap runs after this function has returned.
    stamp='' dir='' holder='' volume='' newest='' epoch='' id='' restored_epoch='' pitr_rc=0 total=0 result=failed
    holder_created=false
    [[ -f "$PITR" ]] || fail "missing $PITR"
    stamp="$(date -u +%Y%m%dT%H%M%SZ)"
    dir="$STATE_DIR/drills/$stamp"
    holder="oteryn-offsite-drill-$stamp"
    mkdir -p -- "$dir"
    started="$(date +%s)"
    # The evidence is written whatever happens. Stopping the holder removes it (--rm) with its anonymous
    # volume; a volume left behind fails the drill.
    # shellcheck disable=SC2329 # invoked by the EXIT trap
    drill_exit() {
        local status=$? deadline
        if [[ "$holder_created" == true ]]; then
            docker stop -t 10 "$holder" > /dev/null 2>&1 || true
            deadline=$((SECONDS + 60))
            while { docker container inspect "$holder" > /dev/null 2>&1 \
                    || { [[ -n "$volume" ]] && docker volume inspect "$volume" > /dev/null 2>&1; }; } && (( SECONDS < deadline )); do
                sleep 1
            done
            if [[ -n "$volume" ]] && docker volume inspect "$volume" > /dev/null 2>&1; then
                log "scratch volume $volume is still present (anonymous volume of $holder)"
                status=1
            fi
        fi
        total=$(( $(date +%s) - started ))
        (( status != 0 )) || result=passed
        {
            printf 'DRILL=%s\nPROJECT=%s\nREPOSITORY_SCHEME=%s\nSNAPSHOT=%s\nSNAPSHOT_TIME=%s\n' \
                "$stamp" "$PROJECT" "$([[ "$REPOSITORY" == /* ]] && echo local || echo "${REPOSITORY%%:*}")" "${id:-none}" "${epoch:+$(date -u -d "@$epoch" +%Y-%m-%dT%H:%M:%SZ)}"
            printf 'DOWNLOAD_SECONDS=%s\nPITR_EXIT=%s\nTOTAL_SECONDS=%s\nRTO_SECONDS=%s\nSCRATCH_VOLUME=%s\nRESULT=%s\n' \
                "${restored_epoch:+$(( restored_epoch - started ))}" "$pitr_rc" "$total" "$RTO_SECONDS" \
                "${volume:-none}" "$result"
        } > "$dir/offsite.env" || status=1
        log "off-NAS drill $result in ${total}s; evidence $dir/offsite.env and $dir/pitr/*/evidence.env"
        exit "$status"
    }
    trap drill_exit EXIT
    trap 'exit 143' TERM INT
    if [[ "$snapshot" == latest ]]; then
        newest="$(latest_snapshot)" || fail "no snapshot of $PROJECT at the destination"
        read -r epoch id <<< "$newest"
    else
        id="$snapshot"
    fi
    # The holder only keeps the scratch volume alive; it has no network and no capability.
    docker create --name "$holder" --rm --label oteryn.offsite-drill=true --network none --cap-drop ALL \
        --security-opt no-new-privileges --mount type=volume,target=/restore --entrypoint sh "$RESTIC_IMAGE" \
        -c 'trap "exit 0" TERM; while :; do sleep 5 & wait $!; done' > /dev/null \
        || fail "cannot create the drill holder container"
    holder_created=true
    docker start "$holder" > /dev/null || fail "cannot start the drill holder container"
    volume="$(docker inspect --format '{{range .Mounts}}{{if eq .Destination "/restore"}}{{.Name}}{{end}}{{end}}' "$holder")"
    [[ "$volume" =~ ^[A-Za-z0-9][A-Za-z0-9_.-]{0,254}$ ]] || fail "the drill holder has no scratch volume"
    log "restoring snapshot ${id:0:8} into scratch volume $volume"
    restore_into "$volume" "$id"
    restored_epoch="$(date +%s)"
    log "restored and verified in $(( restored_epoch - started ))s; PITR restore of the copy"
    COMPOSE_PROJECT_NAME="$PROJECT" bash "$PITR" --offline --spool-volume "$volume" --target latest \
        --evidence-dir "$dir/pitr" "${pitr_args[@]}" || pitr_rc=$?
    (( pitr_rc == 0 )) || fail "restore-postgres-pitr.sh failed on the off-NAS copy (exit $pitr_rc)"
    (( $(date +%s) - started <= RTO_SECONDS )) \
        || fail "download and restore took $(( $(date +%s) - started ))s, above the RTO of ${RTO_SECONDS}s"
}

command -v docker > /dev/null || fail "docker is not on PATH"
command -v flock > /dev/null || fail "flock is not on PATH"
case "$mode" in
    init) run_init ;;
    push) run_push ;;
    forget) run_forget ;;
    check) run_check ;;
    status) run_status ;;
    restore) run_restore ;;
    drill) run_drill ;;
esac
