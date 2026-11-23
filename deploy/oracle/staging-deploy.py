#!/usr/bin/env python3
"""Root-operated Oracle staging, with checkout code confined to staging containers."""
import argparse
import base64
from contextlib import contextmanager, nullcontext
import fcntl
import hashlib
import http.client
import io
import json
import os
from pathlib import Path
import re
import secrets
import shutil
import stat
import subprocess
import sys
import tarfile
import tempfile
import time
import tomllib
import urllib.request

ROOT = Path('/opt/oteryn-staging')
# The only production path this CLI may observe, and only via production_idle().
PRODUCTION_STATE = Path('/opt/oteryn-oracle/state/actions')
PROJECT = 'oteryn-staging'
REPO = 'https://github.com/Oteryn/Oteryn-Platform.git'
REVISION = 'org.opencontainers.image.revision'
PLAYWRIGHT_VERSION = '1.58.2'
SHA_RE = re.compile(r'[0-9a-f]{40}')
DEPLOY_ONLY = 'none'
SPEC_RE = re.compile(r'[A-Za-z0-9_./-]{0,200}')
CRATE_RE = re.compile(r'[A-Za-z0-9_][A-Za-z0-9_-]{0,99}')
CARGO_FAILURE = re.compile(r'^test (\S{1,500}) \.\.\. FAILED$')
ORIGIN = 'https://localhost:18443'
# The owner may put https://<hostname> in ROOT/public-origin for the
# tunnel-facing portal-public service; without it both use ORIGIN.
PUBLIC_ORIGIN_RE = re.compile(r'https://(?:[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?\.)+[a-z]{2,63}')
PORTAL_DB = 'portal_e2e_staging'
TEST_DB = 'portal_staging_crate_tests'
OFF = (
    'GATEWAY_NATIVE_LOGIN_ENABLED', 'GATEWAY_NATIVE_PROTOCOL3_ENABLED',
    'GAME_AUTH_NATIVE_EVIDENCE_ENABLED', 'OTERYN_CHARACTER_COMMANDS_ENABLED',
    'OTERYN_CHARACTER_COMMAND_CREATE_ENABLED', 'OTERYN_CHARACTER_CREATE_WEB_ENABLED',
    'OTERYN_ACCOUNT_BALANCE_ENABLED',
    'OTERYN_SHOP_ENABLED', 'OTERYN_SHOP_TOP_UPS_ENABLED', 'OTERYN_SHOP_TEST_PAYMENTS_ENABLED',
    'OTERYN_STORE_ENABLED', 'OTERYN_PAYMENTS_ENABLED', 'OTERYN_REFUNDS_ENABLED',
    'OTERYN_ADMIN_COMMERCE', 'OTERYN_SEASON_PASS_ENABLED', 'OTERYN_SEASON_PASS_WALLET_ENABLED',
    'OTERYN_BAZAAR_ENABLED', 'OTERYN_BAZAAR_MODERATION', 'OTERYN_BAZAAR_PROFILES',
    'OTERYN_BAZAAR_PROMOTIONS', 'OTERYN_GAME_WORLD_EVENT_GUILD_SEASON',
    'OTERYN_PLAYERDATA_GAME_LOGIN_HISTORY_ENABLED', 'PORTAL_GAME_RESTORE_NOTICE_ENABLED',
    'PRODUCTS_ENTITLEMENTS_STORE_DELIVERY_ENABLED', 'PRODUCTS_ENTITLEMENTS_STORE_PURCHASE_ENABLED',
    'PANEL_PREMIUM_PAYMENTS', 'PORTAL_NATIVE_PROTOCOL3_ENABLED', 'PORTAL_NATIVE_RECOVERY_ENABLED',
    'PORTAL_PLAYERDATA_G9_INGEST_ENABLED', 'PORTAL_PLAYERDATA_G10_INGEST_ENABLED',
    'PORTAL_PLAYERDATA_PROGRESS_INGEST_ENABLED', 'PORTAL_ACCOUNT_CLOSURE_ENABLED',
)
# The deployed staging profile keeps the read-only public features. By design,
# it has no Game or payment fixture configuration.
PUBLIC_FLAGS = (
    'OTERYN_ACCOUNT_BADGES_ENABLED', 'OTERYN_ACCOUNT_EXPORT_ENABLED',
    'OTERYN_ATLAS_ENABLED', 'OTERYN_ATLAS_BESTIARY_ENABLED', 'OTERYN_ATLAS_DELIVERY_ENABLED',
    'OTERYN_ATLAS_SPAWNS_ENABLED', 'OTERYN_CHARACTER_PROFILES_ENABLED',
    'OTERYN_CHARACTER_SERVICES_ENABLED', 'OTERYN_COLLECTION_ENABLED',
    'OTERYN_COLLECTION_PVP_ENABLED', 'OTERYN_COLLECTION_SETS_ENABLED',
    'OTERYN_DAILY_PUZZLES_ENABLED', 'OTERYN_DOWNLOADS_ENABLED', 'OTERYN_JOURNAL_REPORTS_ENABLED',
    'OTERYN_LEAGUE_ENABLED', 'OTERYN_PLAYERDATA_PROGRESS_READ_ENABLED', 'OTERYN_REPORTS_ENABLED',
    'OTERYN_SECURITY_WIZARD_ENABLED', 'OTERYN_SUPPORT_TICKETS', 'OTERYN_WORLD_ENABLED',
    # The rest of the production read-only profile (/opt/oteryn-portal/portal.env).
    'OTERYN_PLAYERDATA_HUNTS_READ', 'OTERYN_PLAYERDATA_PLACES_AGGREGATE',
    'PORTAL_CALCULATORS_ENABLED', 'PORTAL_CHARACTER_STATS_ENABLED',
    'PORTAL_DEATHS_ENABLED', 'PORTAL_GROWTH_RANKINGS_ENABLED', 'PORTAL_GUILDS_ENABLED',
    'PORTAL_GUILD_STATS_ENABLED', 'PORTAL_HIRELINGS_ENABLED', 'PORTAL_HOUSES_SERVING_ENABLED',
    'PORTAL_PUBLIC_PROJECTIONS_ENABLED', 'OTERYN_LCFA_ENABLED', 'OTERYN_PREMIUM_ENABLED',
    'PORTAL_ONLINE_ENABLED', 'PORTAL_PARTY_ENABLED', 'PORTAL_PUBLIC_PROFILES_ENABLED',
    'PORTAL_PUBLIC_RANKINGS_ENABLED', 'PORTAL_STATS_ENABLED', 'PORTAL_STATS_WORLD_TOTALS_ENABLED',
    'PORTAL_WIKI_ENTITIES_ENABLED', 'PORTAL_WIKI_PLACES_ENABLED', 'PORTAL_WORLDS_ENABLED',
    'PORTAL_WORLD_DAILY_PUBLIC_ENABLED', 'PORTAL_WORLD_STATS_ENABLED',
)
# Staff panel only on its own host from ROOT/admin-origin (owner decision
# 2026-11-23: the staff login is never on the player portal's host). Cookies
# are host-only, so it is a separate login session; without that file, or with
# the same host as the public portal, the staff panel stays off. Production has
# no staff host yet; this is a staging-only surface.
ADMIN_FLAGS = (
    'OTERYN_ADMIN_PANEL', 'OTERYN_ADMIN_PLAYERS', 'OTERYN_ADMIN_PLAYER_CHARACTERS',
    'OTERYN_ADMIN_CONSOLE', 'OTERYN_ADMIN_CONTROL', 'OTERYN_ADMIN_CHARTS_EXTENDED_ENABLED',
)
STAFF_ROLES = (
    'platform_admin', 'security_admin', 'content_editor', 'moderator', 'support', 'operations',
    'game_admin', 'game_analyst', 'game_dba', 'game_master_senior', 'game_operator',
    'game_release', 'game_security', 'game_support', 'game_viewer',
)
FOUNDER = 'founder@staging.invalid'
STAFF = {FOUNDER: 'platform_admin', **{f'staff-{role.replace("_", "-")}@staging.invalid': role
                                      for role in STAFF_ROLES}}
PLAYERS = tuple(f'player-{kind}@staging.invalid' for kind in
                ('basic', 'mfa', 'premium', 'guild-leader', 'new', 'restricted'))
TOTP_ACCOUNTS = {*STAFF, 'player-mfa@staging.invalid'}
SAMPLE_SQL = 'deploy/oracle/staging-sample-data.sql'

# Staging has no egress, so the breached-password check could never reach the
# range API and registration and reset would fail closed (503). Every prefix is
# cached as complete; only these documented passwords count as breached, so
# black-box tests can still see the refusal. Registration is enabled here.
BREACHED_SAMPLES = ('Staging-Breached-Password-1', 'Staging-Breached-Password-2')
REGISTRATION_POLICY = (
    "BEGIN;\n"
    "UPDATE registration_abuse_policy SET enabled = true WHERE singleton;\n"
    "INSERT INTO registration_password_ranges(prefix, suffixes, expires_at, complete, complete_xact) "
    "SELECT upper(lpad(to_hex(n), 5, '0')), ARRAY[]::text[], now() + interval '30 days', true, "
    "pg_current_xact_id() FROM generate_series(0, 1048575) AS n ON CONFLICT (prefix) DO UPDATE SET "
    "suffixes = EXCLUDED.suffixes, expires_at = EXCLUDED.expires_at, complete = true, "
    "complete_xact = pg_current_xact_id();\n"
    + ''.join(
        "UPDATE registration_password_ranges SET suffixes = suffixes || "
        f"ARRAY['{digest[5:]}'], complete_xact = pg_current_xact_id() WHERE prefix = '{digest[:5]}';\n"
        for digest in (hashlib.sha1(sample.encode()).hexdigest().upper() for sample in BREACHED_SAMPLES))
    + "COMMIT;\n"
).encode()
ACCOUNT_FLAGS = (
    'LOGIN', 'MFA', 'PASSKEYS', 'PASSWORD_RESET', 'SESSIONS', 'CREDENTIALS',
    'EMAIL_CHANGE', 'RECOVERY_KEYS', 'OVERVIEW_ENABLED', 'PRIVACY_ENABLED',
)


class Refusal(ValueError):
    """A diagnostic safe to persist or display."""


def parse(argv):
    parser = argparse.ArgumentParser(allow_abbrev=False)
    commands = parser.add_subparsers(dest='command', required=True)
    deploy = commands.add_parser('deploy', allow_abbrev=False)
    deploy.add_argument('sha')
    deploy.add_argument('spec', nargs='?', default=DEPLOY_ONLY)
    deploy.add_argument('--pg-tests', default=None)
    commands.add_parser('status')
    commands.add_parser('logs')
    args = parser.parse_args(argv)
    if args.command != 'deploy':
        return args.command, None, '', []
    sha = args.sha.lower()
    if not SHA_RE.fullmatch(sha) or not SPEC_RE.fullmatch(args.spec):
        parser.error('deploy requires a full hexadecimal SHA and an optional spec path filter')
    crates = [] if args.pg_tests is None else args.pg_tests.split(',', 20)
    if len(crates) > 20 or any(not CRATE_RE.fullmatch(crate) for crate in crates):
        parser.error('--pg-tests requires comma-separated Cargo package names')
    return args.command, sha, args.spec, list(dict.fromkeys(crates))


def environment():
    return {
        'PATH': '/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin',
        'HOME': str(ROOT), 'LANG': 'C.UTF-8', 'DOCKER_HOST': 'unix:///var/run/docker.sock',
        'DOCKER_CONFIG': str(ROOT / 'docker'), 'DOCKER_BUILDKIT': '1',
        'GIT_CONFIG_NOSYSTEM': '1', 'GIT_CONFIG_GLOBAL': '/dev/null',
        'GIT_TERMINAL_PROMPT': '0', 'GIT_ALLOW_PROTOCOL': 'https',
    }


def run(args, *, cwd=None, data=None, env=None, timeout=180, output=False, check=True):
    result = subprocess.run(
        args, cwd=cwd or ROOT, input=data, stdin=subprocess.DEVNULL if data is None else None,
        stdout=subprocess.PIPE if output else subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        env=environment() | (env or {}), timeout=timeout, check=check, umask=0o022,
    )
    if output and check:
        return result.stdout.decode('utf-8', errors='replace')
    return result


def production_idle():
    try:
        lock = (PRODUCTION_STATE / 'deployment.lock').open('rb')
    except FileNotFoundError:
        # A host with no deployment lock yet can still have a pending receipt.
        lock = None
    try:
        if lock is not None:
            fcntl.flock(lock, fcntl.LOCK_SH | fcntl.LOCK_NB)
        if any(PRODUCTION_STATE.glob('*-pending.json')):
            raise Refusal('production has a pending deployment')
    except BlockingIOError:
        raise Refusal('production deploy is running') from None
    finally:
        if lock is not None:
            lock.close()


def host_lock(name, busy):
    ROOT.mkdir(mode=0o700, parents=True, exist_ok=True)
    lock = (ROOT / name).open('ab')
    try:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BaseException as error:
        lock.close()
        if isinstance(error, BlockingIOError):
            raise Refusal(busy) from None
        raise
    return lock


def deploy_lock():
    return host_lock('deployment.lock', 'another staging deploy holds the lock')


def qualification_lock():
    return host_lock('qualification.lock', 'another staging qualification holds the lock')


def active_qualification():
    with (ROOT / 'qualification.lock').open('ab') as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_SH | fcntl.LOCK_NB)
        except BlockingIOError:
            return pointer('qualifying')
    return None


def remote_heads():
    # One git request instead of paging the REST API: unauthenticated API calls
    # share a 60/hour budget per address and two checks per deploy exhausted it.
    listing = run(['git', '-c', 'core.hooksPath=/dev/null', '-c', 'http.followRedirects=false',
                   'ls-remote', '--heads', REPO, 'refs/heads/codex/*', 'refs/heads/staging/*'],
                  output=True, timeout=60)
    heads = []
    for line in listing.splitlines():
        sha, _, ref = line.partition('\t')
        if not ref.startswith('refs/heads/'):
            raise Refusal('unexpected git ls-remote response')
        heads.append((ref.removeprefix('refs/heads/'), sha))
    return heads


def qualify(sha):
    if not any(name.startswith(('codex/', 'staging/')) and head == sha
               for name, head in remote_heads()):
        raise Refusal('source must be the current head of a codex/ or staging/ branch')


def git(directory, *args):
    return run(['git', '-c', 'core.hooksPath=/dev/null', '-c', 'http.followRedirects=false',
                *args], cwd=directory, output=True).strip()


def source(sha):
    parent = ROOT / 'src'
    parent.mkdir(exist_ok=True)
    directory = parent / sha
    if not directory.exists():
        with tempfile.TemporaryDirectory(prefix='.fetch-', dir=parent) as temporary:
            checkout = Path(temporary)
            git(checkout, 'init', '--quiet', '--template=')
            git(checkout, 'fetch', '--quiet', '--depth=1', '--no-tags', REPO, sha)
            git(checkout, 'checkout', '--quiet', '--detach', 'FETCH_HEAD')
            verify_checkout(checkout, sha)
            checkout.rename(directory)
    verify_checkout(directory, sha)
    return directory


def verify_checkout(directory, sha):
    if directory.is_symlink() or git(directory, 'rev-parse', 'HEAD') != sha:
        raise Refusal('source identity differs')
    if git(directory, 'status', '--porcelain', '--untracked-files=all', '--ignored=matching'):
        raise Refusal('source checkout is dirty')


def build_context(directory, pg=False, browser=True):
    portal = directory / 'portal-rs'
    e2e = portal / 'e2e'
    lock = e2e / 'package-lock.json'
    cargo_lock = portal / 'Cargo.lock'
    paths = [portal, cargo_lock]
    if browser:
        paths += [e2e, lock, e2e / 'package.json', directory / 'docs/contracts/fixtures']
    if browser or pg:
        paths += [directory / 'docs', directory / 'docs/contracts']
    if pg:
        paths += [directory / 'deploy', directory / 'deploy/oracle']
    if any(path.is_symlink() for path in paths):
        raise Refusal('staging build inputs must be within the checkout')
    if browser:
        packages = json.loads(lock.read_text())['packages']
        if packages['node_modules/@playwright/test']['version'] != PLAYWRIGHT_VERSION:
            raise Refusal('Playwright lockfile and installed browser image versions differ')
        if any(not package.get('resolved', '').startswith('https://registry.npmjs.org/')
               for name, package in packages.items() if name):
            raise Refusal('staging npm dependencies must use the public npm registry')
    if any(package.get('source', 'registry+https://github.com/rust-lang/crates.io-index')
           != 'registry+https://github.com/rust-lang/crates.io-index'
           for package in tomllib.loads(cargo_lock.read_text())['package']):
        raise Refusal('staging Cargo dependencies must use crates.io')


def require_capacity():
    # One test container at a time, alongside the serving stack.
    cpus, memory = map(int, run(['docker', 'info', '--format', '{{.NCPU}} {{.MemTotal}}'],
                               output=True).split())
    if cpus < 4 or memory < 10 * 1024**3:
        raise Refusal('staging requires at least 4 CPUs and 10 GiB of host memory')


def image_tag(sha, target='runtime'):
    suffix = '' if target == 'runtime' else f'-{target}'
    return f'{PROJECT}:{sha[:8]}{suffix}-arm64'


def verify_image(image, sha):
    info, = json.loads(run(['docker', 'image', 'inspect', image], output=True))
    if info['Architecture'] != 'arm64' or (info['Config']['Labels'] or {}).get(REVISION) != sha:
        raise Refusal('staging image provenance differs')


def build(directory, sha, target):
    portal = directory / 'portal-rs'
    image = image_tag(sha, target)
    args = ['docker', 'build', '--platform', 'linux/arm64', '--target', target,
            '--build-arg', 'CARGO_BUILD_JOBS=2', '--label', f'{REVISION}={sha}',
            '-f', str(ROOT / 'staging.Dockerfile'), '-t', image]
    if target == 'browser':
        args += ['--build-context', f'e2e={portal / "e2e"}',
                 '--build-context', f'contracts={directory / "docs/contracts"}']
    elif target == 'builder':
        args += ['--build-context', f'contracts={directory / "docs/contracts"}',
                 '--build-context', f'oracle={directory / "deploy/oracle"}']
    run([*args, str(portal)], timeout=7200)
    verify_image(image, sha)
    if target == 'runtime':
        run(['docker', 'run', '--rm', '--network', 'none', '--read-only',
             '--cap-drop', 'ALL', '--security-opt', 'no-new-privileges:true',
             '--cpus', '0.3', '--memory', '512m', image, 'check-image'], timeout=120)


def write(path, content):
    with tempfile.NamedTemporaryFile(mode='w', dir=path.parent, delete=False) as stream:
        stream.write(content)
        temporary = Path(stream.name)
    temporary.replace(path)


def env_file(directory, filename, values):
    write(directory / filename, ''.join(f'{key}={value}\n' for key, value in values.items()))


def tunnel_origin(name, default):
    path = ROOT / name
    if not path.exists():
        return default
    value = path.read_text().strip()
    if not PUBLIC_ORIGIN_RE.fullmatch(value):
        raise Refusal(f'{name} must be https://<hostname>')
    return value


def public_origin():
    return tunnel_origin('public-origin', ORIGIN)


def admin_origin(public):
    admin = tunnel_origin('admin-origin', None)
    if admin is not None and admin == public:
        raise Refusal('admin-origin must differ from public-origin')
    return admin


def random_hex_key():
    return hashlib.sha256(secrets.token_urlsafe(32).encode()).hexdigest()


def test_accounts():
    path = ROOT / 'test-accounts.json'
    emails = (*PLAYERS, *STAFF)
    try:
        fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    except FileNotFoundError:
        accounts = {}
        for email in emails:
            entry = {'password': secrets.token_urlsafe(32)}
            if email in TOTP_ACCOUNTS:
                entry['totpSecret'] = base64.b32encode(secrets.token_bytes(20)).decode('ascii')
            accounts[email] = entry
        # Publish a complete file without replacing an existing one.
        with tempfile.NamedTemporaryFile(mode='w', dir=ROOT) as stream:
            if os.geteuid() == 0:
                os.fchown(stream.fileno(), 0, 0)
            json.dump(accounts, stream, indent=2)
            stream.write('\n')
            stream.flush()
            os.fsync(stream.fileno())
            try:
                os.link(stream.name, path)
            except FileExistsError:
                pass
        fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(fd) as stream:
        info = os.fstat(stream.fileno())
        owner = (0, 0) if os.geteuid() == 0 else (os.geteuid(), os.getegid())
        if (not stat.S_ISREG(info.st_mode) or stat.S_IMODE(info.st_mode) != 0o600
                or (info.st_uid, info.st_gid) != owner):
            raise Refusal('test-accounts.json must be an owner-only (0600) regular file')
        try:
            accounts = json.load(stream)
        except (ValueError, UnicodeError):
            raise Refusal('test-accounts.json is invalid') from None
    if not isinstance(accounts, dict) or accounts.keys() != set(emails):
        raise Refusal('test-accounts.json has an unexpected account list')
    for email, entry in accounts.items():
        expected = {'password', 'totpSecret'} if email in TOTP_ACCOUNTS else {'password'}
        if (not isinstance(entry, dict) or entry.keys() != expected
                or not isinstance(entry['password'], str)
                or not re.fullmatch(r'[A-Za-z0-9_-]{43}', entry['password'])
                or ('totpSecret' in entry and
                    (not isinstance(entry['totpSecret'], str)
                     or not re.fullmatch(r'[A-Z2-7]{32}', entry['totpSecret'])))):
            raise Refusal('test-accounts.json has an invalid credential')
    return accounts


def configure(directory, sha):
    db_password, smtp_password, test_password = (secrets.token_urlsafe(32) for _ in range(3))
    browser_password = secrets.token_urlsafe(32)
    test_db = f'postgres://staging_tests:{test_password}@127.0.0.1:5432/{TEST_DB}'
    env_file(directory, 'compose.env', {
        'STAGING_IMAGE': image_tag(sha), 'STAGING_BROWSER': image_tag(sha, 'browser'),
        'STAGING_BUILDER': image_tag(sha, 'builder'),
    })
    env_file(directory, 'postgres.env', {
        'POSTGRES_USER': 'portal', 'POSTGRES_DB': PORTAL_DB, 'POSTGRES_PASSWORD': db_password,
    })
    portal = dict.fromkeys(OFF, 'false')
    portal.update({f'OTERYN_ACCOUNT_{flag}': 'true' for flag in ACCOUNT_FLAGS})
    portal.update({flag: 'true' for flag in PUBLIC_FLAGS + ADMIN_FLAGS})
    portal['OTERYN_ADMIN_HOST'] = ORIGIN.split('://', 1)[1]
    portal.update({
        'PORTAL_WORLDS_FRESHNESS_SECONDS': '120',
        'PORTAL_WORLD_ENABLED': '1', 'PORTAL_ATLAS_DELIVERY_ENABLED': '1',
        'PORTAL_WORLD_DAILY_WORLDS': ','.join(f'019a0000-0000-7001-8000-{n:012}' for n in range(1, 4)),
        'OTERYN_ATLAS_DIRECTORY': '/app/staging-atlas',
        'PORTAL_PUBLIC_PROJECTIONS_AUTHORITY': 'staging-sample',
        'OTERYN_LCFA_SOURCE_AUTHORITY': 'staging-sample',
        'PORTAL_STAGING_SAMPLE_DATA': 'true',
        'PORTAL_CALCULATORS_EXPERIENCE_SOURCE': 'StagingPolicy|experience',
        'PORTAL_CALCULATORS_TRAINING_SOURCE': 'StagingPolicy|training',
        'OTERYN_ENV': 'testing', 'APP_ENV': 'testing', 'OTERYN_BIND': '0.0.0.0:8080',
        'OTERYN_STATIC_DIR': '/app/static', 'OTERYN_PUBLIC_ORIGIN': ORIGIN,
        'OTERYN_WEBAUTHN_RP_ID': 'localhost', 'OTERYN_MFA_KEY': random_hex_key(),
        'OTERYN_RESET_DELIVERY_KEY': random_hex_key(), 'OTERYN_SMTP_HOST': 'mail',
        'OTERYN_SMTP_PORT': '1025', 'OTERYN_SMTP_TLS_MODE': 'starttls',
        'OTERYN_SMTP_USERNAME': 'portal', 'OTERYN_SMTP_PASSWORD': smtp_password,
        'OTERYN_MAIL_FROM': 'staging@example.invalid', 'OTERYN_SMTP_FROM': 'staging@example.invalid',
        'OTERYN_NOTIFY_UNSUBSCRIBE_KEY': random_hex_key(),
        'OTERYN_NOTIFY_UNSUBSCRIBE_URL': f'{ORIGIN}/en/unsubscribe', 'SSL_CERT_FILE': '/trust/ca.pem',
        'OTERYN_DATABASE_URL': f'postgres://portal:{db_password}@postgres:5432/{PORTAL_DB}',
    })
    env_file(directory, 'portal.env', portal)
    origin = public_origin()
    host = origin.split('://', 1)[1]
    admin = admin_origin(origin)
    # Override the loopback staff settings inherited from portal.env: staff only
    # on the admin host, or no staff panel at all.
    staff = dict.fromkeys(ADMIN_FLAGS, 'true' if admin else 'false')
    staff['OTERYN_ADMIN_HOST'] = admin.split('://', 1)[1] if admin else ''
    env_file(directory, 'public.env', {
        **staff,
        'OTERYN_PUBLIC_ORIGIN': origin,
        'OTERYN_WEBAUTHN_RP_ID': host.split(':', 1)[0],
        'OTERYN_NOTIFY_UNSUBSCRIBE_URL': f'{origin}/en/unsubscribe',
    })
    env_file(directory, 'browser-postgres.env', {
        'POSTGRES_USER': 'portal', 'POSTGRES_DB': 'postgres', 'POSTGRES_PASSWORD': browser_password,
    })
    env_file(directory, 'browser.env', {
        'PORTAL_E2E_ADMIN_DATABASE_URL': f'postgres://portal:{browser_password}@127.0.0.1:5432/postgres',
        'PORTAL_E2E_BINARY': '/usr/local/bin/staging-portal',
        'PORTAL_E2E_SCRATCH': '/tmp/portal-e2e',
    })
    env_file(directory, 'pg-tests.env', {
        'DATABASE_URL': test_db, 'PORTAL_TEST_DATABASE_URL': test_db, 'RUST_TEST_THREADS': '1',
    })
    accounts = test_accounts()
    sensitive = [db_password, smtp_password, test_password, browser_password,
                 portal['OTERYN_MFA_KEY'], portal['OTERYN_RESET_DELIVERY_KEY'],
                 portal['OTERYN_NOTIFY_UNSUBSCRIBE_KEY']]
    sensitive.extend(value for entry in accounts.values() for value in entry.values())
    return smtp_password, test_password, accounts, sensitive


def compose(directory, *args, qualification=False, **kwargs):
    project = f'{PROJECT}-qualification' if qualification else PROJECT
    return run(['docker', 'compose', '-p', project, '--project-directory', str(directory),
                '--env-file', str(directory / 'compose.env'), '-f',
                str(ROOT / 'staging.compose.yml'), *args], **kwargs)


def install_tls(directory, smtp_password):
    with tempfile.TemporaryDirectory(dir=directory) as temporary:
        tls = Path(temporary)
        run(['openssl', 'req', '-x509', '-newkey', 'rsa:2048', '-noenc', '-days', '7',
             '-subj', '/CN=Oracle Staging CA', '-addext', 'basicConstraints=critical,CA:TRUE',
             '-keyout', 'ca-key.pem', '-out', 'ca.pem'], cwd=tls)
        files = [('trust/ca.pem', (tls / 'ca.pem').read_bytes(), 0o444)]
        run(['openssl', 'req', '-newkey', 'rsa:2048', '-noenc', '-subj', '/CN=mail',
             '-keyout', 'key.pem', '-out', 'leaf.csr'], cwd=tls)
        (tls / 'extensions').write_text('subjectAltName=DNS:mail\nextendedKeyUsage=serverAuth\n')
        run(['openssl', 'x509', '-req', '-in', 'leaf.csr', '-CA', 'ca.pem', '-CAkey',
             'ca-key.pem', '-CAcreateserial', '-days', '7', '-extfile', 'extensions',
             '-out', 'leaf.pem'], cwd=tls)
        for name in ('key.pem', 'leaf.pem'):
            files.append((f'mail/{name}', (tls / name).read_bytes(), 0o444))
        files.append(('mail/auth', f'portal:{smtp_password}\n'.encode(), 0o444))
        payload = io.BytesIO()
        with tarfile.open(fileobj=payload, mode='w') as archive:
            for name, data, mode in files:
                item = tarfile.TarInfo(name)
                item.size, item.mode = len(data), mode
                archive.addfile(item, io.BytesIO(data))
        compose(directory, 'run', '--rm', '--no-deps', '-T', 'tls-install', data=payload.getvalue())


def ready():
    connection = http.client.HTTPConnection('127.0.0.1', 18080, timeout=2)
    try:
        connection.request('GET', '/readyz')
        return connection.getresponse().status == 200
    except (OSError, http.client.HTTPException):
        return False
    finally:
        connection.close()


def wait_ready(timeout=90):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if ready():
            return
        time.sleep(1)
    raise Refusal('portal readiness deadline expired')


def seed_accounts(directory, accounts):
    for email, entry in accounts.items():
        args = {'email': email, 'password': entry['password'],
                'verified': email != 'player-new@staging.invalid'}
        if email in STAFF:
            args['role'] = STAFF[email]
        seeded = seed(directory, 'account', args, output=True)
        if 'totpSecret' in entry:
            seed(directory, 'account-mfa', {'accountId': json.loads(seeded)['accountId'], **entry})


def seed(directory, entity, args, *, output=False):
    return compose(directory, 'run', '--rm', '--no-deps', '-T', 'portal',
                   'seed', entity, '-', data=json.dumps(args).encode(), output=output)


def replace_stack(directory, sha, smtp_password, accounts, sample_sql):
    compose(directory, 'down', '--remove-orphans', '--timeout', '15')
    for suffix in ('data', 'trust', 'mail_tls', 'browser_tls'):
        volume = f'{PROJECT}_{suffix}'
        if run(['docker', 'volume', 'inspect', volume], check=False).returncode == 0:
            run(['docker', 'volume', 'rm', volume])
    write(ROOT / 'run' / 'current', sha + '\n')
    install_tls(directory, smtp_password)
    compose(directory, 'up', '-d', '--wait', '--wait-timeout', '90', 'postgres', 'mail')
    oneoff = ('run', '--rm', '--no-deps', '-T', 'portal')
    compose(directory, *oneoff, 'migrate')
    compose(directory, *oneoff, 'install-legal-pages')
    seed_accounts(directory, accounts)
    psql = ('exec', '-T', 'postgres', 'psql', '-X', '-v', 'ON_ERROR_STOP=1',
            '-U', 'portal', '-d', PORTAL_DB)
    compose(directory, *psql, data=REGISTRATION_POLICY, timeout=600)
    compose(directory, *psql, data=sample_sql)
    compose(directory, 'up', '-d', 'portal', 'portal-public', 'worker', 'ingress', 'sample-feed')
    wait_ready()


# Use the CI launcher on a disposable PostgreSQL cluster in the browser's
# network namespace. Neither service joins the deployed portal's network.
BROWSER = r"""
const fs = require('node:fs');
const https = require('node:https');
const { Client } = require('pg');
const { spawn } = require('node:child_process');
const { setTimeout: delay } = require('node:timers/promises');

const scratch = process.env.PORTAL_E2E_SCRATCH;
const envFile = scratch + '/env.json';
const authDirectory = scratch + '/auth';
const reportFile = process.env.PLAYWRIGHT_JSON_OUTPUT_FILE;
const configFile = '/tmp/staging.config.mjs';

function portalGet(path, headers = {}) {
  return new Promise(resolve => {
    const request = https.get({
      hostname: 'localhost', family: 4, port: 18443, path, headers,
      ca: fs.readFileSync(scratch + '/ca.pem'), timeout: 2000,
    }, response => {
      response.resume();
      resolve(response.statusCode);
    });
    request.on('timeout', () => request.destroy());
    request.on('error', () => resolve(0));
  });
}

async function keepStaffFixtureActive() {
  try {
    const {cookies} = JSON.parse(fs.readFileSync(authDirectory + '/staff-pl-1440.json', 'utf8'));
    const session = cookies.find(cookie => cookie.name === '__Host-oteryn_session' &&
      cookie.domain === 'localhost' && /^[a-f0-9]{64}$/.test(cookie.value));
    if (session) await portalGet('/admin', {cookie: session.name + '=' + session.value});
  } catch {
    // The staff bootstrap test writes the private fixture after signing in.
    return;
  }
}

async function seedPasswordRanges() {
  const { PORTAL_E2E_DATABASE_URL } = JSON.parse(fs.readFileSync(envFile, 'utf8'));
  const client = new Client({
    connectionString: PORTAL_E2E_DATABASE_URL,
    connectionTimeoutMillis: 2000,
    statement_timeout: 600_000,
  });
  try {
    await client.connect();
    await client.query(process.argv[2]);
  } finally {
    await client.end();
  }
}

function startupFailure(message) {
  process.stdout.write(JSON.stringify({
    stats: {expected: 0, unexpected: 0, flaky: 0, skipped: 0},
    suites: [], errors: [{message}],
  }));
  return 1;
}

function publish(report) {
  // Assertion messages, attachments and stdout may contain generated credentials.
  // Send only test metadata and a fixed classification for each failure.
  function suites(input) {
    return input.map(suite => ({
      suites: suites(suite.suites ?? []),
      specs: (suite.specs ?? []).map(spec => ({
        file: spec.file, title: spec.title,
        tests: spec.tests.map(test => ({
          status: test.status, projectName: test.projectName,
          results: test.results.slice(-1).map(result => ({
            error: {message: {
              timedOut: 'test timed out', failed: 'test failed', interrupted: 'test interrupted',
            }[result.status] ?? 'test did not pass'},
          })),
        })),
      })),
    }));
  }
  process.stdout.write(JSON.stringify({
    stats: report.stats, suites: suites(report.suites),
    errors: report.errors.map(() => ({message: 'Playwright runner error'})),
  }));
}

async function start() {
  const launcher = spawn(process.execPath, ['local.mjs'], {stdio: 'ignore'});
  const stopped = new Promise(resolve => launcher.once('close', resolve));
  try {
    const deadline = Date.now() + 180_000;
    while (!fs.existsSync(envFile) || await portalGet('/en') !== 200) {
      if (launcher.exitCode !== null || launcher.signalCode !== null || Date.now() >= deadline) {
        return startupFailure('CI fixture stack did not become ready');
      }
      await delay(1000);
    }
    try {
      await seedPasswordRanges();
    } catch {
      return startupFailure('Offline password-range fixtures did not load');
    }
    fs.writeFileSync(configFile, `
      import base from '/workspace/e2e/playwright.config.mjs';
      import { chromium } from '/workspace/e2e/node_modules/playwright/index.mjs';
      export default {...base, testDir: '/workspace/e2e', outputDir: '/tmp/artifacts',
        use: {...base.use, trace: 'off', screenshot: 'off', video: 'off',
          launchOptions: {...base.use.launchOptions, executablePath: chromium.executablePath()}}};
    `);
    const args = ['run.mjs', envFile, 'test', '--config=' + configFile,
                  '--project=pl-1440', '--reporter=json'];
    if (process.argv[1]) args.push('--', process.argv[1]);
    fs.mkdirSync(authDirectory, {mode: 0o700});
    const child = spawn(process.execPath, args, {
      env: {...process.env, PORTAL_E2E_AUTH_DIRECTORY: authDirectory},
      stdio: 'ignore',
    });
    const heartbeat = setInterval(keepStaffFixtureActive, 60_000);
    let code;
    try {
      code = await new Promise(resolve => child.once('close', resolve));
    } finally {
      clearInterval(heartbeat);
    }
    if (fs.existsSync(reportFile) && fs.statSync(reportFile).size <= 16 * 1024 * 1024) {
      publish(JSON.parse(fs.readFileSync(reportFile, 'utf8')));
    }
    return code ?? 1;
  } finally {
    launcher.kill('SIGINT');
    await stopped;
  }
}
start().then(code => {process.exitCode = code;}, () => {process.exitCode = 1;});
"""


def safe_line(value, sensitive):
    for secret in sensitive:
        value = value.replace(secret, '[redacted]')
    return ''.join(char if char.isprintable() else ' ' for char in value)[:300]


def playwright_summary(result, sensitive):
    if not result.stdout:
        return False, ['Playwright did not produce a JSON report']
    try:
        report = json.loads(result.stdout)
    except (UnicodeError, json.JSONDecodeError):
        return False, ['Playwright produced an invalid JSON report']
    stats = report['stats']
    passed, failed, skipped = stats['expected'], stats['unexpected'] + stats['flaky'], stats['skipped']
    lines = [f'Playwright passed {passed}, failed {failed}, skipped {skipped}']

    skipped_files = {}

    def visit(suites):
        for suite in suites:
            for spec in suite.get('specs', []):
                where = spec.get('file', '?')
                for test in spec['tests']:
                    if test['status'] in ('unexpected', 'flaky'):
                        attempt = test['results'][-1] if test['results'] else {}
                        error = (attempt.get('error') or {}).get('message') or 'no assertion recorded'
                        first = next((line.strip() for line in error.splitlines() if line.strip()), '')
                        lines.append(safe_line(
                            f"{where}: {spec['title']} [{test['projectName']}] — {first}", sensitive))
                    elif test['status'] == 'skipped':
                        skipped_files[where] = skipped_files.get(where, 0) + 1
            visit(suite.get('suites', []))

    visit(report['suites'])
    for where, count in sorted(skipped_files.items()):
        lines.append(safe_line(f'skipped {count} in {where}', sensitive))
    for error in report.get('errors', []):
        first = next(iter(error.get('message', '').splitlines()), 'Playwright runner error')
        lines.append(safe_line(first, sensitive))
    return result.returncode == 0 and passed > 0 and failed == skipped == 0 and not report.get('errors'), lines


@contextmanager
def qualification_database(directory):
    cleanup = ('rm', '--stop', '--force', '--volumes', 'browser-postgres')
    compose(directory, *cleanup, qualification=True)
    try:
        compose(directory, 'up', '-d', '--wait', '--wait-timeout', '90', 'browser-postgres',
                qualification=True)
        yield
    finally:
        compose(directory, *cleanup, qualification=True)


def playwright(directory, spec, sensitive):
    # Playwright takes positional file filters as regexes. Match the literal spec.
    result = test_container(directory, 'browser', '-e', BROWSER, '--', re.escape(spec),
                            REGISTRATION_POLICY.decode(), timeout=7200)
    if result is None:
        return False, ['Playwright timed out']
    return playwright_summary(result, sensitive)


def test_container(directory, service, *arguments, timeout=3600):
    name = f'{PROJECT}-qualification_{service}_job'
    try:
        return compose(directory, 'run', '--rm', '--no-deps', '-T', '--name', name,
                       service, *arguments, output=True, check=False, timeout=timeout,
                       qualification=True)
    except subprocess.TimeoutExpired:
        run(['docker', 'container', 'rm', '--force', name], check=False)
        return None


CREATE_TEST_ROLE = br"""
\getenv staging_password STAGING_TEST_PASSWORD
CREATE ROLE staging_tests LOGIN CREATEDB PASSWORD :'staging_password';
"""


def pg_tests(directory, crates, password, sensitive):
    compose(directory, 'exec', '-T', '-e', 'STAGING_TEST_PASSWORD', 'browser-postgres', 'psql',
            '-X', '-v', 'ON_ERROR_STOP=1', '-U', 'portal', '-d', 'postgres',
            data=CREATE_TEST_ROLE, env={'STAGING_TEST_PASSWORD': password}, qualification=True)
    lines, passed = [], True
    for crate in crates:
        compose(directory, 'exec', '-T', 'browser-postgres', 'createdb', '-U', 'portal',
                '-O', 'staging_tests', TEST_DB, qualification=True)
        try:
            result = test_container(directory, 'pg-tests', 'test', '--locked', '-p', crate)
            if result is None:
                lines.append(f'PG {crate} FAIL (timed out)')
                passed = False
                continue
            ok = result.returncode == 0
            lines.append(f'PG {crate} {"PASS" if ok else "FAIL"}')
            for line in result.stdout.decode('utf-8', errors='replace').splitlines():
                if match := CARGO_FAILURE.fullmatch(line):
                    lines.append(safe_line(f'  {match[1]}', sensitive))
            passed &= ok
        finally:
            compose(directory, 'exec', '-T', 'browser-postgres', 'dropdb', '-U', 'portal',
                    '--force', TEST_DB, qualification=True)
    return passed, lines


def pointer(name):
    path = ROOT / 'run' / name
    try:
        sha = path.read_text().strip()
    except FileNotFoundError:
        return None
    if not SHA_RE.fullmatch(sha):
        raise Refusal('invalid staging run pointer')
    return sha


def retain(sha):
    parent = ROOT / 'run'
    runs = sorted((path for path in parent.iterdir() if SHA_RE.fullmatch(path.name)),
                  key=lambda path: (path / 'status.json').stat().st_mtime, reverse=True)
    keep = {path.name for path in runs[:3]} | {sha, pointer('current'), active_qualification()}
    for directory in runs:
        old = directory.name
        if old in keep:
            continue
        for target in ('runtime', 'browser', 'builder'):
            image = image_tag(old, target)
            if run(['docker', 'image', 'inspect', image], check=False).returncode == 0:
                verify_image(image, old)
                run(['docker', 'image', 'rm', '--no-prune', image])
        checkout = ROOT / 'src' / old
        if checkout.exists():
            shutil.rmtree(checkout)
        shutil.rmtree(directory)


def progress(directory, sha, phase):
    write(directory / 'status.json', json.dumps({'sha': sha, 'phase': phase}) + '\n')
    with (directory / 'progress.log').open('a') as stream:
        stream.write(phase + '\n')


def run_qualification(directory, checkout, sha, spec, crates, password, sensitive, summary):
    build_context(checkout, pg=bool(crates))
    progress(directory, sha, 'building browser')
    build(checkout, sha, 'browser')
    progress(directory, sha, 'running Playwright')
    with qualification_database(directory):
        browser_ok, lines = playwright(directory, spec, sensitive)
    summary.extend(lines)
    if not crates:
        return browser_ok

    progress(directory, sha, 'building builder')
    build(checkout, sha, 'builder')
    progress(directory, sha, 'running PostgreSQL tests')
    with qualification_database(directory):
        pg_ok, lines = pg_tests(directory, crates, password, sensitive)
    summary.extend(lines)
    return browser_ok and pg_ok


def deploy(directory, sha, spec, crates, lock):
    summary = []
    outcome = 'failed'
    qualifying = ROOT / 'run' / 'qualifying'
    released = False
    sensitive = []
    try:
        progress(directory, sha, 'qualifying source')
        qualify(sha)
        checkout = source(sha)
        build_context(checkout, browser=False)
        require_capacity()
        production_idle()
        progress(directory, sha, 'building runtime')
        build(checkout, sha, 'runtime')
        progress(directory, sha, 'starting fresh staging stack')
        qualify(sha)
        production_idle()
        # Read the verified Git blob, avoiding host filesystem traversal through checkout links.
        sample_sql = git(checkout, 'show', f'{sha}:{SAMPLE_SQL}').encode()
        smtp_password, test_password, accounts, sensitive = configure(directory, sha)
        replace_stack(directory, sha, smtp_password, accounts, sample_sql)
        outcome = 'passed'
        if spec == DEPLOY_ONLY:
            summary.append('Deployed without test runs')
        else:
            retain(sha)
            write(qualifying, sha + '\n')
            fcntl.flock(lock, fcntl.LOCK_UN)
            released = True
            if not run_qualification(directory, checkout, sha, spec, crates, test_password,
                                     sensitive, summary):
                outcome = 'failed'
    except Refusal as error:
        summary.append(str(error))
        outcome = 'failed'
    except Exception as error:
        # External tools and branch code can include credentials in errors.
        summary.append(f'Staging step failed ({type(error).__name__})')
        outcome = 'failed'
    if not released:
        try:
            retain(sha)
        except Exception as error:
            summary.append(f'Staging retention failed ({type(error).__name__})')
            outcome = 'failed'
    try:
        write(directory / 'summary.txt', '\n'.join(safe_line(line, sensitive) for line in summary) + '\n')
        progress(directory, sha, outcome)
    finally:
        if released:
            qualifying.unlink(missing_ok=True)
    return outcome == 'passed'


def observe(command):
    sha = pointer('latest')
    if sha is None:
        print('No staging deployment')
        return 2
    directory = ROOT / 'run' / sha
    status = (directory / 'status.json').read_text()
    if command == 'status':
        active = pointer('current')
        print(json.dumps({'current_sha': active, 'ready': bool(active and ready())}))
        print(status, end='')
    else:
        print((directory / 'progress.log').read_text(), end='')
    summary = directory / 'summary.txt'
    if summary.exists():
        print(summary.read_text(), end='')
    return {'passed': 0, 'failed': 1}.get(json.loads(status)['phase'], 2)


def launch(sha, spec, crates):
    production_idle()
    qualification = qualification_lock() if spec != DEPLOY_ONLY else nullcontext()
    with qualification, deploy_lock() as lock:
        parent = ROOT / 'run'
        parent.mkdir(exist_ok=True)
        if spec == DEPLOY_ONLY and active_qualification() == sha:
            raise Refusal('this revision is already being qualified on staging')
        if spec != DEPLOY_ONLY:
            (parent / 'qualifying').unlink(missing_ok=True)
        if any(path.name != sha and SHA_RE.fullmatch(path.name) for path in parent.glob(sha[:8] + '*')):
            raise Refusal('staging image tag collides with a retained SHA')
        directory = parent / sha
        directory.mkdir(exist_ok=True)
        for filename in ('summary.txt', 'progress.log'):
            (directory / filename).unlink(missing_ok=True)
        progress(directory, sha, 'queued')
        write(parent / 'latest', sha + '\n')
        if os.fork() != 0:
            # Closing the parent's descriptor leaves the child's flock held.
            print(f'Staging deployment queued for {sha}', flush=True)
            return 0
        try:
            os.setsid()
            os.chdir(ROOT)
            with open(os.devnull, 'r+b') as null:
                for descriptor in (0, 1, 2):
                    os.dup2(null.fileno(), descriptor)
            os._exit(0 if deploy(directory, sha, spec, crates, lock) else 1)
        finally:
            os._exit(1)


def main(argv=None):
    command, sha, spec, crates = parse(sys.argv[1:] if argv is None else argv)
    if os.geteuid() != 0:
        print('host root required', file=sys.stderr)
        return 1
    os.umask(0o077)
    try:
        return launch(sha, spec, crates) if command == 'deploy' else observe(command)
    except Refusal as error:
        print(str(error), file=sys.stderr)
        return 1
    except OSError as error:
        print(f'Staging host operation failed ({type(error).__name__})', file=sys.stderr)
        return 1


if __name__ == '__main__':
    sys.exit(main())
