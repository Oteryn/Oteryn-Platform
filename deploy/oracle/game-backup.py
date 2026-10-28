#!/usr/bin/env python3
"""Daily host-owned Game backups; peer authentication stays inside PostgreSQL.

No runtime credentials are exported and no database listener is published. The
backup spool is a separate retained volume. Failed partial files never become
completed backups. Retention is deliberately manual during migration acceptance.
"""
import datetime
import fcntl
import os
from pathlib import Path
import subprocess

ROOT = Path('/opt/oteryn-oracle/state/game-backup')
CONTAINER = 'oteryn-oracle-game-postgres-1'


def run(*args):
    subprocess.run(['docker', 'exec', '--user', 'postgres', CONTAINER, *args],
                   check=True, stdout=subprocess.DEVNULL, timeout=1800)


def main():
    os.umask(0o077)
    ROOT.mkdir(mode=0o700, parents=True, exist_ok=True)
    with (ROOT / 'lock').open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        stamp = datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%SZ')
        base = '/var/lib/oteryn-postgres-backup/daily/' + stamp
        run('mkdir', '-p', '-m', '0700', base)
        run('pg_basebackup', '-h', '/var/run/postgresql', '-U', 'postgres',
            '-D', base + '/physical.partial', '-Ft', '-z', '-X', 'stream', '-c', 'fast')
        # Validate PostgreSQL's manifest and the required WAL before publication.
        run('mkdir', '-m', '0700', base + '/verify')
        run('tar', '-xzf', base + '/physical.partial/base.tar.gz', '-C', base + '/verify')
        run('mkdir', '-p', base + '/verify/pg_wal')
        run('tar', '-xzf', base + '/physical.partial/pg_wal.tar.gz', '-C', base + '/verify/pg_wal')
        run('cp', base + '/physical.partial/backup_manifest', base + '/verify/backup_manifest')
        run('pg_verifybackup', base + '/verify')
        run('rm', '-rf', '--', base + '/verify')
        run('mv', base + '/physical.partial', base + '/physical')
        run('pg_dump', '-h', '/var/run/postgresql', '-U', 'postgres', '-d', 'oteryn_game',
            '-Fc', '-f', base + '/game.dump.partial')
        run('pg_restore', '--list', base + '/game.dump.partial')
        run('mv', base + '/game.dump.partial', base + '/game.dump')
        (ROOT / 'last-complete').write_text(stamp + '\n')
        print('game_backup_complete=' + stamp)


if __name__ == '__main__':
    main()
