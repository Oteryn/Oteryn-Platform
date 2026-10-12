#!/bin/bash
# restore_command for point-in-time recovery from the NAS spool (plan P5, runbook
# docs/operations/PLATFORM_POSTGRES_BACKUP_PITR.md):
#   restore_command = 'bash /path/to/restore-wal.sh "%f" "%p"'
# Exit 1 means "not in the archive" (normal at the end of the archive or for an absent timeline history).
# A file that is present but fails its checksum exits 199: PostgreSQL aborts recovery on an exit status
# above 125 instead of treating a damaged segment as the end of the archive and stopping early.
set -uo pipefail
umask 077
export LC_ALL=C

# The whole script is one function, called on the last line: bash parses it completely before running
# any of it, so a copy truncated while a deploy replaces it fails instead of running a prefix.
main() {
    spool="${OTERYN_BACKUP_SPOOL:-/var/lib/oteryn-postgres-backup}"

    [[ $# -eq 2 ]] || { echo "usage: restore-wal.sh <%f> <%p>" >&2; exit 199; }
    name="$1"
    destination="$2"
    if [[ ! "$name" =~ ^[0-9A-F]{24}(\.partial|\.[0-9A-F]{8}\.backup)?$|^[0-9A-F]{8}\.history$ ]]; then
        exit 1
    fi

    # An unreadable archive is damage, not the end of the archive: recovery must not stop early on it.
    if [[ ! -d "$spool/wal" || ! -r "$spool/wal" || ! -x "$spool/wal" ]]; then
        echo "oteryn-restore-wal: the WAL archive $spool/wal is missing or unreadable" >&2
        exit 199
    fi
    source="$spool/wal/$name.gz"
    [[ -f "$source" ]] || exit 1
    if [[ ! -f "$source.sha256" ]]; then
        echo "oteryn-restore-wal: $name.gz has no checksum file; refusing it" >&2
        exit 199
    fi
    expected="$(awk '{ print $1 }' "$source.sha256")"
    actual="$(sha256sum < "$source" | awk '{ print $1 }')"
    if [[ ! "$expected" =~ ^[0-9a-f]{64}$ || "$expected" != "$actual" ]]; then
        echo "oteryn-restore-wal: checksum mismatch for $name.gz; the archive is damaged" >&2
        exit 199
    fi

    tmp="$destination.oteryn-restore.$$"
    if ! gzip -dc -- "$source" > "$tmp"; then
        rm -f -- "$tmp"
        echo "oteryn-restore-wal: cannot decompress $name.gz" >&2
        exit 199
    fi
    mv -f -- "$tmp" "$destination" || exit 199
}

main "$@"
