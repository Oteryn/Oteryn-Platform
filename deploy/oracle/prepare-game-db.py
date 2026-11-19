#!/usr/bin/env python3
"""Prepare Game PostgreSQL or maintain its TLS certificates without replacing keys."""
import argparse
import fcntl
import os
from pathlib import Path
import secrets
import shutil
import stat
import subprocess
import tempfile

ROOT = Path('/opt/oteryn-oracle/game-db')
# Reviewed deployment files travel together; the PostgreSQL templates sit beside this script.
SOURCE = Path(__file__).resolve().parent

POSTGRES = 'oteryn-oracle-game-postgres-1'
CA_DAYS = 3650
SERVER_DAYS = 30
RENEW_BEFORE_DAYS = 7
SERVER_NAME = 'game-postgres'
SECONDS_PER_DAY = 86400


def run(*args):
    subprocess.run(args, check=True, stdout=subprocess.DEVNULL)


def write(path, value, mode=0o600, uid=0):
    # Game's shipped node/ops contracts require password files. Create those
    # files owner-only from the first byte, never follow or truncate a path,
    # and sync before exposing the completed root-private configuration tree.
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, mode)
    with os.fdopen(fd, 'w') as handle:
        os.fchmod(handle.fileno(), mode)
        os.fchown(handle.fileno(), uid, uid)
        handle.write(value)
        handle.flush()
        os.fsync(handle.fileno())


def sync_directory(path):
    fd = os.open(path, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(fd)
    finally:
        os.close(fd)


def publish_certificate(staged, target, mode):
    staged.chmod(mode)
    with staged.open('rb') as handle:
        os.fsync(handle.fileno())
    staged.replace(target)
    sync_directory(target.parent)


def valid_for(certificate, days):
    return subprocess.run(
        ['openssl', 'x509', '-in', str(certificate), '-noout', '-checkend', str(days * SECONDS_PER_DAY)],
        stdout=subprocess.DEVNULL, check=False,
    ).returncode == 0


def issue_server_certificate(ca, key, work):
    csr = work / 'server.csr'
    extensions = work / 'server.ext'
    certificate = work / 'server.crt'
    run('openssl', 'req', '-new', '-key', str(key), '-subj', '/CN=' + SERVER_NAME, '-out', str(csr))
    write(extensions, 'subjectAltName=DNS:' + SERVER_NAME + '\nextendedKeyUsage=serverAuth\n'
          'keyUsage=digitalSignature,keyEncipherment\n')
    run('openssl', 'x509', '-req', '-in', str(csr),
        '-CA', str(ca / 'ca.crt'), '-CAkey', str(ca / 'ca.key'),
        '-set_serial', '0x' + secrets.token_hex(16), '-days', str(SERVER_DAYS),
        '-sha256', '-extfile', str(extensions), '-out', str(certificate))
    run('openssl', 'verify', '-purpose', 'sslserver', '-verify_hostname', SERVER_NAME,
        '-CAfile', str(ca / 'ca.crt'), str(certificate))
    return certificate


def renew_server_certificate():
    ca = ROOT / 'signer'
    server = ROOT / 'server'
    certificate = server / 'server.crt'
    pending = ROOT / 'postgres-tls-reload-pending'
    ca_certificate = ca / 'ca.crt'

    if not valid_for(ca_certificate, SERVER_DAYS):
        raise SystemExit('Game PostgreSQL CA expires soon: extend the retained CA and refresh client trust.')
    if ca_certificate.read_bytes() != (ROOT / 'client/ca.crt').read_bytes():
        raise SystemExit('Game PostgreSQL client and signer CA certificates differ: finish the CA upgrade.')

    if not valid_for(certificate, RENEW_BEFORE_DAYS):
        with tempfile.TemporaryDirectory(prefix='.renew-', dir=server) as temporary:
            staged = issue_server_certificate(ca, server / 'server.key', Path(temporary))
            if not pending.exists():
                write(pending, '')
                sync_directory(ROOT)
            publish_certificate(staged, certificate, 0o644)

    # A retry after a failed reload must still signal PostgreSQL even when the
    # new certificate on disk is already valid. A stopped server loads it on start.
    if pending.exists():
        running = subprocess.check_output(['docker', 'inspect', '--format', '{{.State.Running}}', POSTGRES],
                                          text=True, timeout=30).strip()
        if running == 'true':
            run('docker', 'kill', '--signal', 'HUP', POSTGRES)
        elif running != 'false':
            raise ValueError('Could not determine Game PostgreSQL container state.')
        pending.unlink()
        sync_directory(ROOT)


def extend_ca():
    """Extend the legacy CA certificate while retaining its subject and key."""
    ca = ROOT / 'signer' / 'ca.crt'
    client = ROOT / 'client' / 'ca.crt'
    if not valid_for(ca, 365):
        with tempfile.TemporaryDirectory(prefix='.extend-', dir=ca.parent) as temporary:
            staged = Path(temporary) / ca.name
            run('openssl', 'x509', '-in', str(ca), '-signkey', str(ca.with_suffix('.key')),
                '-set_serial', '0x' + secrets.token_hex(16), '-days', str(CA_DAYS),
                '-sha256', '-out', str(staged))
            original_key = subprocess.check_output(['openssl', 'x509', '-in', str(ca), '-noout', '-pubkey'])
            renewed_key = subprocess.check_output(['openssl', 'x509', '-in', str(staged), '-noout', '-pubkey'])
            if renewed_key != original_key:
                raise SystemExit('Game PostgreSQL CA signing key does not match the retained certificate.')
            run('openssl', 'verify', '-check_ss_sig', '-CAfile', str(staged), str(staged))
            publish_certificate(staged, ca, 0o600)

    # Repair an interrupted upgrade from the already published signer.
    if not client.exists() or client.read_bytes() != ca.read_bytes():
        with tempfile.TemporaryDirectory(prefix='.extend-', dir=client.parent) as temporary:
            staged = Path(temporary) / client.name
            shutil.copyfile(ca, staged)
            publish_certificate(staged, client, 0o644)
    print('Game PostgreSQL CA ready; update retained Game runtime and ops client trust before legacy expiry.')


def main(operation=None):
    if os.geteuid() != 0:
        raise SystemExit('Run as root.')
    os.umask(0o077)
    if operation is not None:
        metadata = ROOT.lstat()
        if not stat.S_ISDIR(metadata.st_mode) or metadata.st_uid != 0 or stat.S_IMODE(metadata.st_mode) != 0o700:
            raise SystemExit('Unsafe Game database directory.')
        fd = os.open(ROOT / 'postgres-tls.lock', os.O_WRONLY | os.O_CREAT | os.O_NOFOLLOW, 0o600)
        with os.fdopen(fd, 'w') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            if operation == 'extend-ca':
                extend_ca()
            else:
                renew_server_certificate()
        return
    if ROOT.exists():
        raise SystemExit('Existing Game database configuration retained; refusing to regenerate it.')
    ROOT.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    staged = Path(tempfile.mkdtemp(prefix='.game-db-', dir=ROOT.parent))
    try:
        for name in ('signer', 'server', 'client', 'config', 'init', 'secrets'):
            (staged / name).mkdir(mode=0o755 if name != 'signer' else 0o700)
            # mkdir's mode is masked by the owner-only process umask. Mounted
            # directories must remain searchable by PostgreSQL / Game UIDs;
            # secret files themselves retain their individual 0600 ownership.
            (staged / name).chmod(0o700 if name in ('signer', 'secrets') else 0o755)
        ca = staged / 'signer'
        server = staged / 'server'
        run('openssl', 'req', '-x509', '-newkey', 'rsa:3072', '-nodes', '-sha256',
            '-days', str(CA_DAYS), '-subj', '/CN=Oteryn Oracle Game PostgreSQL CA',
            '-keyout', str(ca / 'ca.key'), '-out', str(ca / 'ca.crt'))
        key = server / 'server.key'
        run('openssl', 'genpkey', '-algorithm', 'RSA', '-pkeyopt', 'rsa_keygen_bits:2048', '-out', str(key))
        os.chown(key, 999, 999)
        key.chmod(0o600)
        with tempfile.TemporaryDirectory(prefix='.issue-', dir=server) as temporary:
            certificate = issue_server_certificate(ca, key, Path(temporary))
            publish_certificate(certificate, server / 'server.crt', 0o644)
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
-- Game migrations hand Control-issued tables and functions to the Control role
-- (ALTER ... OWNER TO, then triggers on them), so the migration role must be
-- able to act as Control. Control gets CREATE on the schema only while
-- migrate runs (prepare-game.sh, actions-deploy.py).
GRANT oteryn_game_control TO oteryn_game_owner WITH ADMIN FALSE, INHERIT TRUE, SET TRUE;
-- Game migrations bind these NOLOGIN groups but run without CREATEROLE; the
-- migration role administers them only (no inherit, no SET).
CREATE ROLE oteryn_game_highscore_job NOLOGIN NOINHERIT;
CREATE ROLE oteryn_game_admin_agent NOLOGIN;
GRANT oteryn_game_highscore_job TO oteryn_game_owner WITH ADMIN TRUE, INHERIT FALSE, SET FALSE;
GRANT oteryn_game_admin_agent TO oteryn_game_owner WITH ADMIN TRUE, INHERIT FALSE, SET FALSE;
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
        config = (SOURCE / 'game-postgres/postgresql.conf').read_text()
        config = config.replace('/bootstrap/postgres-pg_hba.conf', '/etc/oteryn/pg_hba.conf')
        config = config.replace('/run/oteryn-postgres-tls/', '/run/oteryn-game-db-tls/')
        config = config.replace('/bootstrap/postgres-backup/archive-wal.sh', '/etc/oteryn/archive-wal.sh')
        write(staged / 'config/postgresql.conf', config, 0o644)
        shutil.copyfile(SOURCE / 'game-postgres/archive-wal.sh', staged / 'config/archive-wal.sh')
        (staged / 'config/archive-wal.sh').chmod(0o644)
        write(staged / 'migrate.env', 'OTERYN_GAME_MIGRATION_DATABASE_URL=postgresql://oteryn_game_owner:' + passwords['owner'] + '@game-postgres:5432/oteryn_game?sslmode=verify-full&sslrootcert=/run/game-ca/ca.crt\n')
        for path in staged.rglob('*'):
            if path.is_file():
                with path.open('rb') as handle:
                    os.fsync(handle.fileno())
        staged.rename(ROOT)
        sync_directory(ROOT.parent)
        print('Game PostgreSQL configuration created; private keys and passwords were not printed.')
    finally:
        if staged.exists():
            shutil.rmtree(staged)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('operation', nargs='?', choices=('renew-server-certificate', 'extend-ca'))
    main(parser.parse_args().operation)
