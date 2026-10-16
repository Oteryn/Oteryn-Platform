#!/usr/bin/env python3
"""Supervise Docker incarnations through shipped durable Game ops commands.

An uncertain registration without its receipt stops for explicit reconciliation.
No spent launch authorization is replayed by Docker or systemd.
"""
import fcntl
import json
import os
from pathlib import Path
import re
import signal
import stat
import subprocess
import sys
import time
import tomllib
import uuid

ROOT = Path('/opt/oteryn-oracle/state/game-controller')
BASE = Path('/opt/oteryn-game')
NODE = BASE / 'node/node.toml'
COMPOSE = ['docker', 'compose', '-f', '/opt/oteryn-oracle/game-runtime.compose.yml']
CONTAINER = 'oteryn-oracle-runtime-game-1'
INSIDE = '/volume1/oteryn/game-preprod'
REPORT = '/volume1/oteryn/game-preprod-root/ops/report.toml'
STOP = False
UUID = r'[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}'
STEP_WAIT_ENV = 'OTERYN_STEP_WAIT_RECEIPTS'


def atomic(path, data, uid=0):
    """Never follow a service-owned destination or temporary symlink."""
    parent = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    name = '.oracle-' + uuid.uuid4().hex
    try:
        fd = os.open(name, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW,
                     0o600, dir_fd=parent)
        with os.fdopen(fd, 'wb') as out:
            os.fchown(out.fileno(), uid, uid)
            out.write(data)
            out.flush()
            os.fsync(out.fileno())
        os.replace(name, path.name, src_dir_fd=parent, dst_dir_fd=parent)
        os.fsync(parent)
    finally:
        os.close(parent)


def invoke(args, *, check=True):
    result = subprocess.run(args, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                            timeout=180, text=True)
    if check and result.returncode:
        raise RuntimeError('command failed: ' + args[0])
    return result


def inspect():
    result = invoke(['docker', 'inspect', CONTAINER], check=False)
    return json.loads(result.stdout)[0] if result.returncode == 0 else None


def save(state):
    atomic(ROOT / 'current.json', json.dumps(state, sort_keys=True).encode())


def ops(state, *args):
    result = invoke(COMPOSE + ['run', '--rm', '-T', '--no-deps', 'ops', *args], check=False)
    # Ops prints fixed diagnostics/receipts, never credential or launch bytes.
    fd = os.open(ROOT / (state['cycle'] + '.log'),
                 os.O_WRONLY | os.O_APPEND | os.O_CREAT | os.O_NOFOLLOW, 0o600)
    with os.fdopen(fd, 'a') as log:
        log.write(result.stdout)
        log.flush()
        os.fsync(log.fileno())
    return result


def fail(reason):
    atomic(ROOT / 'reconciliation-required', (reason + '\n').encode())
    raise RuntimeError(reason)


def copy_certificate():
    directory = Path('/var/lib/docker/volumes/oteryn-oracle-edge_caddy_data/_data/caddy/certificates')
    certificates = list(directory.glob('*/game.oteryn.com/game.oteryn.com.crt'))
    if len(certificates) != 1:
        raise RuntimeError('expected one managed game.oteryn.com certificate')
    cert = certificates[0]
    invoke(['openssl', 'x509', '-in', str(cert), '-noout', '-checkhost', 'game.oteryn.com'])
    invoke(['openssl', 'x509', '-in', str(cert), '-noout', '-checkend', '0'])
    changed = False
    for source, target in [(cert, BASE / 'node/secrets/gameplay.crt'),
                           (cert.with_suffix('.key'), BASE / 'node/secrets/gameplay.key')]:
        if source.is_symlink() or target.is_symlink():
            raise RuntimeError('certificate symlink refused')
        data = source.read_bytes()
        if not target.exists() or target.read_bytes() != data:
            atomic(target, data, 1001)
            changed = True
    return changed


def step_wait_receipts():
    """Owner marker BASE/run/step-wait-receipts: on only for a regular file holding 1.

    Anything else (absent, symlink, directory, other bytes) keeps Game's default off.
    """
    try:
        fd = os.open(BASE / 'run' / 'step-wait-receipts', os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    except OSError:
        return '0'
    try:
        if not stat.S_ISREG(os.fstat(fd).st_mode):
            return '0'
        data = os.read(fd, 16)
    finally:
        os.close(fd)
    return '1' if data.replace(b'\n', b'') == b'1' else '0'


def stop_game(*_):
    global STOP
    STOP = True
    subprocess.run(['docker', 'stop', '-t', '60', CONTAINER],
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=75)


def wait_registration(state):
    for _ in range(180):
        if STOP:
            return False
        current = inspect()
        if current is None or current['Config']['Labels'].get('io.oteryn.launch-cycle') != state['cycle']:
            fail('launch container identity differs; reconcile the retained cycle')
        logs = invoke(['docker', 'logs', '--tail', '500', CONTAINER], check=False).stdout
        match = re.search(r'event=registered .*?node_id=(' + UUID + r') registration_revision=([1-9][0-9]*)', logs)
        if match:
            state.update(node_id=match[1], revision=match[2], phase='assigning')
            save(state)
            return True
        if not current['State']['Running']:
            fail('node exited without a registered receipt; reconcile the registration gap')
        time.sleep(1)
    fail('registered receipt deadline exceeded; reconcile before another launch')


def start(state, baseline):
    os.environ['OTERYN_GAME_CYCLE'] = state['cycle']
    if state['phase'] == 'authorizing':
        launch = BASE / 'state' / state['launch']
        if launch.exists():
            result = ops(state, 'authorization', 'reconcile', '--file', state['launch'])
        else:
            result = ops(state, 'authorization', 'issue', '--file', state['launch'],
                         '--binding', 'oracle-' + state['cycle'], '--supersedes', state['previous'])
            if result.returncode and launch.exists():
                result = ops(state, 'authorization', 'reconcile', '--file', state['launch'])
        if result.returncode:
            fail('launch authorization unresolved; retain and reconcile its exact file')
        fd = os.open(NODE, os.O_RDONLY | os.O_NOFOLLOW)
        with os.fdopen(fd) as source:
            document = source.read()
        document, count = re.subn(r'(?m)^authorization_file = ".*"$',
                                 'authorization_file = "' + INSIDE + '/state/' + state['launch'] + '"', document)
        if count != 1:
            fail('node must name exactly one authorization file')
        atomic(NODE, document.encode(), 1001)
        state['phase'] = 'starting'
        save(state)
    if state['phase'] == 'starting':
        current = inspect()
        if current is None or current['Config']['Labels'].get('io.oteryn.launch-cycle') != state['cycle']:
            # Game reads the switch once per process, so the marker applies per incarnation.
            os.environ[STEP_WAIT_ENV] = step_wait_receipts()
            print('oracle_game_step_wait_receipts=' + os.environ[STEP_WAIT_ENV], flush=True)
            invoke(COMPOSE + ['up', '-d', '--no-deps', '--force-recreate', 'game'])
        if not wait_registration(state):
            return
    if state['phase'] == 'assigning':
        request = 'assign-oracle-' + state['cycle'] + '.json'
        common = ['--report-config', REPORT, '--node-config', INSIDE + '/node/node.toml']
        if (BASE / 'state' / request).exists():
            result = ops(state, 'assignment', 'reconcile', *common, '--request', request)
        else:
            result = ops(state, 'assignment', 'replace', *common, '--request', request,
                         '--node-identity', baseline['node_identity'], '--world', baseline['world_id'],
                         '--channel', baseline['channel_id'], '--node-id', state['node_id'],
                         '--revision', state['revision'])
        if result.returncode and (BASE / 'state' / request).exists():
            result = ops(state, 'assignment', 'reconcile', *common, '--request', request)
        if result.returncode or 'assignment=absent' in result.stdout:
            fail('assignment/report unresolved; reconcile its exact retained request')
        for _ in range(180):
            if STOP:
                return
            current = inspect()
            logs = invoke(['docker', 'logs', '--tail', '500', CONTAINER], check=False).stdout
            if current and current['State']['Running'] and re.search(r'readiness.*ready=true', logs):
                state['phase'] = 'ready'
                save(state)
                print('oracle_game_ready node_id=' + state['node_id'], flush=True)
                return
            if not current or not current['State']['Running']:
                fail('assigned node stopped before readiness; retain receipts for reconciliation')
            time.sleep(1)
        fail('assigned node readiness deadline exceeded')


def main():
    if os.geteuid() != 0:
        raise RuntimeError('root required')
    os.umask(0o077)
    ROOT.mkdir(mode=0o700, parents=True, exist_ok=True)
    if len(sys.argv) == 2 and sys.argv[1] == '--refresh-certificate':
        if copy_certificate():
            invoke(['systemctl', 'try-restart', 'oteryn-game.service'])
        return
    lock = os.open(ROOT / 'lock', os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW, 0o600)
    fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    if (ROOT / 'reconciliation-required').exists():
        raise RuntimeError('retained operation requires explicit reconciliation')
    baseline = json.loads((ROOT / 'migration-baseline.json').read_text())
    node = tomllib.loads(NODE.read_text())
    if any(not re.fullmatch(UUID, baseline[k]) for k in ['previous', 'world_id', 'channel_id']):
        raise RuntimeError('invalid migration identity')
    if node['scope']['world_id'] != baseline['world_id'] or node['scope']['channel_id'] != baseline['channel_id']:
        raise RuntimeError('node differs from root-owned migration scope')
    for _ in range(120):
        db = invoke(['docker', 'inspect', 'oteryn-oracle-game-postgres-1'], check=False)
        if db.returncode == 0 and json.loads(db.stdout)[0]['State'].get('Health', {}).get('Status') == 'healthy':
            break
        time.sleep(1)
    else:
        raise RuntimeError('Game database startup deadline exceeded')
    copy_certificate()
    signal.signal(signal.SIGTERM, stop_game)
    signal.signal(signal.SIGINT, stop_game)
    state = json.loads((ROOT / 'current.json').read_text()) if (ROOT / 'current.json').exists() else None
    while not STOP:
        current = inspect()
        if state and state['phase'] == 'ready' and current and current['State']['Running']:
            if current['Config']['Labels'].get('io.oteryn.launch-cycle') != state['cycle']:
                fail('running container differs from the retained ready incarnation')
        if state is None or (state['phase'] == 'ready' and (not current or not current['State']['Running'])):
            previous = state['node_id'] if state else baseline['previous']
            cycle = uuid.uuid4().hex
            state = dict(cycle=cycle, previous=previous, launch='launch-oracle-' + cycle + '.json', phase='authorizing')
            save(state)
        if state['phase'] != 'ready':
            start(state, baseline)
        time.sleep(3)


if __name__ == '__main__':
    try:
        main()
    except Exception as error:
        print('oracle_game_supervisor_stopped: ' + str(error), file=sys.stderr, flush=True)
        # A stopped controller must never leave an unqualified serving process.
        if '--refresh-certificate' not in sys.argv:
            subprocess.run(['docker', 'stop', '-t', '60', CONTAINER],
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=75)
        sys.exit(78)
