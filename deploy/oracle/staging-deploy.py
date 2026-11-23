#!/usr/bin/env python3
"""Root-operated Oracle staging, with checkout code confined to staging containers."""
import argparse
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
BRANCHES = 'https://api.github.com/repos/Oteryn/Oteryn-Platform/branches'
REVISION = 'org.opencontainers.image.revision'
PLAYWRIGHT_VERSION = '1.58.2'
SHA_RE = re.compile(r'[0-9a-f]{40}')
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
    'OTERYN_LCFA_ENABLED', 'OTERYN_ACCOUNT_BALANCE_ENABLED', 'OTERYN_PREMIUM_ENABLED',
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
    deploy.add_argument('spec', nargs='?', default='')
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


def deploy_lock():
    ROOT.mkdir(mode=0o700, parents=True, exist_ok=True)
    lock = (ROOT / 'deployment.lock').open('ab')
    try:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BaseException as error:
        lock.close()
        if isinstance(error, BlockingIOError):
            raise Refusal('another staging deploy holds the lock') from None
        raise
    return lock


def branches(page):
    request = urllib.request.Request(
        f'{BRANCHES}?per_page=100&page={page}',
        headers={'Accept': 'application/vnd.github+json', 'User-Agent': PROJECT,
                 'Cache-Control': 'no-cache'},
    )
    # Ignore operator proxy settings and authenticate the API with system TLS.
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    with opener.open(request, timeout=15) as response:
        if response.geturl() != request.full_url:
            raise Refusal('unexpected GitHub branches endpoint')
        return json.load(response)


def qualify(sha):
    page = 1
    while True:
        batch = branches(page)
        if not isinstance(batch, list):
            raise Refusal('unexpected GitHub branch response')
        if any(branch['name'].startswith(('codex/', 'staging/')) and branch['commit']['sha'] == sha
               for branch in batch):
            return
        if len(batch) < 100:
            raise Refusal('source must be the current head of a codex/ or staging/ branch')
        page += 1


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


def build_context(directory, pg=False):
    portal = directory / 'portal-rs'
    e2e = portal / 'e2e'
    lock = e2e / 'package-lock.json'
    cargo_lock = portal / 'Cargo.lock'
    paths = [portal, e2e, lock, cargo_lock, e2e / 'package.json']
    if pg:
        paths += [directory / name for name in ('docs', 'docs/contracts', 'deploy', 'deploy/oracle')]
    if any(path.is_symlink() for path in paths):
        raise Refusal('staging build inputs must be within the checkout')
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
        args += ['--build-context', f'e2e={portal / "e2e"}']
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


def public_origin():
    path = ROOT / 'public-origin'
    if not path.exists():
        return ORIGIN
    value = path.read_text().strip()
    if not PUBLIC_ORIGIN_RE.fullmatch(value):
        raise Refusal('public-origin must be https://<hostname>')
    return value


def random_hex_key():
    return hashlib.sha256(secrets.token_urlsafe(32).encode()).hexdigest()


def configure(directory, sha):
    db_password, smtp_password, test_password = (secrets.token_urlsafe(32) for _ in range(3))
    local_db = f'postgres://portal:{db_password}@127.0.0.1:5432/{PORTAL_DB}'
    test_db = f'postgres://staging_tests:{test_password}@postgres:5432/{TEST_DB}'
    env_file(directory, 'compose.env', {
        'STAGING_IMAGE': image_tag(sha), 'STAGING_BROWSER': image_tag(sha, 'browser'),
        'STAGING_BUILDER': image_tag(sha, 'builder'),
    })
    env_file(directory, 'postgres.env', {
        'POSTGRES_USER': 'portal', 'POSTGRES_DB': PORTAL_DB, 'POSTGRES_PASSWORD': db_password,
    })
    portal = dict.fromkeys(OFF, 'false')
    portal.update({f'OTERYN_ACCOUNT_{flag}': 'true' for flag in ACCOUNT_FLAGS})
    portal.update({
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
    env_file(directory, 'public.env', {
        'OTERYN_PUBLIC_ORIGIN': origin,
        'OTERYN_WEBAUTHN_RP_ID': origin.split('://', 1)[1].split(':', 1)[0],
        'OTERYN_NOTIFY_UNSUBSCRIBE_URL': f'{origin}/en/unsubscribe',
    })
    env_file(directory, 'browser.env', {
        'OTERYN_ENV': 'testing', 'APP_ENV': 'testing', 'OTERYN_PUBLIC_ORIGIN': ORIGIN,
        'OTERYN_STATIC_DIR': '/app/static', 'PORTAL_E2E_ORIGIN': ORIGIN,
        'PORTAL_E2E_BINARY': '/usr/local/bin/staging-portal',
        'OTERYN_DATABASE_URL': local_db, 'PORTAL_E2E_DATABASE_URL': local_db,
    })
    env_file(directory, 'pg-tests.env', {
        'DATABASE_URL': test_db, 'PORTAL_TEST_DATABASE_URL': test_db, 'RUST_TEST_THREADS': '1',
    })
    seed_password = secrets.token_urlsafe(32)
    sensitive = [db_password, smtp_password, test_password, seed_password,
                 portal['OTERYN_MFA_KEY'], portal['OTERYN_RESET_DELIVERY_KEY'],
                 portal['OTERYN_NOTIFY_UNSUBSCRIBE_KEY']]
    return smtp_password, test_password, seed_password, sensitive


def compose(directory, *args, **kwargs):
    return run(['docker', 'compose', '-p', PROJECT, '--project-directory', str(directory),
                '--env-file', str(directory / 'compose.env'), '-f',
                str(ROOT / 'staging.compose.yml'), *args], **kwargs)


def install_tls(directory, smtp_password):
    with tempfile.TemporaryDirectory(dir=directory) as temporary:
        tls = Path(temporary)
        run(['openssl', 'req', '-x509', '-newkey', 'rsa:2048', '-noenc', '-days', '7',
             '-subj', '/CN=Oracle Staging CA', '-addext', 'basicConstraints=critical,CA:TRUE',
             '-keyout', 'ca-key.pem', '-out', 'ca.pem'], cwd=tls)
        files = [('trust/ca.pem', (tls / 'ca.pem').read_bytes(), 0o444)]
        for service, host in (('mail', 'mail'), ('browser', 'localhost')):
            run(['openssl', 'req', '-newkey', 'rsa:2048', '-noenc', '-subj', f'/CN={host}',
                 '-keyout', 'key.pem', '-out', 'leaf.csr'], cwd=tls)
            (tls / 'extensions').write_text(f'subjectAltName=DNS:{host}\nextendedKeyUsage=serverAuth\n')
            run(['openssl', 'x509', '-req', '-in', 'leaf.csr', '-CA', 'ca.pem', '-CAkey',
                 'ca-key.pem', '-CAcreateserial', '-days', '7', '-extfile', 'extensions',
                 '-out', 'leaf.pem'], cwd=tls)
            for name in ('key.pem', 'leaf.pem'):
                files.append((f'{service}/{name}', (tls / name).read_bytes(), 0o444))
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


def replace_stack(directory, sha, smtp_password, seed_password):
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
    compose(directory, *oneoff, 'seed', 'account',
            json.dumps({'email': 'player1@example.invalid', 'password': seed_password}))
    compose(directory, 'up', '-d', 'portal', 'portal-public', 'worker', 'ingress')
    wait_ready()


# Loopback services mirror the repository browser fixtures. Branch Node code
# runs as a confined container user on the internal staging network.
BROWSER = r"""
const fs = require('node:fs');
const https = require('node:https');
const http = require('node:http');
const net = require('node:net');
const { spawn, execFileSync } = require('node:child_process');

async function proxy(req, res) {
  if (req.url === '/__mail') {
    try {
      const mail = 'http://mail:8025/api/v1/';
      const { messages } = await (await fetch(mail + 'messages')).json();
      const captured = await Promise.all((messages ?? []).map(async message => ({
        to: message.To.map(recipient => recipient.Address),
        raw: await (await fetch(new URL('message/' + encodeURIComponent(message.ID) + '/raw', mail))).text(),
      })));
      res.setHeader('content-type', 'application/json');
      res.end(JSON.stringify(captured.reverse()));
    } catch {
      res.writeHead(502).end();
    }
    return;
  }
  const upstream = http.request({
    hostname: 'portal', port: 8080, path: req.url, method: req.method, headers: req.headers,
  }, result => { res.writeHead(result.statusCode, result.headers); result.pipe(res); });
  upstream.on('error', () => res.writeHead(502).end());
  req.pipe(upstream);
}

async function start() {
  const nss = '/tmp/.pki/nssdb';
  fs.mkdirSync(nss, {recursive: true});
  execFileSync('certutil', ['-N', '--empty-password', '-d', 'sql:' + nss]);
  execFileSync('certutil', ['-A', '-n', 'Oracle staging', '-t', 'C,,',
                           '-i', '/trust/ca.pem', '-d', 'sql:' + nss]);
  const web = https.createServer({
    cert: fs.readFileSync('/tls/leaf.pem'), key: fs.readFileSync('/tls/key.pem'),
  }, proxy);
  const pg = net.createServer(local => {
    const remote = net.connect(5432, 'postgres');
    local.on('error', () => remote.destroy());
    remote.on('error', () => local.destroy());
    local.pipe(remote).pipe(local);
  });
  const listen = (server, port) => new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(port, '127.0.0.1', resolve);
  });
  await Promise.all([listen(web, 18443), listen(pg, 5432)]);
  fs.writeFileSync('/tmp/staging.config.mjs', `
    import base from '/workspace/e2e/playwright.config.mjs';
    import { chromium } from '/workspace/e2e/node_modules/playwright/index.mjs';
    export default {...base, testDir: '/workspace/e2e', outputDir: '/tmp/artifacts',
      use: {...base.use, ignoreHTTPSErrors: false,
        launchOptions: {...base.use.launchOptions, executablePath: chromium.executablePath()}}};
  `);
  const args = ['node_modules/@playwright/test/cli.js', 'test', '--config=/tmp/staging.config.mjs',
                '--reporter=json'];
  if (process.argv[1]) args.push('--', process.argv[1]);
  const child = spawn(process.execPath, args, {stdio: 'ignore'});
  child.on('exit', code => {
    pg.close();
    web.close();
    const report = '/tmp/report.json';
    if (fs.existsSync(report) && fs.statSync(report).size <= 16 * 1024 * 1024) {
      process.stdout.write(fs.readFileSync(report), () => process.exit(code ?? 1));
    } else {
      process.exit(1);
    }
  });
}
start().catch(() => process.exit(1));
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

    def visit(suites):
        for suite in suites:
            for spec in suite.get('specs', []):
                for test in spec['tests']:
                    if test['status'] in ('unexpected', 'flaky'):
                        attempt = test['results'][-1] if test['results'] else {}
                        error = (attempt.get('error') or {}).get('message') or 'no assertion recorded'
                        first = next((line.strip() for line in error.splitlines() if line.strip()), '')
                        lines.append(safe_line(f"{spec['title']} [{test['projectName']}] — {first}", sensitive))
            visit(suite.get('suites', []))

    visit(report['suites'])
    for error in report.get('errors', []):
        first = next(iter(error.get('message', '').splitlines()), 'Playwright runner error')
        lines.append(safe_line(first, sensitive))
    return result.returncode == 0 and passed > 0 and failed == skipped == 0 and not report.get('errors'), lines


def playwright(directory, spec, sensitive):
    # Playwright takes positional file filters as regexes. Match the literal spec.
    result = test_container(directory, 'browser', '-e', BROWSER, '--', re.escape(spec))
    if result is None:
        return False, ['Playwright timed out']
    return playwright_summary(result, sensitive)


def test_container(directory, service, *arguments):
    name = f'{PROJECT}_{service}_job'
    try:
        return compose(directory, 'run', '--rm', '--no-deps', '-T', '--name', name,
                       service, *arguments, output=True, check=False, timeout=3600)
    except subprocess.TimeoutExpired:
        run(['docker', 'container', 'rm', '--force', name], check=False)
        return None


CREATE_TEST_ROLE = br"""
\getenv staging_password STAGING_TEST_PASSWORD
CREATE ROLE staging_tests LOGIN CREATEDB PASSWORD :'staging_password';
REVOKE CONNECT ON DATABASE portal_e2e_staging FROM PUBLIC;
"""


def pg_tests(directory, crates, password, sensitive):
    compose(directory, 'exec', '-T', '-e', 'STAGING_TEST_PASSWORD', 'postgres', 'psql',
            '-X', '-v', 'ON_ERROR_STOP=1', '-U', 'portal', '-d', 'postgres',
            data=CREATE_TEST_ROLE, env={'STAGING_TEST_PASSWORD': password})
    lines, passed = [], True
    for crate in crates:
        compose(directory, 'exec', '-T', 'postgres', 'createdb', '-U', 'portal',
                '-O', 'staging_tests', TEST_DB)
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
            compose(directory, 'exec', '-T', 'postgres', 'dropdb', '-U', 'portal', '--force', TEST_DB)
    return passed, lines


def pointer(name):
    path = ROOT / 'run' / name
    if path.exists():
        sha = path.read_text().strip()
        if not SHA_RE.fullmatch(sha):
            raise Refusal('invalid staging run pointer')
        return sha
    return None


def retain(sha):
    parent = ROOT / 'run'
    runs = sorted((path for path in parent.iterdir() if SHA_RE.fullmatch(path.name)),
                  key=lambda path: (path / 'status.json').stat().st_mtime, reverse=True)
    keep = {path.name for path in runs[:3]} | {sha, pointer('current')}
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


def deploy(directory, sha, spec, crates):
    summary = []
    outcome = 'failed'
    try:
        progress(directory, sha, 'qualifying source')
        qualify(sha)
        checkout = source(sha)
        build_context(checkout, pg=bool(crates))
        require_capacity()
        production_idle()
        for target in ('runtime', 'browser', 'builder') if crates else ('runtime', 'browser'):
            progress(directory, sha, f'building {target}')
            build(checkout, sha, target)
        progress(directory, sha, 'starting fresh staging stack')
        qualify(sha)
        production_idle()
        smtp_password, test_password, seed_password, sensitive = configure(directory, sha)
        replace_stack(directory, sha, smtp_password, seed_password)
        progress(directory, sha, 'running Playwright')
        browser_ok, lines = playwright(directory, spec, sensitive)
        summary.extend(lines)
        pg_ok = True
        if crates:
            progress(directory, sha, 'running PostgreSQL tests')
            pg_ok, lines = pg_tests(directory, crates, test_password, sensitive)
            summary.extend(lines)
        outcome = 'passed' if browser_ok and pg_ok else 'failed'
    except Refusal as error:
        summary.append(str(error))
    except Exception as error:
        # External tools and branch code can include credentials in errors.
        summary.append(f'Staging step failed ({type(error).__name__})')
    try:
        retain(sha)
    except Exception as error:
        summary.append(f'Staging retention failed ({type(error).__name__})')
        outcome = 'failed'
    write(directory / 'summary.txt', '\n'.join(summary) + '\n')
    progress(directory, sha, outcome)
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
    with deploy_lock():
        parent = ROOT / 'run'
        parent.mkdir(exist_ok=True)
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
            os._exit(0 if deploy(directory, sha, spec, crates) else 1)
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
