#!/usr/bin/env python3
"""Prepare a fresh isolated Game database. Never replace existing state or keys."""
import os
from pathlib import Path
import secrets
import shutil
import subprocess
import tempfile

ROOT = Path('/opt/oteryn-oracle/game-db')
SOURCE = Path('/opt/oteryn-build/Oteryn-Platform-1194d5548760d9d773a1cebb182bc5986a6a42bf')


def run(*args):
    subprocess.run(args, check=True, stdout=subprocess.DEVNULL)


def write(path, value, mode=0o600, uid=0):
    path.write_text(value)
    path.chmod(mode)
    os.chown(path, uid, uid)


def main():
    if os.geteuid() != 0:
        raise SystemExit('Run as root.')
    if ROOT.exists():
        raise SystemExit('Existing Game database configuration retained; refusing to regenerate it.')
    os.umask(0o077)
    ROOT.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    staged = Path(tempfile.mkdtemp(prefix='.game-db-', dir=ROOT.parent))
    try:
        for name in ('signer', 'server', 'client', 'config', 'init', 'secrets'):
            (staged / name).mkdir(mode=0o755 if name != 'signer' else 0o700)
        ca = staged / 'signer'
        server = staged / 'server'
        run('openssl', 'req', '-x509', '-newkey', 'rsa:3072', '-nodes', '-sha256',
            '-days', '30', '-subj', '/CN=Oteryn Oracle Game PostgreSQL CA',
            '-keyout', str(ca / 'ca.key'), '-out', str(ca / 'ca.crt'))
        run('openssl', 'req', '-new', '-newkey', 'rsa:2048', '-nodes',
            '-subj', '/CN=game-postgres', '-keyout', str(server / 'server.key'),
            '-out', str(server / 'server.csr'))
        write(server / 'server.ext', 'subjectAltName=DNS:game-postgres\nextendedKeyUsage=serverAuth\nkeyUsage=digitalSignature,keyEncipherment\n')
        run('openssl', 'x509', '-req', '-in', str(server / 'server.csr'),
            '-CA', str(ca / 'ca.crt'), '-CAkey', str(ca / 'ca.key'),
            '-CAcreateserial', '-days', '30', '-sha256', '-extfile', str(server / 'server.ext'),
            '-out', str(server / 'server.crt'))
        (server / 'server.csr').unlink()
        (server / 'server.ext').unlink()
        os.chown(server / 'server.key', 999, 999)
        (server / 'server.key').chmod(0o600)
        (server / 'server.crt').chmod(0o644)
        shutil.copyfile(ca / 'ca.crt', staged / 'client/ca.crt')
        (staged / 'client/ca.crt').chmod(0o644)
        passwords = {role: secrets.token_hex(32) for role in ('postgres', 'owner', 'control', 'runtime')}
        for role, password in passwords.items():
            write(staged / 'secrets' / (role + '-password'), password + '\n', uid=1001 if role == 'runtime' else 999 if role == 'postgres' else 0)
        sql = '''\\set ON_ERROR_STOP on
SET log_statement = 'none';
CREATE ROLE oteryn_game_control NOLOGIN;
CREATE ROLE oteryn_game_runtime NOLOGIN;
CREATE ROLE oteryn_game_owner LOGIN PASSWORD '{owner}' NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;
CREATE ROLE oteryn_game_control_login LOGIN PASSWORD '{control}' NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS IN ROLE oteryn_game_control;
CREATE ROLE oteryn_game_runtime_login LOGIN PASSWORD '{runtime}' NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS IN ROLE oteryn_game_runtime;
CREATE DATABASE oteryn_game OWNER oteryn_game_owner ENCODING 'UTF8' LOCALE 'C.UTF-8' TEMPLATE template0;
REVOKE ALL ON DATABASE oteryn_game FROM PUBLIC;
GRANT CONNECT ON DATABASE oteryn_game TO oteryn_game_control_login, oteryn_game_runtime_login;
'''.format(**passwords)
        write(staged / 'init/10-game-roles.sql', sql, uid=999)
        write(staged / 'config/pg_hba.conf', '''local all postgres peer
hostssl oteryn_game oteryn_game_owner 172.30.84.0/24 scram-sha-256
hostssl oteryn_game oteryn_game_control_login 172.30.84.0/24 scram-sha-256
hostssl oteryn_game oteryn_game_runtime_login 172.30.84.0/24 scram-sha-256
local all all reject
host all all all reject
host replication all all reject
''', 0o644)
        config = (SOURCE / 'deploy/synology/postgres/postgresql.conf').read_text()
        config = config.replace('/bootstrap/postgres-pg_hba.conf', '/etc/oteryn/pg_hba.conf')
        config = config.replace('/run/oteryn-postgres-tls/', '/run/oteryn-game-db-tls/')
        config = config.replace('/bootstrap/postgres-backup/archive-wal.sh', '/etc/oteryn/archive-wal.sh')
        write(staged / 'config/postgresql.conf', config, 0o644)
        shutil.copyfile(SOURCE / 'deploy/synology/postgres/backup/archive-wal.sh', staged / 'config/archive-wal.sh')
        (staged / 'config/archive-wal.sh').chmod(0o644)
        write(staged / 'migrate.env', 'OTERYN_GAME_MIGRATION_DATABASE_URL=postgresql://oteryn_game_owner:' + passwords['owner'] + '@game-postgres:5432/oteryn_game?sslmode=verify-full&sslrootcert=/run/game-ca/ca.crt\n')
        for path in staged.rglob('*'):
            if path.is_file():
                with path.open('rb') as handle:
                    os.fsync(handle.fileno())
        staged.rename(ROOT)
        fd = os.open(ROOT.parent, os.O_RDONLY | os.O_DIRECTORY)
        try:
            os.fsync(fd)
        finally:
            os.close(fd)
        print('Game PostgreSQL configuration created; private keys and passwords were not printed.')
    finally:
        if staged.exists():
            shutil.rmtree(staged)


if __name__ == '__main__':
    main()
