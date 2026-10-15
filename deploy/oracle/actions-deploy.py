#!/usr/bin/env python3
"""Root-owned, component-bound SSH forced command; never a general shell.

Install manually after review. A runner may observe its component or deploy the
current protected main after its aggregate gate passes. Host secrets/configuration,
World content, custody and the incarnation controller remain separately installed.
"""
import fcntl
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
        return 'status', None
    match = re.fullmatch(r'deploy (' + SHA + ')', command)
    if not match:
        raise ValueError('only status or deploy <full protected-main SHA> is allowed')
    return 'deploy', match[1]


def run(args, *, output=False, env=None, cwd=None, timeout=1800):
    result = subprocess.run(args, check=True, stdout=subprocess.PIPE,
                            stderr=subprocess.STDOUT, text=True, env=env,
                            cwd=cwd, timeout=timeout)
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
    if api(repo + '/branches/main')['commit']['sha'] != sha:
        raise ValueError('source must equal current protected main')
    checks = api(repo + '/commits/' + sha + '/check-runs?per_page=100')['check_runs']
    gate = component + '-gate'
    # Latest matching check, bound to GitHub Actions and this exact commit.
    matches = [c for c in checks if c['name'] == gate and c['head_sha'] == sha
               and c['app']['slug'] == 'github-actions']
    if not matches or max(matches, key=lambda c: c['id'])['conclusion'] != 'success':
        raise ValueError('protected-main aggregate gate has not passed')


def source(component, sha):
    directory = BUILD / ('Actions-' + REPOS[component] + '-' + sha)
    if not directory.exists():
        run(['git', '-c', 'core.hooksPath=/dev/null', 'clone', '--no-checkout',
             '--filter=blob:none', '--depth=1',
             'https://github.com/Oteryn/' + REPOS[component] + '.git', str(directory)])
    run(['git', '-c', 'core.hooksPath=/dev/null', 'fetch', '--depth=1', 'origin', sha], cwd=directory)
    run(['git', '-c', 'core.hooksPath=/dev/null', 'checkout', '--detach', sha], cwd=directory)
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


def compose(source_dir):
    return ['docker', 'compose', '--env-file', str(ROOT / 'platform.env'),
            '-f', str(source_dir / 'deploy/synology/compose.yml'),
            '-f', str(ROOT / 'public-platform.compose.yml'),
            '-f', str(ROOT / 'migrated-storage.compose.yml'),
            '-f', str(ROOT / 'internal-mtls.compose.yml')]


def inspect(name):
    return json.loads(run(['docker', 'inspect', name], output=True))[0]


def status(component):
    if component == 'game':
        state = json.loads((ROOT / 'state/game-controller/current.json').read_text())
        if (ROOT / 'state/game-controller/reconciliation-required').exists() or state['phase'] != 'ready':
            raise ValueError('Game requires reconciliation')
        run(['systemctl', 'is-active', '--quiet', 'oteryn-game.service'])
        names = ['oteryn-oracle-runtime-game-1', 'oteryn-oracle-game-postgres-1']
    else:
        names = ['oteryn-oracle-platform-1', 'oteryn-oracle-gateway-1',
                 'oteryn-oracle-postgres-1', 'oteryn-oracle-postgres-backup-1']
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


def platform_deploy(directory, sha):
    image = 'oteryn-platform:' + sha + '-arm64'
    gateway = 'oteryn-gateway:' + sha + '-arm64'
    for name, dockerfile in [(image, 'platform'), (gateway, 'gateway')]:
        run(['docker', 'build', '--build-arg', 'OTERYN_BUILD_SHA=' + sha,
             '--label', 'org.opencontainers.image.revision=' + sha,
             '-f', 'deploy/synology/docker/' + dockerfile + '.Dockerfile',
             '-t', name, '.'], cwd=directory)
        if inspect_image(name)['Architecture'] != 'arm64':
            raise ValueError('image architecture differs')
    qualify('platform', sha)
    # Preserve every private setting; change only immutable image identities.
    path = ROOT / 'platform.env'
    before = path.read_text()
    old_files = inspect('oteryn-oracle-platform-1')['Config']['Labels']['com.docker.compose.project.config_files']
    old_source = Path(old_files.split(',')[0]).parents[2]
    previous = compose(old_source)
    current = compose(directory)
    env = dict(os.environ, OTERYN_ENV_FILE=str(path))
    run(['bash', str(old_source / 'deploy/synology/scripts/backup-postgres.sh'), 'run', 'dump'], env=env)
    run(previous + ['exec', '-T', 'platform', 'php', 'artisan', 'down', '--retry=60'])
    run(previous + ['stop', 'gateway', 'marketplace-scheduler'])
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
    run(['bash', '-euc', script, 'oracle-migrate', str(directory), *current], env=env)
    run(current + ['up', '-d', '--no-deps', '--force-recreate', 'platform', 'gateway', 'marketplace-scheduler'])
    run(current + ['exec', '-T', 'platform', 'php', 'artisan', 'up'])


def inspect_image(image):
    return json.loads(run(['docker', 'image', 'inspect', image], output=True))[0]


def game_deploy(directory, sha):
    image = 'oteryn-game:' + sha + '-arm64'
    run(['docker', 'build', '--build-arg', 'GAME_SOURCE_SHA=' + sha,
         '-f', str(ROOT / 'game.Dockerfile'), '-t', image, str(directory)], timeout=7200)
    info = inspect_image(image)
    if info['Architecture'] != 'arm64' or info['Config']['Labels']['org.opencontainers.image.revision'] != sha:
        raise ValueError('Game image provenance differs')
    qualify('game', sha)
    status('game')
    override = STATE / (sha + '-game-migrate.json')
    atomic(override, json.dumps({'services': {'migrate': {'image': image}}}))
    run(['systemctl', 'start', 'oteryn-game-backup.service'])
    run(['systemctl', 'stop', 'oteryn-game.service'])
    # Existing configuration, content, custody and durable controller stay intact.
    path = ROOT / 'game-runtime.compose.yml'
    before = path.read_text()
    atomic(STATE / (sha + '-game.compose.before'), before)
    changed, count = re.subn(r'(?m)^(    image:) oteryn-game:[^\n]+$', r'\1 ' + image, before)
    if count != 2:
        raise ValueError('expected exactly Game and ops image declarations')
    run(['docker', 'compose', '-f', str(ROOT / 'game-db.compose.yml'), '-f', str(override),
         'run', '--rm', '-T', '--no-deps', 'migrate'])
    atomic(path, changed)
    run(['systemctl', 'start', 'oteryn-game.service'])


def main():
    os.umask(0o077)
    if os.geteuid() != 0:
        raise ValueError('host root required')
    component = sys.argv[1] if len(sys.argv) == 2 else ''
    action, sha = parse(component, os.environ.get('SSH_ORIGINAL_COMMAND', ''))
    STATE.mkdir(mode=0o700, parents=True, exist_ok=True)
    with (STATE / 'deployment.lock').open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        if action == 'deploy':
            receipt = STATE / (sha + '-' + component + '.json')
            if receipt.exists():
                status(component)
                return
            qualify(component, sha)
            directory = source(component, sha)
            # Recheck after a long build/source transfer before accepting effects.
            qualify(component, sha)
            if component == 'platform':
                platform_deploy(directory, sha)
            else:
                game_deploy(directory, sha)
            for _ in range(90):
                try:
                    status(component)
                    break
                except (ValueError, subprocess.SubprocessError):
                    time.sleep(2)
            else:
                raise ValueError('deployment readiness deadline exceeded')
            atomic(STATE / (sha + '-' + component + '.json'), json.dumps({'component': component, 'sha': sha, 'ready': True}))
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
