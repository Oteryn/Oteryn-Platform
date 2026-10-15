#!/usr/bin/env python3
"""Prove deployed Game runtime credentials use TLS and cannot perform DDL."""
import os
from pathlib import Path
import subprocess

ROOT = Path('/opt/oteryn-oracle/game-db')
IMAGE = 'postgres:17.11@sha256:2d2b8998d31037bf721cfdf764d76ba74171b4fab3431b7f72c27c56ddbdf9e3'


def main():
    if os.geteuid() != 0:
        raise SystemExit('Run as root.')
    password = (ROOT / 'secrets/runtime-password').read_text().strip()
    environment = {
        'PGHOST': 'game-postgres', 'PGPORT': '5432', 'PGDATABASE': 'oteryn_game',
        'PGUSER': 'oteryn_game_runtime_login', 'PGPASSWORD': password,
        'PGSSLMODE': 'verify-full', 'PGSSLROOTCERT': '/run/game-ca/ca.crt',
        'PGCONNECT_TIMEOUT': '3',
    }
    probe = '''set -euo pipefail
psql -XqAt -v ON_ERROR_STOP=1 -c 'SELECT current_user, ssl, version FROM pg_stat_ssl WHERE pid = pg_backend_pid();'
if rejected=$(psql -XqAt -v ON_ERROR_STOP=1 -v VERBOSITY=sqlstate -c 'BEGIN; CREATE TABLE public.oracle_runtime_privilege_probe (id integer); ROLLBACK;' 2>&1); then
    echo runtime_ddl_rejection=FAIL; exit 1
fi
case "$rejected" in *42501*) ;; *) echo runtime_ddl_rejection=UNEXPECTED_FAILURE; exit 1;; esac
echo runtime_ddl_rejection=PASS
if rejected=$(PGSSLMODE=disable psql -XqAt -c 'SELECT 1' 2>&1); then
    echo plaintext_rejection=FAIL; exit 1
fi
case "$rejected" in *'pg_hba.conf rejects connection'*'no encryption'*) ;; *) echo plaintext_rejection=UNEXPECTED_FAILURE; exit 1;; esac
echo plaintext_rejection=PASS
if rejected=$(PGSSLROOTCERT=/nonexistent.crt psql -XqAt -c 'SELECT 1' 2>&1); then
    echo unknown_ca_rejection=FAIL; exit 1
fi
case "$rejected" in *'root certificate file'*'does not exist'*) ;; *) echo unknown_ca_rejection=UNEXPECTED_FAILURE; exit 1;; esac
echo unknown_ca_rejection=PASS
psql -XqAt -v ON_ERROR_STOP=1 -c 'SELECT 1' | grep -qx 1
'''
    # Only the Docker CLI reads the secret input pipe. No credential is placed
    # in arguments, a temporary file, or output; the short-lived probe container
    # receives only the runtime role's password.
    subprocess.run([
        'docker', 'run', '--rm', '--network', 'oteryn-oracle-game_game_private',
        '--env-file', '/dev/stdin', '--mount',
        f'type=bind,source={ROOT}/client,target=/run/game-ca,readonly',
        '--entrypoint', 'bash', IMAGE, '-ec', probe,
    ], input=''.join(f'{key}={value}\n' for key, value in environment.items()), text=True, check=True)


if __name__ == '__main__':
    main()
