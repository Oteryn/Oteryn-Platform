#!/bin/bash
# archive_command of the Platform PostgreSQL server (docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md P5,
# runbook docs/operations/PLATFORM_POSTGRES_BACKUP_PITR.md). postgresql.conf runs it as
#   archive_command = '/bin/bash /bootstrap/postgres-backup/archive-wal.sh "%p" "%f" && test -s <spool>/wal/%f.gz'
# as the postgres user with the data directory as working directory. It copies one finished WAL file into
# the NAS spool (the postgres_backup volume, outside postgres_data) as <name>.gz plus <name>.gz.sha256.
#
# Exit 0 only when the file is durably in the spool: PostgreSQL then may recycle the segment. Any other
# exit keeps the segment in pg_wal and the archiver retries it (pg_stat_archiver.failed_count, the
# `.ready` backlog checked by backup-postgres.sh check). Never overwrites a different archived file.
set -euo pipefail
umask 077
export LC_ALL=C

# The whole script is one function, called on the last line: bash parses it completely before running
# any of it, so a copy truncated while a deploy replaces it fails instead of running a prefix.
main() {
    spool="${OTERYN_BACKUP_SPOOL:-/var/lib/oteryn-postgres-backup}"
    min_free_mb="${OTERYN_ARCHIVE_MIN_FREE_MB:-256}"

    fail() {
        printf 'oteryn-archive-wal: %s\n' "$*" >&2
        exit 1
    }

    [[ $# -eq 2 ]] || fail "usage: archive-wal.sh <%p> <%f>"
    source_path="$1"
    name="$2"
    [[ "$name" =~ ^[0-9A-F]{24}(\.partial|\.[0-9A-F]{8}\.backup)?$|^[0-9A-F]{8}\.history$ ]] \
        || fail "refusing an unexpected WAL file name: $name"
    [[ -f "$source_path" ]] || fail "source $source_path does not exist"
    [[ "$min_free_mb" =~ ^[0-9]{1,7}$ ]] || fail "OTERYN_ARCHIVE_MIN_FREE_MB must be a decimal integer of at most seven digits"
    min_free_mb=$((10#$min_free_mb))

    dir="$spool/wal"
    # The backup sidecar prepares the spool (backup-job.sh service); until then archiving fails and retries.
    [[ -d "$dir" && -w "$dir" ]] || fail "spool $dir is not prepared (is the postgres-backup sidecar running?)"
    free_kb="$(df -Pk -- "$dir" | awk 'NR == 2 { print $4 }')"
    [[ "$free_kb" =~ ^[0-9]+$ ]] || fail "cannot read the free space of $dir"
    (( free_kb >= min_free_mb * 1024 )) || fail "spool free space $((free_kb / 1024)) MiB is below ${min_free_mb} MiB"

    target="$dir/$name.gz"
    if [[ -e "$target" ]]; then
        # A retry after a crash between the rename and PostgreSQL recording success: accept identical content.
        # The crash may also have come before the directory sync, so the success path makes the files and
        # their directory entries durable before PostgreSQL may recycle the segment.
        if [[ -f "$target.sha256" ]] \
            && [[ "$(sha256sum < "$target" | awk '{ print $1 }')" == "$(awk '{ print $1 }' "$target.sha256")" ]] \
            && gzip -dc -- "$target" | cmp -s -- - "$source_path"; then
            sync -- "$target.sha256" "$target" "$dir" || fail "cannot sync the archived $name"
            exit 0
        fi
        fail "$name is already archived with different content; refusing to overwrite it"
    fi

    tmp="$dir/.$name.gz.tmp.$$"
    trap 'rm -f -- "$tmp" "$tmp.sha256"' EXIT
    gzip -c -6 -- "$source_path" > "$tmp"
    printf '%s  %s\n' "$(sha256sum < "$tmp" | awk '{ print $1 }')" "$name.gz" > "$tmp.sha256"
    sync -- "$tmp" "$tmp.sha256"
    # Checksum first: a data file never exists without its checksum (restore-wal.sh refuses one that does).
    mv -f -- "$tmp.sha256" "$target.sha256"
    mv -- "$tmp" "$target"
    sync -- "$dir"
    trap - EXIT
}

main "$@"
