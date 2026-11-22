#!/usr/bin/env python3
"""Root-owned, component-bound SSH forced command; never a general shell.

Install manually after review. A runner may observe its component or deploy the
current protected main after its aggregate gate passes. Platform builds its image
on the host from the qualified source; Game never builds here: its runner streams
the arm64 image artifact of the same qualified Game run on stdin. Host
secrets/configuration, World content, custody and the incarnation controller
remain separately installed.
"""
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tarfile
import tempfile
import time
import traceback
import urllib.request
import zipfile

ROOT = Path('/opt/oteryn-oracle')
STATE = ROOT / 'state/actions'
BUILD = Path('/opt/oteryn-build')
REPOS = {'platform': 'Oteryn-Platform', 'game': 'Oteryn-Game'}
SHA = r'[0-9a-f]{40}'
# The Game runtime image (Debian slim plus three release binaries) is a few
# hundred MiB as `docker save` output. 1 GiB of zip stream and 3 GiB unpacked
# leave several times that headroom while bounding what one transfer may write
# to the host disk before its digest is known.
IMAGE_ZIP_LIMIT = 1 << 30
IMAGE_TAR_LIMIT = 3 << 30
CHUNK = 1 << 20


def parse(component, command):
    if component not in REPOS:
        raise ValueError('unknown component')
    if command == 'status':
        return 'status', None, None, None, None
    if component == 'game':
        # Game is never built on this host: only a qualified CI image artifact.
        match = re.fullmatch(r'deploy-image (' + SHA + r') ([1-9][0-9]{0,19}) ([1-9][0-9]{0,5}) ([1-9][0-9]{0,19})', command)
        if not match:
            raise ValueError('only status or deploy-image <full protected-main SHA> <run> <attempt> <artifact> is allowed')
        return 'deploy', match[1], int(match[2]), int(match[3]), int(match[4])
    match = re.fullmatch(r'deploy (' + SHA + r') ([1-9][0-9]{0,19}) ([1-9][0-9]{0,5})', command)
    if not match:
        raise ValueError('only status or deploy <full protected-main SHA> is allowed')
    return 'deploy', match[1], int(match[2]), int(match[3]), None


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
    durable_replace(temporary, path)


def durable_replace(source, path):
    os.replace(source, path)
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
    migrations = sorted((directory / 'portal-rs/migrations').glob('*.sql'))
    if not migrations or not (directory / 'deploy/oracle/portal.Dockerfile').is_file():
        raise ValueError('Rust build inputs or migrations missing')
    schema = 'portal-rs-' + migrations[-1].name.split('_', 1)[0]
    current = STATE / 'platform-current.json'
    if current.exists():
        previous = json.loads(current.read_text())['schema']
        accepted = {'portal-rs-' + path.name.split('_', 1)[0] for path in migrations}
        if previous not in accepted:
            raise ValueError('retained schema is not a Rust portal predecessor; operator reconciliation required')
    return schema


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
        platform_status(allow_pending)
        names = []
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
              {PORTAL_CONTAINERS[name]: record['platform_image'] for name in PORTAL_APPS})
    if any(inspect(name)['Image'] != identity for name, identity in images.items()):
        raise ValueError('receipt differs from current runtime images')


PORTAL = Path('/opt/oteryn-portal')
PORTAL_PROJECT = 'oteryn-portal'
PORTAL_NETWORK = 'oteryn-oracle_private'
PORTAL_CONTAINERS = {'portal': 'oteryn-portal-portal-1', 'portal-worker': 'oteryn-portal-portal-worker-1',
                     'portal-gateway': 'oteryn-portal-portal-gateway-1', 'portal-pg': 'oteryn-portal-portal-pg-1'}
PORTAL_APPS = ('portal', 'portal-worker', 'portal-gateway')
REVISION = 'org.opencontainers.image.revision'


def portal_compose():
    # The live, host-retained composition; never the manual-deploy checkout under src/.
    return ['docker', 'compose', '-p', PORTAL_PROJECT, '-f', str(PORTAL / 'compose.yml')]


def portal_image(sha):
    return 'oteryn-portal:' + sha[:8] + '-arm64'


def portal_texts(sha):
    """Return (path, before, after) for every live file the release rewrites.

    Validated before any effect, so an unexpected layout leaves no pending marker.
    """
    compose_path = PORTAL / 'compose.yml'
    before = compose_path.read_text()
    after, count = re.subn(r'(?m)^([ \t]+image:[ \t]+)oteryn-portal:[A-Za-z0-9_.-]+[ \t]*$',
                           lambda match: match[1] + portal_image(sha), before)
    if count != 2:
        raise ValueError('expected exactly the portal anchor and gateway image declarations')
    texts = [(compose_path, before, after)]
    gateway_path = PORTAL / 'gateway.env'
    gateway = gateway_path.read_text()
    after, count = re.subn(r'(?m)^GATEWAY_VERSION=.*$', 'GATEWAY_VERSION=portal-rs-' + sha[:8], gateway)
    if count > 1:
        raise ValueError('expected at most one gateway version setting')
    if count:
        texts.append((gateway_path, gateway, after))
    return texts


def rewrite(path, data):
    # Keep the owner and mode of the operator-managed file so the manual
    # deployment path keeps working; atomic() would leave it root-only.
    info = path.stat()
    temporary = path.with_name(path.name + '.actions-next')
    # A temporary left by an interrupted attempt must not block the restore.
    temporary.unlink(missing_ok=True)
    fd = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    with os.fdopen(fd, 'w') as stream:
        stream.write(data)
        stream.flush()
        os.fchown(stream.fileno(), info.st_uid, info.st_gid)
        os.fchmod(stream.fileno(), info.st_mode & 0o7777)
        os.fsync(stream.fileno())
    durable_replace(temporary, path)


def backup_platform(sha):
    # The dump runs inside the live database container with its own local
    # credentials; no URL or password reaches Docker's host-side arguments.
    database = PORTAL_CONTAINERS['portal-pg']
    path = STATE / (sha + '-platform.dump')
    with tempfile.TemporaryDirectory(prefix=path.name + '-', dir=STATE) as attempt:
        staged = Path(attempt) / 'archive'
        fd = os.open(staged, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        with os.fdopen(fd, 'wb') as archive:
            subprocess.run(['docker', 'exec', database, 'sh', '-c',
                            'pg_dump -U "$POSTGRES_USER" -d "${POSTGRES_DB:-$POSTGRES_USER}" -Fc'],
                           check=True, stdout=archive, stderr=subprocess.DEVNULL, timeout=1800)
            archive.flush()
            os.fsync(archive.fileno())
        with staged.open('rb') as archive:
            subprocess.run(['docker', 'exec', '-i', database, 'pg_restore', '--list'],
                           check=True, stdin=archive, stdout=subprocess.DEVNULL,
                           stderr=subprocess.DEVNULL, timeout=120)
        durable_replace(staged, path)


def portal_ready():
    # The portal is published only on the private network and its image has no
    # HTTP client. Host root reaches the bridge address directly; /readyz
    # includes the database check. No edge, DNS or TLS dependency.
    networks = inspect(PORTAL_CONTAINERS['portal'])['NetworkSettings']['Networks']
    address = networks.get(PORTAL_NETWORK, {}).get('IPAddress')
    if not address:
        raise ValueError('portal has no address on the private network')
    try:
        with urllib.request.urlopen('http://' + address + ':8080/readyz', timeout=10) as response:
            if response.status != 200:
                raise ValueError('portal readiness differs')
    except OSError as error:
        raise ValueError('portal readiness probe failed') from error


def portal_containers():
    images = set()
    for service, name in PORTAL_CONTAINERS.items():
        info = inspect(name)
        state = info['State']
        if (not state['Running'] or state.get('Restarting')
                or state.get('Health', {}).get('Status', 'healthy') != 'healthy'):
            raise ValueError('component is not healthy: ' + name)
        if service in PORTAL_APPS:
            images.add(info['Image'])
    return images


def platform_status(allow_pending):
    pending = STATE / 'platform-pending.json'
    record = pending if allow_pending and pending.exists() else STATE / 'platform-current.json'
    if not record.exists():
        raise ValueError('no runner-deployed portal release is recorded')
    sha = json.loads(record.read_text())['sha']
    images = portal_containers()
    if len(images) != 1:
        raise ValueError('portal roles run different images')
    labels = inspect_image(images.pop())['Config'].get('Labels') or {}
    if labels.get(REVISION) != sha:
        raise ValueError('running portal differs from the recorded release')
    portal_ready()


def wait_platform(image_id):
    # Ready means: the new image in every role and healthy on two consecutive
    # samples with no restart in between.
    previous = None
    for _ in range(90):
        try:
            platform_status(True)
            roles = [inspect(PORTAL_CONTAINERS[name]) for name in PORTAL_APPS]
            if any(info['Image'] != image_id for info in roles):
                raise ValueError('running image differs from qualified build')
            restarts = [info['RestartCount'] for info in roles]
            if restarts == previous:
                return
            previous = restarts
        except (ValueError, subprocess.SubprocessError):
            previous = None
        time.sleep(2)
    raise ValueError('deployment readiness deadline exceeded')


def platform_deploy(directory, sha, run_id, attempt):
    image = portal_image(sha)
    schema = contract(directory)
    run(['docker', 'build', '--label', REVISION + '=' + sha,
         '-f', 'deploy/oracle/portal.Dockerfile', '-t', image, 'portal-rs'], cwd=directory, timeout=7200)
    info = inspect_image(image)
    if info['Architecture'] != 'arm64' or info['Config']['Labels'].get(REVISION) != sha:
        raise ValueError('portal image provenance differs')
    run(['docker', 'run', '--rm', '--network', 'none', image, 'check-image'], timeout=120)
    texts = portal_texts(sha)
    # Prove the readiness probe works against the running release before any
    # effect, so a host that cannot be probed never rolls back a good release.
    portal_containers()
    portal_ready()
    reauthorize('platform', sha, run_id, attempt)
    backup_platform(sha)
    reauthorize('platform', sha, run_id, attempt)
    journal('platform', sha, 'prepared', schema=schema,
            previous_image=inspect(PORTAL_CONTAINERS['portal'])['Image'], platform_image=info['Id'])
    for path, before, _ in texts:
        atomic(STATE / (sha + '-portal.' + path.name.split('.')[0] + '.before'), before)
    current = portal_compose()
    try:
        for path, _, after in texts:
            rewrite(path, after)
        reauthorize('platform', sha, run_id, attempt)
        journal('platform', sha, 'schema_unknown')
        run(current + ['run', '--rm', '-T', 'portal', 'migrate'])
        journal('platform', sha, 'schema_known')
        run(current + ['up', '-d', *PORTAL_APPS])
        wait_platform(info['Id'])
    except BaseException:
        # Return the previous image to service. The pending marker stays, so
        # status reports reconciliation; the database is never dropped or restored.
        for path, before, _ in texts:
            rewrite(path, before)
        run(current + ['up', '-d', *PORTAL_APPS])
        raise
    return {PORTAL_CONTAINERS[name]: info['Id'] for name in PORTAL_APPS}


def inspect_image(image):
    return json.loads(run(['docker', 'image', 'inspect', image], output=True))[0]


def exclusive(path):
    return os.fdopen(os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600), 'wb')


def receive_artifact(sha, run_id, artifact_id, stream, path):
    """Store the streamed artifact zip and prove it is the qualified run's upload."""
    digest, size = hashlib.sha256(), 0
    with exclusive(path) as out:
        while chunk := stream.read(CHUNK):
            size += len(chunk)
            if size > IMAGE_ZIP_LIMIT:
                raise ValueError('Game image artifact exceeds the size limit')
            digest.update(chunk)
            out.write(chunk)
        out.flush()
        os.fsync(out.fileno())
    artifact = api(REPOS['game'] + '/actions/artifacts/' + str(artifact_id))
    workflow = artifact.get('workflow_run') or {}
    expected = {'id': artifact_id, 'name': 'oteryn-game-image-' + sha, 'expired': False,
                'digest': 'sha256:' + digest.hexdigest()}
    if any(artifact.get(key) != value for key, value in expected.items()) \
            or workflow.get('id') != run_id or workflow.get('head_sha') != sha:
        raise ValueError('Game image artifact differs from the qualified run upload')


def unpack_image(archive_path, path, image):
    """Extract the single image.tar and refuse any other tag in it."""
    with zipfile.ZipFile(archive_path) as archive:
        members = archive.infolist()
        if [member.filename for member in members] != ['image.tar'] or members[0].file_size > IMAGE_TAR_LIMIT:
            raise ValueError('Game image artifact must hold only image.tar')
        copied = 0
        with archive.open(members[0]) as source, exclusive(path) as out:
            while chunk := source.read(CHUNK):
                copied += len(chunk)
                if copied > IMAGE_TAR_LIMIT:
                    raise ValueError('Game image artifact exceeds the size limit')
                out.write(chunk)
            out.flush()
            os.fsync(out.fileno())
    # `docker load` applies every tag named in the archive; one foreign tag
    # could replace another local image, such as the Platform's.
    with tarfile.open(path) as layout:
        manifest = layout.extractfile('manifest.json')
        entries = json.load(manifest) if manifest else None
    if not isinstance(entries, list) or len(entries) != 1 or entries[0].get('RepoTags') != [image]:
        raise ValueError('Game image archive must carry exactly its qualified tag')


def load_image(sha, run_id, artifact_id, stream):
    image = 'oteryn-game:' + sha + '-arm64'
    archive, tarball = STATE / (sha + '-game-image.zip'), STATE / (sha + '-game-image.tar')
    # The deployment lock is held: a leftover is an aborted transfer of this host.
    for path in (archive, tarball):
        path.unlink(missing_ok=True)
    try:
        receive_artifact(sha, run_id, artifact_id, stream, archive)
        unpack_image(archive, tarball, image)
        run(['docker', 'load', '--input', str(tarball)])
    finally:
        for path in (archive, tarball):
            path.unlink(missing_ok=True)
    info = inspect_image(image)
    if image not in (info.get('RepoTags') or []) or info['Architecture'] != 'arm64' \
            or (info['Config'].get('Labels') or {}).get(REVISION) != sha:
        raise ValueError('Game image provenance differs')
    return image, info


def game_schema_create(statement):
    run(['docker', 'compose', '-f', str(ROOT / 'game-db.compose.yml'), 'exec', '-T', 'postgres',
         'gosu', 'postgres', 'psql', '-XqAt', '-v', 'ON_ERROR_STOP=1', '-d', 'oteryn_game', '-c',
         statement + ' CREATE ON SCHEMA public ' + ('TO' if statement == 'GRANT' else 'FROM')
         + ' oteryn_game_control'])


def game_migrate(override):
    # Game migrations hand Control-issued objects to oteryn_game_control
    # (ALTER ... OWNER TO), which needs Control's CREATE on the schema. Control
    # holds it only while migrate runs; ownership already transferred stays.
    game_schema_create('GRANT')
    try:
        run(['docker', 'compose', '-f', str(ROOT / 'game-db.compose.yml'), '-f', str(override),
             'run', '--rm', '-T', '--no-deps', 'migrate'])
    finally:
        game_schema_create('REVOKE')


def game_deploy(sha, run_id, attempt, artifact_id, stream):
    image, info = load_image(sha, run_id, artifact_id, stream)
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
    game_migrate(override)
    journal('game', sha, 'schema_known')
    atomic(path, changed)
    run(['systemctl', 'start', 'oteryn-game.service'])
    return {'oteryn-oracle-runtime-game-1': info['Id']}


def main():
    os.umask(0o077)
    if os.geteuid() != 0:
        raise ValueError('host root required')
    component = sys.argv[1] if len(sys.argv) == 2 else ''
    action, sha, run_id, attempt, artifact = parse(component, os.environ.get('SSH_ORIGINAL_COMMAND', ''))
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
            if component == 'platform':
                directory = source(component, sha)
                # Recheck after a long build/source transfer before accepting effects.
                qualify(component, sha)
                images = platform_deploy(directory, sha, run_id, attempt)
            else:
                # game_deploy re-qualifies after the transfer, before any effect.
                images = game_deploy(sha, run_id, attempt, artifact, sys.stdin.buffer)
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
