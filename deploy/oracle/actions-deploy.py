#!/usr/bin/env python3
"""Root-owned, component-bound SSH forced command; never a general shell.

Install manually after review. A runner may observe its component or deploy the
current protected main after its aggregate gate passes. Host secrets/configuration,
World content, custody and the incarnation controller remain separately installed.
"""
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time
import traceback
import urllib.request

ROOT = Path('/opt/oteryn-oracle')
STATE = ROOT / 'state/actions'
BUILD = Path('/opt/oteryn-build')
REPOS = {'platform': 'Oteryn-Platform', 'game': 'Oteryn-Game'}
SHA = r'[0-9a-f]{40}'


def parse(component, command):
    if component not in REPOS:
        raise ValueError('unknown component')
    if command == 'status':
        return 'status', None, None, None
    match = re.fullmatch(r'deploy (' + SHA + r') ([1-9][0-9]{0,19}) ([1-9][0-9]{0,5})', command)
    if not match:
        raise ValueError('only status or deploy <full protected-main SHA> is allowed')
    return 'deploy', match[1], int(match[2]), int(match[3])


def run(args, *, output=False, env=None, cwd=None, timeout=1800, umask=-1):
    result = subprocess.run(args, check=True, stdout=subprocess.PIPE,
                            stderr=subprocess.STDOUT, text=True, env=env,
                            cwd=cwd, timeout=timeout, umask=umask)
    if output:
        return result.stdout


def api(path):
    request = urllib.request.Request('https://api.github.com/repos/Oteryn/' + path,
                                     headers={'User-Agent': 'oteryn-oracle-deploy',
                                              'Accept': 'application/vnd.github+json'})
    with urllib.request.urlopen(request, timeout=30) as response:
        return json.load(response)


def qualify(component, sha):
    repo = REPOS[component]
    branch = api(repo + '/branches/main')
    if branch.get('protected') is not True or branch['commit']['sha'] != sha:
        raise ValueError('source must equal current protected main')
    checks = api(repo + '/commits/' + sha + '/check-runs?per_page=100')['check_runs']
    gate = component + '-gate'
    # Latest matching check, bound to GitHub Actions and this exact commit.
    matches = [c for c in checks if c['name'] == gate and c['head_sha'] == sha
               and c['app']['slug'] == 'github-actions']
    if not matches or max(matches, key=lambda c: c['id'])['conclusion'] != 'success':
        raise ValueError('protected-main aggregate gate has not passed')


def qualify_workflow(component, sha, run_id, attempt):
    repo = REPOS[component]
    workflow = api(repo + '/actions/runs/' + str(run_id))
    expected = {'head_sha': sha, 'head_branch': 'main', 'event': 'workflow_dispatch',
                'path': '.github/workflows/oracle-operations.yml', 'status': 'in_progress',
                'run_attempt': attempt, 'display_title': 'Oracle deploy ' + sha}
    if any(workflow.get(key) != value for key, value in expected.items()):
        raise ValueError('deployment requires its exact active protected workflow request')
    if workflow['repository']['full_name'] != 'Oteryn/' + repo or workflow['head_repository']['full_name'] != 'Oteryn/' + repo:
        raise ValueError('workflow repository differs')
    jobs = api(repo + '/actions/runs/' + str(run_id) + '/attempts/' + str(attempt) + '/jobs')['jobs']
    # A job is in progress on this runner only after GitHub's deployment
    # environment protections have released it. Status jobs cannot authorize deploy.
    if not any(job['name'] == 'operate' and job['status'] == 'in_progress'
               and job.get('runner_name') == 'oteryn-oracle-' + component for job in jobs):
        raise ValueError('protected operation is not running on its bound runner')


def source(component, sha):
    directory = BUILD / ('Actions-' + REPOS[component] + '-' + sha)
    if not directory.exists():
        run(['git', '-c', 'core.hooksPath=/dev/null', 'clone', '--no-checkout',
             '--filter=blob:none', '--depth=1',
             'https://github.com/Oteryn/' + REPOS[component] + '.git', str(directory)], umask=0o022)
    # These are public tracked build inputs. Their copied image metadata must
    # permit the non-root application to read them. Keep the operator process
    # and every private state/configuration write at its owner-only umask.
    run(['git', '-c', 'core.hooksPath=/dev/null', 'fetch', '--depth=1', 'origin', sha], cwd=directory, umask=0o022)
    run(['git', '-c', 'core.hooksPath=/dev/null', 'checkout', '--detach', sha], cwd=directory, umask=0o022)
    if run(['git', 'rev-parse', 'HEAD'], cwd=directory, output=True).strip() != sha:
        raise ValueError('source identity differs')
    if run(['git', 'status', '--porcelain'], cwd=directory, output=True).strip():
        raise ValueError('source is dirty')
    return directory


def atomic(path, data):
    temporary = path.with_name(path.name + '.actions-next')
    fd = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    with os.fdopen(fd, 'w') as stream:
        stream.write(data)
        stream.flush()
        os.fsync(stream.fileno())
    os.replace(temporary, path)
    parent = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(parent)
    finally:
        os.close(parent)


def reauthorize(component, sha, run_id, attempt):
    qualify(component, sha)
    qualify_workflow(component, sha, run_id, attempt)


def journal(component, sha, phase, **facts):
    path = STATE / (component + '-pending.json')
    previous = json.loads(path.read_text()) if path.exists() else {}
    if previous and previous['sha'] != sha:
        raise ValueError('another release requires reconciliation')
    previous.update(component=component, sha=sha, phase=phase, **facts)
    atomic(path, json.dumps(previous, sort_keys=True))


def contract(directory):
    values = {}
    for line in (directory / 'deploy/synology/release-contract.env').read_text().splitlines():
        if line and not line.startswith('#'):
            key, value = line.split('=', 1)
            values[key] = value
    schema = values.get('OTERYN_SCHEMA_COMPATIBILITY_ID', '')
    accepts = values.get('OTERYN_APP_ACCEPTS_SCHEMA_IDS', '').split(',')
    if (values.get('OTERYN_MIGRATION_POLICY') != 'expand-contract'
            or values.get('PLATFORM_DB_ENGINE') != 'postgres'
            or not re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9._-]{0,127}', schema)
            or schema not in accepts):
        raise ValueError('invalid Platform schema contract')
    current = STATE / 'platform-current.json'
    if current.exists() and json.loads(current.read_text())['schema'] not in accepts:
        raise ValueError('new application does not accept the retained current schema')
    return schema


def compose(source_dir):
    return ['docker', 'compose', '--env-file', str(ROOT / 'platform.env'),
            '-f', str(source_dir / 'deploy/synology/compose.yml'),
            '-f', str(ROOT / 'public-platform.compose.yml'),
            '-f', str(ROOT / 'migrated-storage.compose.yml'),
            '-f', str(ROOT / 'internal-mtls.compose.yml')]


def inspect(name):
    return json.loads(run(['docker', 'inspect', name], output=True))[0]


def status(component, *, allow_pending=False):
    if not allow_pending and (STATE / (component + '-pending.json')).exists():
        raise ValueError('release requires reconciliation')
    if component == 'game':
        state = json.loads((ROOT / 'state/game-controller/current.json').read_text())
        if (ROOT / 'state/game-controller/reconciliation-required').exists() or state['phase'] != 'ready':
            raise ValueError('Game requires reconciliation')
        run(['systemctl', 'is-active', '--quiet', 'oteryn-game.service'])
        names = ['oteryn-oracle-runtime-game-1', 'oteryn-oracle-game-postgres-1']
    else:
        names = ['oteryn-oracle-platform-1', 'oteryn-oracle-gateway-1',
                 'oteryn-oracle-marketplace-scheduler-1', 'oteryn-oracle-postgres-1',
                 'oteryn-oracle-postgres-backup-1']
        for url in ['https://oteryn.com/login', 'https://gateway.oteryn.com/ready']:
            run(['curl', '--fail', '--silent', '--show-error', '--max-time', '20', '-o', '/dev/null', url])
    for name in names:
        info = inspect(name)
        if not info['State']['Running'] or info['State'].get('Health', {}).get('Status', 'healthy') != 'healthy':
            raise ValueError('component is not healthy: ' + name)
        if component == 'game' and name == 'oteryn-oracle-runtime-game-1':
            if info['Config']['Labels']['io.oteryn.launch-cycle'] != state['cycle']:
                raise ValueError('running Game differs from retained ready incarnation')
    print(component + '_ready=true', flush=True)


def verify_current_release(component, sha):
    record = json.loads((STATE / (component + '-current.json')).read_text())
    if record.get('sha') != sha or record.get('phase') != 'ready':
        raise ValueError('receipt differs from retained current release')
    images = ({'oteryn-oracle-runtime-game-1': record['image']} if component == 'game' else
              {'oteryn-oracle-platform-1': record['platform_image'],
               'oteryn-oracle-gateway-1': record['gateway_image'],
               'oteryn-oracle-marketplace-scheduler-1': record['platform_image']})
    if any(inspect(name)['Image'] != identity for name, identity in images.items()):
        raise ValueError('receipt differs from current runtime images')
    if component == 'platform':
        config = run(['docker', 'exec', 'oteryn-oracle-edge-edge-1', 'cat', '/etc/caddy/Caddyfile'], output=True)
        if hashlib.sha256(config.encode()).hexdigest() != record['caddy_sha256']:
            raise ValueError('receipt differs from current public edge configuration')
        if 'mtls_sha256' in record:
            config = run(['docker', 'exec', MTLS, 'cat', MTLS_CONFIG], output=True)
            if hashlib.sha256(config.encode()).hexdigest() != record['mtls_sha256']:
                raise ValueError('receipt differs from current internal mTLS configuration')


def prepare_edge(directory, sha):
    name = 'oteryn-oracle-edge-edge-1'
    temporary = '/tmp/oracle-' + sha + '.Caddyfile'
    run(['docker', 'cp', str(directory / 'deploy/oracle/Caddyfile'), name + ':' + temporary])
    run(['docker', 'exec', name, 'caddy', 'validate', '--config', temporary, '--adapter', 'caddyfile'])


def deploy_edge(directory, sha, run_id, attempt):
    candidate = (directory / 'deploy/oracle/Caddyfile').read_text()
    name = 'oteryn-oracle-edge-edge-1'
    current = run(['docker', 'exec', name, 'cat', '/etc/caddy/Caddyfile'], output=True)
    if current != candidate:
        prepare_edge(directory, sha)
        reauthorize('platform', sha, run_id, attempt)
        info = inspect(name)
        edge_compose = Path(info['Config']['Labels']['com.docker.compose.project.config_files'])
        if edge_compose.parent != ROOT / 'edge':
            raise ValueError('edge compose path differs from installed root-owned boundary')
        # Recreate to acquire the new inode of the atomically replaced bind file.
        atomic(ROOT / 'edge/Caddyfile', candidate)
        run(['docker', 'compose', '-f', str(edge_compose), 'up', '-d', '--no-deps', '--force-recreate', 'edge'])
    observed = run(['docker', 'exec', name, 'cat', '/etc/caddy/Caddyfile'], output=True)
    if observed != candidate:
        raise ValueError('running edge did not acquire the qualified configuration')
    journal('platform', sha, 'schema_known', caddy_sha256=hashlib.sha256(candidate.encode()).hexdigest())


MTLS = 'oteryn-oracle-internal-mtls-1'
MTLS_CONFIG = '/bootstrap/internal-mtls-nginx.conf'


def mtls_bootstrap():
    # The listener mounts the bootstrap volume read-only; write its host directory instead.
    mounts = [m for m in inspect(MTLS)['Mounts'] if m['Destination'] == '/bootstrap']
    if (len(mounts) != 1 or mounts[0].get('Type') != 'volume'
            or mounts[0].get('Name') != 'oteryn-oracle_bootstrap_files'):
        raise ValueError('internal mTLS bootstrap mount differs from installed boundary')
    return Path(mounts[0]['Source'])


def prepare_mtls(directory, sha):
    bootstrap = mtls_bootstrap()
    candidate = (directory / 'deploy/synology/nginx/internal-mtls.conf').read_text()
    for stale in bootstrap.glob('internal-mtls-nginx.*.conf'):
        if re.fullmatch(r'internal-mtls-nginx\.' + SHA + r'\.conf', stale.name):
            stale.unlink()
    staged = bootstrap / ('internal-mtls-nginx.' + sha + '.conf')
    # Root-only like the rest of the bootstrap files; the nginx master reads it as root.
    fd = os.open(staged, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    with os.fdopen(fd, 'w') as stream:
        stream.write(candidate)
        stream.flush()
        os.fsync(stream.fileno())
    run(['docker', 'exec', MTLS, 'nginx', '-t', '-q', '-c', '/bootstrap/' + staged.name])
    return staged


def deploy_mtls(directory, sha):
    # Neither prepare-platform.sh nor this deploy otherwise refreshes the listener's
    # nginx configuration, so new internal routes would stay unreachable (404).
    candidate = (directory / 'deploy/synology/nginx/internal-mtls.conf').read_text()
    bootstrap = mtls_bootstrap()
    staged = bootstrap / ('internal-mtls-nginx.' + sha + '.conf')
    live = bootstrap / Path(MTLS_CONFIG).name
    if staged.read_text() != candidate:
        raise ValueError('staged internal mTLS configuration differs from qualified source')
    previous = live.read_text()
    if previous != candidate:
        os.replace(staged, live)
        try:
            run(['docker', 'exec', MTLS, 'nginx', '-t', '-q', '-c', MTLS_CONFIG])
        except subprocess.CalledProcessError:
            # Never leave a file the listener cannot start with on its next restart.
            atomic(live, previous)
            raise
    staged.unlink(missing_ok=True)
    # Reload even when the file already matches: an earlier attempt may have replaced
    # it and failed before the running listener loaded it.
    run(['docker', 'exec', MTLS, 'nginx', '-c', MTLS_CONFIG, '-s', 'reload'])
    if run(['docker', 'exec', MTLS, 'cat', MTLS_CONFIG], output=True) != candidate:
        raise ValueError('internal mTLS listener did not acquire the qualified configuration')
    journal('platform', sha, 'schema_known', mtls_sha256=hashlib.sha256(candidate.encode()).hexdigest())


def platform_deploy(directory, sha, run_id, attempt):
    image = 'oteryn-platform:' + sha + '-arm64'
    gateway = 'oteryn-gateway:' + sha + '-arm64'
    for name, dockerfile in [(image, 'platform'), (gateway, 'gateway')]:
        run(['docker', 'build', '--build-arg', 'OTERYN_BUILD_SHA=' + sha,
             '--label', 'org.opencontainers.image.revision=' + sha,
             '-f', 'deploy/synology/docker/' + dockerfile + '.Dockerfile',
             '-t', name, '.'], cwd=directory)
        if inspect_image(name)['Architecture'] != 'arm64':
            raise ValueError('image architecture differs')
    # Inherit the actual image USER and WORKDIR, with no keys, mounts or network.
    # Refuse an unreadable application before backup, maintenance or migration.
    run(['docker', 'run', '--rm', '--network', 'none', '--entrypoint', 'php', image,
         '-r', 'foreach (["artisan", "bootstrap/app.php", "vendor/autoload.php"] as $file) { if (!is_readable($file)) { exit(1); } }'],
        timeout=120)
    reauthorize('platform', sha, run_id, attempt)
    schema = contract(directory)
    prepare_edge(directory, sha)
    prepare_mtls(directory, sha)
    # Preserve every private setting; change only immutable image identities.
    path = ROOT / 'platform.env'
    before = path.read_text()
    old_files = inspect('oteryn-oracle-platform-1')['Config']['Labels']['com.docker.compose.project.config_files']
    old_source = Path(old_files.split(',')[0]).parents[2]
    previous = compose(old_source)
    current = compose(directory)
    env = dict(os.environ, OTERYN_ENV_FILE=str(path))
    run(['bash', str(old_source / 'deploy/synology/scripts/backup-postgres.sh'), 'run', 'dump'], env=env)
    reauthorize('platform', sha, run_id, attempt)
    journal('platform', sha, 'prepared', schema=schema,
            previous_image=inspect('oteryn-oracle-platform-1')['Image'],
            platform_image=inspect_image(image)['Id'], gateway_image=inspect_image(gateway)['Id'])
    run(previous + ['exec', '-T', 'platform', 'php', 'artisan', 'down', '--retry=60'])
    run(previous + ['stop', 'platform', 'gateway', 'marketplace-scheduler'])
    # No automatic rollback across a possibly committed schema migration.
    atomic(STATE / (sha + '-platform.env.before'), before)
    settings = {'PLATFORM_IMAGE': image, 'GATEWAY_IMAGE': gateway, 'GATEWAY_VERSION': 'oracle-' + sha}
    lines = before.splitlines()
    for key, value in settings.items():
        if sum(line.startswith(key + '=') for line in lines) != 1:
            raise ValueError('expected exactly one image setting')
        lines = [key + '=' + value if line.startswith(key + '=') else line for line in lines]
    atomic(path, '\n'.join(lines) + '\n')
    script = ('source "$1/deploy/synology/scripts/lib.sh"; '
              'load_oteryn_env_file "$OTERYN_ENV_FILE"; shift; '
              'oteryn_platform_migrate_postgres "$@"')
    reauthorize('platform', sha, run_id, attempt)
    journal('platform', sha, 'schema_unknown')
    run(['bash', '-euc', script, 'oracle-migrate', str(directory), *current], env=env)
    journal('platform', sha, 'schema_known')
    run(current + ['up', '-d', '--no-deps', '--force-recreate', 'platform', 'gateway', 'marketplace-scheduler'])
    run(current + ['exec', '-T', 'platform', 'php', 'artisan', 'up'])
    deploy_edge(directory, sha, run_id, attempt)
    deploy_mtls(directory, sha)
    return {'oteryn-oracle-platform-1': inspect_image(image)['Id'],
            'oteryn-oracle-gateway-1': inspect_image(gateway)['Id'],
            'oteryn-oracle-marketplace-scheduler-1': inspect_image(image)['Id']}


def inspect_image(image):
    return json.loads(run(['docker', 'image', 'inspect', image], output=True))[0]


def game_deploy(directory, sha, run_id, attempt):
    image = 'oteryn-game:' + sha + '-arm64'
    run(['docker', 'build', '--build-arg', 'GAME_SOURCE_SHA=' + sha,
         '-f', str(ROOT / 'game.Dockerfile'), '-t', image, str(directory)], timeout=7200)
    info = inspect_image(image)
    if info['Architecture'] != 'arm64' or info['Config']['Labels']['org.opencontainers.image.revision'] != sha:
        raise ValueError('Game image provenance differs')
    reauthorize('game', sha, run_id, attempt)
    status('game')
    override = STATE / (sha + '-game-migrate.json')
    atomic(override, json.dumps({'services': {'migrate': {'image': image}}}))
    run(['systemctl', 'start', 'oteryn-game-backup.service'])
    reauthorize('game', sha, run_id, attempt)
    previous = json.loads((ROOT / 'state/game-controller/current.json').read_text())
    journal('game', sha, 'prepared', image=info['Id'], previous_cycle=previous['cycle'])
    run(['systemctl', 'stop', 'oteryn-game.service'])
    if inspect('oteryn-oracle-runtime-game-1')['State']['Running']:
        raise ValueError('old Game is still running; migration refused')
    # Existing configuration, content, custody and durable controller stay intact.
    path = ROOT / 'game-runtime.compose.yml'
    before = path.read_text()
    atomic(STATE / (sha + '-game.compose.before'), before)
    changed, count = re.subn(r'(?m)^(    image:) oteryn-game:[^\n]+$', r'\1 ' + image, before)
    if count != 2:
        raise ValueError('expected exactly Game and ops image declarations')
    reauthorize('game', sha, run_id, attempt)
    journal('game', sha, 'schema_unknown')
    run(['docker', 'compose', '-f', str(ROOT / 'game-db.compose.yml'), '-f', str(override),
         'run', '--rm', '-T', '--no-deps', 'migrate'])
    journal('game', sha, 'schema_known')
    atomic(path, changed)
    run(['systemctl', 'start', 'oteryn-game.service'])
    return {'oteryn-oracle-runtime-game-1': info['Id']}


def main():
    os.umask(0o077)
    if os.geteuid() != 0:
        raise ValueError('host root required')
    component = sys.argv[1] if len(sys.argv) == 2 else ''
    action, sha, run_id, attempt = parse(component, os.environ.get('SSH_ORIGINAL_COMMAND', ''))
    STATE.mkdir(mode=0o700, parents=True, exist_ok=True)
    with (STATE / 'deployment.lock').open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        if action == 'deploy':
            if (STATE / (component + '-pending.json')).exists():
                raise ValueError('retained release requires explicit reconciliation')
            receipt = STATE / (sha + '-' + component + '.json')
            reauthorize(component, sha, run_id, attempt)
            if receipt.exists():
                verify_current_release(component, sha)
                status(component)
                return
            directory = source(component, sha)
            # Recheck after a long build/source transfer before accepting effects.
            qualify(component, sha)
            if component == 'platform':
                images = platform_deploy(directory, sha, run_id, attempt)
            else:
                images = game_deploy(directory, sha, run_id, attempt)
            for _ in range(90):
                try:
                    status(component, allow_pending=True)
                    if any(inspect(name)['Image'] != identity for name, identity in images.items()):
                        raise ValueError('running image differs from qualified build')
                    if component == 'game':
                        before = json.loads((STATE / 'game-pending.json').read_text())
                        after = json.loads((ROOT / 'state/game-controller/current.json').read_text())
                        if before['previous_cycle'] == after['cycle']:
                            raise ValueError('Game incarnation did not advance')
                    break
                except (ValueError, subprocess.SubprocessError):
                    time.sleep(2)
            else:
                raise ValueError('deployment readiness deadline exceeded')
            journal(component, sha, 'ready')
            pending = STATE / (component + '-pending.json')
            atomic(STATE / (component + '-current.json'), pending.read_text())
            atomic(STATE / (sha + '-' + component + '.json'), json.dumps({'component': component, 'sha': sha, 'ready': True}))
            pending.unlink()
        else:
            status(component)


if __name__ == '__main__':
    try:
        main()
    except Exception as error:
        # Captured command output may contain private operational data. Keep it
        # host-private; SSH/GitHub receives only the exception class.
        if STATE.is_dir():
            with (STATE / ('failure-' + str(time.time_ns()) + '.log')).open('x') as log:
                traceback.print_exc(file=log)
                if isinstance(error, subprocess.CalledProcessError):
                    log.write(error.stdout or '')
        print('oracle_operation_failed=' + type(error).__name__, file=sys.stderr)
        sys.exit(1)
