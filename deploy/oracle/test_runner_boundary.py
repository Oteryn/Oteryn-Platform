"""Exercise the actual workflow shell before its host SSH boundary."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

WORKFLOW = Path(__file__).resolve().parents[2] / '.github/workflows/oracle-operations.yml'


class RunnerBoundary(unittest.TestCase):
    def test_platform_witness_uses_a_durable_writable_mount(self):
        compose = Path(__file__).with_name('public-platform.compose.yml')
        environment = dict(os.environ, PLATFORM_IMAGE='platform:fixture', GATEWAY_IMAGE='gateway:fixture')
        with tempfile.TemporaryDirectory() as temporary:
            fixture = Path(temporary) / 'compose.yml'
            env_file = fixture.with_suffix('.env')
            env_file.touch()
            text = compose.read_text()
            for name in ('platform.env', 'platform-migrate.env'):
                text = text.replace('/opt/oteryn-oracle/' + name, str(env_file))
            fixture.write_text(text)
            result = subprocess.run(
                ['docker', 'compose', '-f', str(fixture), '--profile', 'migration', 'config', '--format', 'json'],
                env=environment, check=True, capture_output=True, text=True,
            )
        services = json.loads(result.stdout)['services']
        platform = services['platform']
        directory = Path(platform['environment']['GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY'])
        mounts = [mount for mount in platform['volumes'] if directory.is_relative_to(mount['target'])]
        self.assertEqual(len(mounts), 1)
        mount, = mounts
        self.assertEqual(mount['type'], 'bind')
        self.assertEqual(mount['source'], '/opt/oteryn-oracle/native-evidence-witness')
        self.assertFalse(mount.get('read_only', False))
        self.assertFalse(mount['bind'].get('create_host_path', False))
        self.assertIn('GAME_AUTH_NATIVE_EVIDENCE_HIGH_WATER_DIRECTORY', compose.with_name('README.md').read_text())
        for service in ('worker', 'migrate', 'gateway'):
            self.assertFalse(any(item['source'] == mount['source'] for item in services[service]['volumes']))

    def test_reachable_alternative_docker_endpoint_stops_before_ssh(self):
        text = WORKFLOW.read_text()
        script = text.split('        run: |\n', 1)[1]
        script = '\n'.join(line[10:] for line in script.splitlines())
        # Identity/socket facts are separate invariants. Start at the endpoint
        # probe so the test host's own Docker socket cannot determine this case.
        script = script[script.index('if command -v docker'):]
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            stubs = {'uname': 'echo aarch64', 'id': 'echo 1001',
                     'docker': 'exit "${PROBE_DOCKER_RESULT:-0}"',
                     'ssh': 'touch "$SSH_BOUNDARY_MARKER"'}
            for name, body in stubs.items():
                path = directory / name
                path.write_text('#!/bin/sh\n' + body + '\n')
                path.chmod(0o755)
            environment = dict(os.environ, PATH=str(directory) + ':' + os.environ['PATH'],
                               ACTUAL_RUNNER_NAME='oteryn-oracle-platform',
                               EXPECTED_RUNNER_NAME='oteryn-oracle-platform',
                               OPERATION='status', SOURCE_SHA='',
                               DOCKER_HOST='tcp://unintended-daemon:2375',
                               SSH_BOUNDARY_MARKER=str(directory / 'ssh-called'))
            rejected = subprocess.run(['bash', '-c', script], env=environment, capture_output=True)
            self.assertNotEqual(rejected.returncode, 0)
            self.assertIn(b'unexpectedly controls', rejected.stderr)
            self.assertFalse((directory / 'ssh-called').exists())
            environment['PROBE_DOCKER_RESULT'] = '1'
            allowed = subprocess.run(['bash', '-c', script], env=environment, capture_output=True)
            self.assertEqual(allowed.returncode, 0, allowed.stderr)
            self.assertTrue((directory / 'ssh-called').exists())


class PrepareGameImage(unittest.TestCase):
    SCRIPT = Path(__file__).with_name('prepare-game.sh')

    def run_prepare(self, image):
        environment = {key: value for key, value in os.environ.items() if key != 'OTERYN_GAME_IMAGE'}
        if image is not None:
            environment['OTERYN_GAME_IMAGE'] = image
        return subprocess.run(['bash', str(self.SCRIPT)], env=environment, capture_output=True, text=True)

    def test_refuses_a_missing_or_malformed_image_before_any_host_effect(self):
        sha = 'a' * 40
        for image in (None, '', 'oteryn-game:77af046ce14c-arm64', 'oteryn-game:' + sha,
                      'oteryn-game:' + sha + '-amd64', 'other:' + sha + '-arm64',
                      'oteryn-game:' + sha.upper() + '-arm64', 'oteryn-game:' + sha + '-arm64 x'):
            with self.subTest(image=image):
                result = self.run_prepare(image)
                self.assertEqual(result.returncode, 2)
                self.assertIn('OTERYN_GAME_IMAGE', result.stderr)

    def test_control_holds_schema_create_only_while_migrate_runs(self):
        lines = self.SCRIPT.read_text().splitlines()
        grant = lines.index("game_sql 'GRANT CREATE ON SCHEMA public TO oteryn_game_control'")
        trap = lines.index('trap \'game_sql "REVOKE CREATE ON SCHEMA public FROM oteryn_game_control"; rm -f "$override"\' EXIT')
        migrate = lines.index('"${compose[@]}" --profile operations run --rm --no-deps migrate')
        revoke = lines.index("game_sql 'REVOKE CREATE ON SCHEMA public FROM oteryn_game_control'")
        self.assertLess(grant, trap)
        self.assertLess(trap, migrate)
        self.assertEqual(revoke, migrate + 1)
        self.assertEqual(lines[revoke + 1], 'trap \'rm -f "$override"\' EXIT')
        self.assertNotIn('CREATE ON SCHEMA', Path(__file__).with_name('prepare-game-db.py').read_text())

    def test_no_shipped_file_pins_a_game_image_for_the_first_install(self):
        self.assertNotIn('docker image inspect oteryn-game:', self.SCRIPT.read_text())
        runtime = Path(__file__).with_name('game-runtime.compose.yml').read_text()
        self.assertEqual(sum(line.startswith('    image: oteryn-game:') for line in runtime.splitlines()), 2)


if __name__ == '__main__':
    unittest.main()
