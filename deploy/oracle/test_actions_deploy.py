import hashlib
import importlib.util
import inspect
from pathlib import Path
import unittest
import json
import tempfile
import subprocess
import os
import stat
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('oracle_deploy', Path(__file__).with_name('actions-deploy.py'))
deploy = importlib.util.module_from_spec(spec)
spec.loader.exec_module(deploy)


class SourceArtifactPermissions(unittest.TestCase):
    def test_private_operator_umask_does_not_make_public_build_inputs_unreadable(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            origin = root / 'origin'
            origin.mkdir()
            def git(*args):
                return subprocess.check_output(['git', '-c', 'core.hooksPath=/dev/null', *args], cwd=origin, stderr=subprocess.DEVNULL, text=True).strip()
            git('init', '-b', 'main')
            (origin / 'artisan').write_text('<?php echo "application";')
            (origin / 'artisan').chmod(0o755)
            (origin / 'app.php').write_text('<?php echo "source";')
            git('add', 'artisan', 'app.php')
            git('-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid', 'commit', '-m', 'public fixture')
            sha = git('rev-parse', 'HEAD')
            original_run = deploy.run
            def local_transport(args, **kwargs):
                # Only replace the public transport with an isolated Git fixture;
                # exercise the real clone, fetch, checkout and process umask.
                args = [origin.as_uri() if arg == 'https://github.com/Oteryn/Oteryn-Platform.git' else arg for arg in args]
                return original_run(args, **kwargs)
            previous = os.umask(0o077)
            try:
                with patch.object(deploy, 'BUILD', root / 'build'), patch.object(deploy, 'run', side_effect=local_transport):
                    checkout = deploy.source('platform', sha)
                self.assertEqual(stat.S_IMODE((checkout / 'artisan').stat().st_mode), 0o755)
                self.assertEqual(stat.S_IMODE((checkout / 'app.php').stat().st_mode), 0o644)
                self.assertEqual(stat.S_IMODE(checkout.stat().st_mode), 0o755)
                private = root / 'private-after'
                private.write_text('operator state')
                self.assertEqual(stat.S_IMODE(private.stat().st_mode), 0o600)
            finally:
                os.umask(previous)


class AuthorityBoundary(unittest.TestCase):
    def test_receipt_cannot_accept_a_different_healthy_image(self):
        sha = 'a' * 40
        with tempfile.TemporaryDirectory() as temporary:
            state = Path(temporary)
            (state / 'game-current.json').write_text(json.dumps({'sha': sha, 'phase': 'ready', 'image': 'expected'}))
            with patch.object(deploy, 'STATE', state), patch.object(deploy, 'inspect', return_value={'Image': 'other'}):
                with self.assertRaisesRegex(ValueError, 'runtime images'):
                    deploy.verify_current_release('game', sha)

    def test_game_refuses_migration_while_old_container_is_running(self):
        sha = 'a' * 40
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            controller = root / 'state/game-controller'
            controller.mkdir(parents=True)
            (controller / 'current.json').write_text('{"cycle":"old-cycle"}')
            calls = []
            image = {'Architecture': 'arm64', 'Id': 'new-id', 'Config': {'Labels': {'org.opencontainers.image.revision': sha}}}
            with patch.object(deploy, 'ROOT', root), patch.object(deploy, 'STATE', root), patch.object(deploy, 'inspect_image', return_value=image), patch.object(deploy, 'inspect', return_value={'State': {'Running': True}}), patch.object(deploy, 'status'), patch.object(deploy, 'atomic'), patch.object(deploy, 'journal'), patch.object(deploy, 'run', side_effect=lambda args, **kw: calls.append(args)), patch.object(deploy, 'reauthorize'):
                with self.assertRaisesRegex(ValueError, 'still running'):
                    deploy.game_deploy(root, sha, 100, 1)
            self.assertFalse(any('migrate' in args for args in calls))

    def test_component_cannot_select_another_component_or_general_shell(self):
        for component, command in [('other', 'status'), ('game', 'bash'),
                                   ('game', 'status; id'), ('game', 'deploy ' + 'a' * 40 + ' extra'),
                                   ('platform', 'deploy ../../main'), ('game', 'deploy ' + 'A' * 40)]:
            with self.subTest(component=component, command=command), self.assertRaises(ValueError):
                deploy.parse(component, command)

    def test_exact_current_main_and_aggregate_success_are_independent(self):
        sha = 'a' * 40
        good = {'id': 10, 'name': 'game-gate', 'head_sha': sha,
                'app': {'slug': 'github-actions'}, 'conclusion': 'success'}
        # Change exactly one authority fact per negative case.
        cases = [({'protected': False, 'commit': {'sha': sha}}, [good]), ({'protected': True, 'commit': {'sha': 'b' * 40}}, [good]),
                 ({'protected': True, 'commit': {'sha': sha}}, []),
                 ({'protected': True, 'commit': {'sha': sha}}, [dict(good, head_sha='b' * 40)]),
                 ({'protected': True, 'commit': {'sha': sha}}, [dict(good, app={'slug': 'other'})]),
                 ({'protected': True, 'commit': {'sha': sha}}, [dict(good, conclusion='failure')]),
                 ({'protected': True, 'commit': {'sha': sha}}, [good, dict(good, id=11, conclusion=None)])]
        for main, checks in cases:
            with self.subTest(main=main, checks=checks), patch.object(deploy, 'api', side_effect=[main, {'check_runs': checks}]), self.assertRaises(ValueError):
                deploy.qualify('game', sha)
        with patch.object(deploy, 'api', side_effect=[{'protected': True, 'commit': {'sha': sha}}, {'check_runs': [good]}]):
            deploy.qualify('game', sha)

    def test_running_container_must_match_retained_incarnation(self):
        with patch.object(deploy, 'ROOT') as root, patch.object(deploy, 'run'), patch.object(deploy, 'inspect') as inspect:
            root.__truediv__.return_value.read_text.return_value = '{"phase":"ready","cycle":"expected"}'
            root.__truediv__.return_value.exists.return_value = False
            inspect.return_value = {'State': {'Running': True}, 'Config': {'Labels': {'io.oteryn.launch-cycle': 'other'}}}
            with self.assertRaisesRegex(ValueError, 'incarnation'):
                deploy.status('game')

    def test_key_cannot_bypass_workflow_or_environment_release(self):
        sha = 'a' * 40
        workflow = {'head_sha': sha, 'head_branch': 'main', 'event': 'workflow_dispatch',
                    'path': '.github/workflows/oracle-operations.yml', 'status': 'in_progress',
                    'run_attempt': 1, 'display_title': 'Oracle deploy ' + sha,
                    'repository': {'full_name': 'Oteryn/Oteryn-Game'},
                    'head_repository': {'full_name': 'Oteryn/Oteryn-Game'}}
        job = {'name': 'operate', 'status': 'in_progress', 'runner_name': 'oteryn-oracle-game'}
        for field, value in [('head_sha', 'b' * 40), ('head_branch', 'other'),
                             ('event', 'pull_request'), ('path', '.github/workflows/other.yml'),
                             ('status', 'completed'), ('run_attempt', 2),
                             ('display_title', 'Oracle status '),
                             ('head_repository', {'full_name': 'other/repo'})]:
            with self.subTest(field=field), patch.object(deploy, 'api', return_value=dict(workflow, **{field: value})), self.assertRaises(ValueError):
                deploy.qualify_workflow('game', sha, 100, 1)
        for changed_job in [dict(job, status='waiting'), dict(job, runner_name='other')]:
            with patch.object(deploy, 'api', side_effect=[workflow, {'jobs': [changed_job]}]), self.assertRaises(ValueError):
                deploy.qualify_workflow('game', sha, 100, 1)
        with patch.object(deploy, 'api', side_effect=[workflow, {'jobs': [job]}]):
            deploy.qualify_workflow('game', sha, 100, 1)


class RustDeployment(unittest.TestCase):
    def test_edge_passes_its_verified_client_as_one_trusted_proxy_hop(self):
        edge = (Path(__file__).with_name('Caddyfile')).read_text()
        self.assertIn('trusted_proxies_strict', edge)
        self.assertIn('header_up X-Forwarded-For {client_ip}', edge)
        self.assertIn('TRUSTED_PROXIES: 172.18.0.10', Path(__file__).with_name('public-platform.compose.yml').read_text())

    def test_gateway_composition_reaches_the_rust_loopback_issuer(self):
        checkout = Path(__file__).resolve().parents[2]
        config = (checkout / 'deploy/oracle/public-platform.compose.yml').read_text()
        edge = (checkout / 'deploy/oracle/Caddyfile').read_text()
        self.assertIn('network_mode: service:platform', config)
        self.assertIn('PORTAL_NATIVE_ADMISSION_BIND: 127.0.0.1:8091', config)
        self.assertIn('OTERYN_PLATFORM_BASE_URL: http://127.0.0.1:8091', config)
        self.assertIn('GATEWAY_LISTEN_ADDR: 0.0.0.0:8081', config)
        self.assertIn('reverse_proxy platform:8081', edge)
        self.assertIn('rewrite @native_ticket /api/v1/game-auth/tickets', edge)


LIVE_COMPOSE = '''name: oteryn-portal
x-portal: &portal
  image: oteryn-portal:0123abcd-arm64
  env_file: [portal.env]
services:
  portal-pg:
    image: postgres:18.6-bookworm
  portal:
    <<: *portal
    command: serve
  portal-worker:
    <<: *portal
    command: worker
  portal-gateway:
    image: oteryn-portal:0123abcd-arm64
    network_mode: service:portal
    command: gateway
networks:
  oteryn-oracle_private:
    external: true
'''
LIVE_GATEWAY = 'GATEWAY_LISTEN_ADDR=0.0.0.0:8081\nGATEWAY_VERSION=portal-rs-0123abcd\n'


class LivePortalDeployment(unittest.TestCase):
    sha = 'fedcba98' + 'a' * 32

    def host(self, temporary, gateway=LIVE_GATEWAY):
        root = Path(temporary)
        portal = root / 'opt-oteryn-portal'
        portal.mkdir()
        (portal / 'compose.yml').write_text(LIVE_COMPOSE)
        (portal / 'compose.yml').chmod(0o640)
        (portal / 'gateway.env').write_text(gateway)
        (portal / 'gateway.env').chmod(0o600)
        state = root / 'state'
        state.mkdir()
        return portal, state

    def deploy(self, portal, state, *, fail=None, reauthorize=None, ready=None, probe=None):
        calls = []
        compose = ['docker', 'compose', '-p', 'oteryn-portal', '-f', str(portal / 'compose.yml')]
        def command(args, **kwargs):
            calls.append((args, kwargs))
            if fail and args == compose + fail:
                raise subprocess.CalledProcessError(1, args)
        image = {'Architecture': 'arm64', 'Id': 'new-id',
                 'Config': {'Labels': {'org.opencontainers.image.revision': self.sha}}}
        with patch.object(deploy, 'PORTAL', portal), patch.object(deploy, 'STATE', state), \
             patch.object(deploy, 'run', side_effect=command), \
             patch.object(deploy, 'inspect', return_value={'Image': 'old-id', 'State': {'Running': True}}), \
             patch.object(deploy, 'inspect_image', return_value=image), \
             patch.object(deploy, 'portal_ready', side_effect=probe or (lambda: calls.append((['probe'], {})))), \
             patch.object(deploy, 'contract', return_value='portal-rs-1'), \
             patch.object(deploy, 'backup_platform', side_effect=lambda sha: calls.append((['backup'], {}))), \
             patch.object(deploy, 'reauthorize', side_effect=reauthorize), \
             patch.object(deploy, 'wait_platform', side_effect=ready or (lambda image_id: calls.append((['ready', image_id], {})))):
            try:
                return deploy.platform_deploy(Path('/archive'), self.sha, 100, 1), calls, compose
            except BaseException as error:
                error.calls = calls
                raise

    def test_release_rewrites_both_image_lines_then_migrates_and_starts_the_roles(self):
        with tempfile.TemporaryDirectory() as temporary:
            portal, state = self.host(temporary)
            images, calls, compose = self.deploy(portal, state)
            argv = [args for args, _ in calls]
            build, build_kwargs = calls[0]
            self.assertEqual(build, ['docker', 'build', '--label', 'org.opencontainers.image.revision=' + self.sha,
                                     '-f', 'deploy/oracle/portal.Dockerfile', '-t', 'oteryn-portal:fedcba98-arm64', 'portal-rs'])
            self.assertEqual(build_kwargs, {'cwd': Path('/archive'), 'timeout': 7200})
            self.assertEqual(argv[1], ['docker', 'run', '--rm', '--network', 'none', 'oteryn-portal:fedcba98-arm64', 'check-image'])
            migrate = argv.index(compose + ['run', '--rm', '-T', 'portal', 'migrate'])
            up = argv.index(compose + ['up', '-d', 'portal', 'portal-worker', 'portal-gateway'])
            self.assertLess(argv.index(['probe']), argv.index(['backup']))
            self.assertLess(argv.index(['backup']), migrate)
            self.assertLess(migrate, up)
            self.assertLess(up, argv.index(['ready', 'new-id']))
            self.assertEqual(len([a for a in argv if 'up' in a]), 1)
            self.assertNotIn(str(portal / 'src'), ' '.join(' '.join(map(str, a)) for a in argv))
            compose_text = (portal / 'compose.yml').read_text()
            self.assertEqual(compose_text, LIVE_COMPOSE.replace('oteryn-portal:0123abcd-arm64', 'oteryn-portal:fedcba98-arm64'))
            self.assertIn('image: postgres:18.6-bookworm', compose_text)
            self.assertEqual((portal / 'gateway.env').read_text(), LIVE_GATEWAY.replace('portal-rs-0123abcd', 'portal-rs-fedcba98'))
            self.assertEqual(stat.S_IMODE((portal / 'compose.yml').stat().st_mode), 0o640)
            self.assertEqual(stat.S_IMODE((portal / 'gateway.env').stat().st_mode), 0o600)
            self.assertEqual((state / (self.sha + '-portal.compose.before')).read_text(), LIVE_COMPOSE)
            self.assertEqual((state / (self.sha + '-portal.gateway.before')).read_text(), LIVE_GATEWAY)
            pending = json.loads((state / 'platform-pending.json').read_text())
            self.assertEqual(pending, {'component': 'platform', 'sha': self.sha, 'phase': 'schema_known',
                                       'schema': 'portal-rs-1', 'previous_image': 'old-id', 'platform_image': 'new-id'})
            self.assertEqual(images, {'oteryn-portal-portal-1': 'new-id', 'oteryn-portal-portal-worker-1': 'new-id',
                                      'oteryn-portal-portal-gateway-1': 'new-id'})
            self.assertEqual(sorted(p.name for p in portal.iterdir()), ['compose.yml', 'gateway.env'])

    def test_failed_migration_restores_the_previous_release_and_requires_reconciliation(self):
        with tempfile.TemporaryDirectory() as temporary:
            portal, state = self.host(temporary)
            with self.assertRaises(subprocess.CalledProcessError) as raised:
                self.deploy(portal, state, fail=['run', '--rm', '-T', 'portal', 'migrate'])
            argv = [args for args, _ in raised.exception.calls]
            self.assertEqual((portal / 'compose.yml').read_text(), LIVE_COMPOSE)
            self.assertEqual((portal / 'gateway.env').read_text(), LIVE_GATEWAY)
            self.assertEqual(stat.S_IMODE((portal / 'compose.yml').stat().st_mode), 0o640)
            # The previous image returns to service; the database is never touched.
            self.assertEqual(argv[-2][-5:], ['run', '--rm', '-T', 'portal', 'migrate'])
            self.assertEqual(argv[-1][-5:], ['up', '-d', 'portal', 'portal-worker', 'portal-gateway'])
            self.assertFalse(any('pg_restore' in a or 'dropdb' in a or 'down' in a for a in argv))
            self.assertEqual(json.loads((state / 'platform-pending.json').read_text())['phase'], 'schema_unknown')
            self.assertFalse((state / 'platform-current.json').exists())
            with patch.object(deploy, 'STATE', state), self.assertRaisesRegex(ValueError, 'reconciliation'):
                deploy.status('platform')

    def test_failed_readiness_restores_the_previous_release(self):
        with tempfile.TemporaryDirectory() as temporary:
            portal, state = self.host(temporary)
            def unready(image_id):
                raise ValueError('deployment readiness deadline exceeded')
            with self.assertRaisesRegex(ValueError, 'deadline'):
                self.deploy(portal, state, ready=unready)
            self.assertEqual((portal / 'compose.yml').read_text(), LIVE_COMPOSE)
            self.assertEqual((portal / 'gateway.env').read_text(), LIVE_GATEWAY)
            self.assertEqual(json.loads((state / 'platform-pending.json').read_text())['phase'], 'schema_known')

    def test_canceled_workflow_after_backup_changes_nothing_live(self):
        with tempfile.TemporaryDirectory() as temporary:
            portal, state = self.host(temporary)
            with self.assertRaisesRegex(ValueError, 'canceled') as raised:
                self.deploy(portal, state, reauthorize=[None, ValueError('workflow canceled')])
            argv = [args for args, _ in raised.exception.calls]
            self.assertIn(['backup'], argv)
            self.assertFalse(any('migrate' in a or 'up' in a for a in argv))
            self.assertEqual((portal / 'compose.yml').read_text(), LIVE_COMPOSE)
            self.assertFalse((state / 'platform-pending.json').exists())

    def test_unexpected_live_layout_is_refused_before_backup(self):
        layouts = [(LIVE_COMPOSE.replace('    image: oteryn-portal:0123abcd-arm64\n', '    image: other:1\n'), LIVE_GATEWAY),
                   (LIVE_COMPOSE + '  extra:\n    image: oteryn-portal:0123abcd-arm64\n', LIVE_GATEWAY),
                   (LIVE_COMPOSE, LIVE_GATEWAY + 'GATEWAY_VERSION=again\n')]
        for compose_text, gateway in layouts:
            with self.subTest(compose=compose_text, gateway=gateway), tempfile.TemporaryDirectory() as temporary:
                portal, state = self.host(temporary, gateway)
                (portal / 'compose.yml').write_text(compose_text)
                with self.assertRaisesRegex(ValueError, 'expected') as raised:
                    self.deploy(portal, state)
                self.assertNotIn(['backup'], [args for args, _ in raised.exception.calls])
                self.assertEqual((portal / 'compose.yml').read_text(), compose_text)
                self.assertEqual(list(state.iterdir()), [])

    def test_unprobeable_running_release_is_refused_before_backup(self):
        with tempfile.TemporaryDirectory() as temporary:
            portal, state = self.host(temporary)
            def unreachable():
                raise ValueError('portal readiness probe failed')
            with self.assertRaisesRegex(ValueError, 'readiness probe') as raised:
                self.deploy(portal, state, probe=unreachable)
            argv = [args for args, _ in raised.exception.calls]
            self.assertNotIn(['backup'], argv)
            self.assertFalse(any('migrate' in a or 'up' in a for a in argv))
            self.assertEqual((portal / 'compose.yml').read_text(), LIVE_COMPOSE)
            self.assertEqual(list(state.iterdir()), [])

    def test_gateway_without_a_version_line_is_left_unchanged(self):
        gateway = 'GATEWAY_LISTEN_ADDR=0.0.0.0:8081\n'
        with tempfile.TemporaryDirectory() as temporary:
            portal, state = self.host(temporary, gateway)
            self.deploy(portal, state)
            self.assertEqual((portal / 'gateway.env').read_text(), gateway)
            self.assertFalse((state / (self.sha + '-portal.gateway.before')).exists())

    def test_image_without_the_qualified_revision_is_refused_before_effects(self):
        with tempfile.TemporaryDirectory() as temporary:
            portal, state = self.host(temporary)
            calls = []
            image = {'Architecture': 'arm64', 'Id': 'new-id', 'Config': {'Labels': {}}}
            with patch.object(deploy, 'PORTAL', portal), patch.object(deploy, 'STATE', state), \
                 patch.object(deploy, 'run', side_effect=lambda args, **kw: calls.append(args)), \
                 patch.object(deploy, 'inspect_image', return_value=image), \
                 patch.object(deploy, 'contract', return_value='portal-rs-1'), \
                 patch.object(deploy, 'backup_platform') as backup, patch.object(deploy, 'reauthorize'):
                with self.assertRaisesRegex(ValueError, 'provenance'):
                    deploy.platform_deploy(Path('/archive'), self.sha, 100, 1)
            backup.assert_not_called()
            self.assertEqual(len(calls), 1)

    def containers(self, *, image='new-id', worker_image=None, worker_state=None):
        def container(name):
            info = {'State': {'Running': True}, 'Image': image, 'RestartCount': 0,
                    'NetworkSettings': {'Networks': {'oteryn-oracle_private': {'IPAddress': '172.18.0.42'}}}}
            if 'worker' in name:
                if worker_image:
                    info['Image'] = worker_image
                if worker_state:
                    info['State'] = worker_state
            return info
        return container

    def status(self, state, *, revision=None, allow_pending=False, **containers):
        opened = []
        class Response:
            status = 200
            def __enter__(self): return self
            def __exit__(self, *args): return False
        def urlopen(url, timeout):
            opened.append(url)
            return Response()
        labels = {'Config': {'Labels': {'org.opencontainers.image.revision': revision or self.sha}}}
        with patch.object(deploy, 'STATE', state), patch.object(deploy, 'inspect', side_effect=self.containers(**containers)), \
             patch.object(deploy, 'inspect_image', return_value=labels), patch.object(deploy, 'run') as command, \
             patch.object(deploy.urllib.request, 'urlopen', side_effect=urlopen):
            deploy.status('platform', allow_pending=allow_pending)
        command.assert_not_called()
        return opened

    def test_status_requires_the_recorded_release_in_every_role_and_a_ready_portal(self):
        with tempfile.TemporaryDirectory() as temporary:
            state = Path(temporary)
            with self.assertRaisesRegex(ValueError, 'no runner-deployed'):
                self.status(state)
            (state / 'platform-current.json').write_text(json.dumps({'sha': self.sha, 'phase': 'ready'}))
            self.assertEqual(self.status(state), ['http://172.18.0.42:8080/readyz'])
            for kwargs, message in [({'revision': 'b' * 40}, 'recorded release'),
                                    ({'worker_image': 'other-id'}, 'different images'),
                                    ({'worker_state': {'Running': False}}, 'worker'),
                                    ({'worker_state': {'Running': True, 'Restarting': True}}, 'worker'),
                                    ({'worker_state': {'Running': True, 'Health': {'Status': 'unhealthy'}}}, 'worker')]:
                with self.subTest(**kwargs), self.assertRaisesRegex(ValueError, message):
                    self.status(state, **kwargs)
            (state / 'platform-pending.json').write_text(json.dumps({'sha': 'c' * 40}))
            with self.assertRaisesRegex(ValueError, 'reconciliation'):
                self.status(state)
            # During a release, readiness is measured against the pending revision.
            self.status(state, revision='c' * 40, allow_pending=True)

    def test_unreachable_portal_is_not_ready(self):
        with tempfile.TemporaryDirectory() as temporary:
            state = Path(temporary)
            (state / 'platform-current.json').write_text(json.dumps({'sha': self.sha}))
            with patch.object(deploy, 'STATE', state), patch.object(deploy, 'inspect', side_effect=self.containers()), \
                 patch.object(deploy, 'inspect_image', return_value={'Config': {'Labels': {'org.opencontainers.image.revision': self.sha}}}), \
                 patch.object(deploy.urllib.request, 'urlopen', side_effect=ConnectionRefusedError()):
                with self.assertRaisesRegex(ValueError, 'readiness probe'):
                    deploy.status('platform')

    def test_readiness_needs_two_stable_samples_of_the_new_image(self):
        samples = iter([0, 1, 1])
        def container(name):
            return {'Image': 'new-id', 'RestartCount': next(samples) if 'worker' in name else 0}
        with patch.object(deploy, 'platform_status'), patch.object(deploy, 'inspect', side_effect=container), \
             patch.object(deploy.time, 'sleep') as sleep:
            deploy.wait_platform('new-id')
        self.assertEqual(sleep.call_count, 2)
        with patch.object(deploy, 'platform_status'), patch.object(deploy, 'inspect', return_value={'Image': 'old-id', 'RestartCount': 0}), \
             patch.object(deploy.time, 'sleep'):
            with self.assertRaisesRegex(ValueError, 'deadline'):
                deploy.wait_platform('new-id')

    def test_receipt_compares_every_portal_role_with_the_retained_image(self):
        with tempfile.TemporaryDirectory() as temporary:
            state = Path(temporary)
            (state / 'platform-current.json').write_text(json.dumps({'sha': self.sha, 'phase': 'ready', 'platform_image': 'new-id'}))
            seen = []
            def container(name):
                seen.append(name)
                return {'Image': 'other-id' if 'gateway' in name else 'new-id'}
            with patch.object(deploy, 'STATE', state), patch.object(deploy, 'inspect', side_effect=container):
                with self.assertRaisesRegex(ValueError, 'runtime images'):
                    deploy.verify_current_release('platform', self.sha)
            self.assertIn('oteryn-portal-portal-gateway-1', seen)

    def test_backup_dumps_inside_the_live_database_container(self):
        calls = []
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            def command(args, **kwargs):
                calls.append(args)
                if len(calls) == 1:
                    kwargs['stdout'].write(b'PGDMP-fixture-archive')
                else:
                    self.assertEqual(kwargs['stdin'].read(), b'PGDMP-fixture-archive')
            previous = os.umask(0o077)
            try:
                with patch.object(deploy, 'STATE', root), patch.object(deploy.subprocess, 'run', side_effect=command):
                    deploy.backup_platform(self.sha)
                self.assertEqual(stat.S_IMODE((root / (self.sha + '-platform.dump')).stat().st_mode), 0o600)
            finally:
                os.umask(previous)
        self.assertEqual(calls[0], ['docker', 'exec', 'oteryn-portal-portal-pg-1', 'sh', '-c',
                                    'pg_dump -U "$POSTGRES_USER" -d "${POSTGRES_DB:-$POSTGRES_USER}" -Fc'])
        self.assertEqual(calls[1], ['docker', 'exec', '-i', 'oteryn-portal-portal-pg-1', 'pg_restore', '--list'])

    def test_failed_backup_leaves_no_final_archive_and_same_sha_can_retry(self):
        for failure in ('pg_dump', 'pg_restore'):
            with self.subTest(failure=failure), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                path = root / (self.sha + '-platform.dump')
                failing = True

                def command(args, **kwargs):
                    operation = 'pg_restore' if 'pg_restore' in args else 'pg_dump'
                    if operation == 'pg_dump':
                        kwargs['stdout'].write(b'PGDMP-fixture-archive')
                    else:
                        self.assertFalse(path.exists(), 'archive published before validation')
                        self.assertEqual(kwargs['stdin'].read(), b'PGDMP-fixture-archive')
                    if failing and operation == failure:
                        raise subprocess.CalledProcessError(1, args)

                with patch.object(deploy, 'STATE', root), patch.object(deploy.subprocess, 'run', side_effect=command):
                    with self.assertRaises(subprocess.CalledProcessError):
                        deploy.backup_platform(self.sha)
                    self.assertEqual(list(root.iterdir()), [])
                    failing = False
                    deploy.backup_platform(self.sha)
                self.assertEqual(path.read_bytes(), b'PGDMP-fixture-archive')
                self.assertEqual(list(root.iterdir()), [path])

    def test_portal_image_recipe_matches_the_live_runtime_and_its_inputs_exist(self):
        checkout = Path(__file__).resolve().parents[2]
        recipe = (checkout / 'deploy/oracle/portal.Dockerfile').read_text()
        context = checkout / 'portal-rs'
        for line in recipe.splitlines():
            if line.startswith('COPY ') and '--from=' not in line:
                for source in line.split()[1:-1]:
                    if not source.startswith('--'):
                        self.assertTrue((context / source).exists(), source)
        for expected in ('FROM ubuntu:26.04', 'useradd -r -u 10001 portal', 'WORKDIR /app',
                         'COPY --from=build /out/oteryn-portal /app/oteryn-portal', 'static /app/static',
                         'migrations /app/migrations', 'USER portal', 'EXPOSE 8080 8091',
                         'ENTRYPOINT ["/app/oteryn-portal"]'):
            self.assertIn(expected, recipe)
        builder = (checkout / 'portal-rs/Dockerfile').read_text().split(' AS build', 1)[0].rsplit('FROM ', 1)[1]
        self.assertIn('FROM ' + builder + ' AS build', recipe)
        ignored = (context / '.dockerignore').read_text().split()
        self.assertFalse({'static', 'migrations', 'bin', 'crates', 'templates'} & set(ignored))


class GameBranchUnchanged(unittest.TestCase):
    # Source hashes of the shared and Game functions as reviewed; the same file
    # serves the Game forced command, so a Platform change must not move these.
    REVIEWED = {
        'parse': '4668faa3ef77617afc02609d647d56a106363f13c714a24ffb65c44f9a0b6cf7',
        'run': 'b4446e5599544fc74c497bdd5c224355cf8285cbf143452bc5bebbf5a69bc37b',
        'api': '46fe84673422b96e3ec33ca9372e34ec8f9383c6c082a420dd9cd12524994353',
        'qualify': 'c47dcd195b0d58caa797657877a43abd7c96786fceba2749a4b82b2ee68f4c88',
        'qualify_workflow': '7b5ec86dfdad08ff539682317822a3bb81f9abd1005b2442600184e0b33b2697',
        'source': 'e28e7a4f34b60934374bedb7f7bafd15702c0ed54c9483e75e595b421fe034f4',
        'atomic': 'cdf0f2b78cf04ddc69be17720336b5cd981c6fad2dd3027bf8f804143c149c66',
        'durable_replace': '15c1765449691b7ddda7785ef564d3e49325193e8caa27342751609bd0c47eef',
        'reauthorize': 'af126cd10bb2f6e3e2a8cb4b3bf977a932b70976fcd9a679531c38d27b60d145',
        'journal': '52217f6588382fc23ac58265c01109010994c50b91ee3b7555363d32da229230',
        'inspect': '7818ca397395df4b461b4dfbb7bf67c81fb2bbf3a26d80c53ac354dac839b333',
        'inspect_image': '31e6f71d5f300b649cd367c59baefd1e8186ed83a71f8cf44fff2f79fc155092',
        'game_deploy': '6aa4834fb49fe8d48cc86664c3a8f671125066a2bb027de393e2562e6c59a47f',
        'main': '70eccad29e6ddf46520901fec7e1ccb59e1a7ce205537219f51f7b7560cec2e0',
    }

    def test_shared_and_game_functions_are_unchanged(self):
        for name, digest in self.REVIEWED.items():
            with self.subTest(name=name):
                source = inspect.getsource(getattr(deploy, name))
                self.assertEqual(hashlib.sha256(source.encode()).hexdigest(), digest)

    def test_game_status_never_reaches_the_portal(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / 'state/game-controller').mkdir(parents=True)
            (root / 'state/game-controller/current.json').write_text('{"phase":"ready","cycle":"c1"}')
            seen = []
            def container(name):
                seen.append(name)
                return {'State': {'Running': True}, 'Config': {'Labels': {'io.oteryn.launch-cycle': 'c1'}}}
            with patch.object(deploy, 'ROOT', root), patch.object(deploy, 'STATE', root), patch.object(deploy, 'run') as command, \
                 patch.object(deploy, 'inspect', side_effect=container):
                deploy.status('game')
            self.assertEqual(seen, ['oteryn-oracle-runtime-game-1', 'oteryn-oracle-game-postgres-1'])
            command.assert_called_once_with(['systemctl', 'is-active', '--quiet', 'oteryn-game.service'])


if __name__ == '__main__':
    unittest.main()
