"""Exercise the staging host boundary without building a portal or touching Oracle."""
import fcntl
import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import DEFAULT, Mock, call, patch

HERE = Path(__file__).resolve().parent
SHA = 'a' * 40


def load_helper():
    spec = importlib.util.spec_from_file_location('staging_deploy', HERE / 'staging-deploy.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class StagingDeploy(unittest.TestCase):
    def setUp(self):
        self.staging = load_helper()
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        staging_root = patch.object(self.staging, 'ROOT', self.root)
        staging_root.start()
        self.addCleanup(staging_root.stop)

    def test_cli_rejects_bad_inputs_before_host_access(self):
        staging = self.staging
        self.assertEqual(staging.parse(['deploy', SHA, 'specs/foo.spec.mjs',
                                        '--pg-tests', 'oteryn-db,oteryn-web']),
                         ('deploy', SHA, 'specs/foo.spec.mjs', ['oteryn-db', 'oteryn-web']))
        self.assertEqual(staging.parse(['status'])[0], 'status')
        self.assertEqual(staging.parse(['logs'])[0], 'logs')
        with patch.object(staging, 'production_idle') as idle, patch('sys.stderr', new=io.StringIO()):
            for args in ([], ['status', SHA], ['logs', 'extra'], ['deploy', 'f' * 39],
                         ['deploy', SHA, 'a;id'], ['deploy', SHA, 'a\n'],
                         ['deploy', SHA, '--pg-tests', 'db,,web'],
                         ['deploy', SHA, '--pg-tests', 'db;id']):
                with self.subTest(args=args), self.assertRaises(SystemExit):
                    staging.main(args)
            idle.assert_not_called()
            self.assertEqual(list(self.root.iterdir()), [])

    def test_source_must_be_current_allowed_head_across_pages(self):
        staging = self.staging
        page = [{'name': 'feature/ref', 'commit': {'sha': SHA}}] * 100
        with patch.object(staging, 'branches', side_effect=[
            page, [{'name': 'staging/portal', 'commit': {'sha': SHA}}]
        ]) as branches:
            staging.qualify(SHA)
            self.assertEqual(branches.call_args_list, [call(1), call(2)])
        for name in ('main', 'feature/codex/portal', 'codex-old/portal'):
            with patch.object(staging, 'branches', return_value=[{'name': name, 'commit': {'sha': SHA}}]):
                with self.assertRaisesRegex(ValueError, 'current head'):
                    staging.qualify(SHA)

    def test_source_checks_identity_and_dirty_state(self):
        staging = self.staging
        for outputs, message in (((SHA, '? dirty'), 'dirty'), (('b' * 40, ''), 'identity')):
            with self.subTest(message=message), patch.object(staging, 'git', side_effect=outputs):
                source = self.root / 'src' / SHA
                source.mkdir(parents=True, exist_ok=True)
                with self.assertRaisesRegex(ValueError, message):
                    staging.source(SHA)
        with patch.object(staging, 'run', return_value=SHA) as command:
            staging.git(source, 'rev-parse', 'HEAD')
        self.assertIn('core.hooksPath=/dev/null', command.call_args.args[0])

    def test_checkout_fetches_only_exact_sha(self):
        staging = self.staging
        with patch.object(staging, 'git', side_effect=['', '', '', SHA, '', SHA, '']) as git:
            self.assertEqual(staging.source(SHA), self.root / 'src' / SHA)
        commands = [item.args[1:] for item in git.call_args_list]
        self.assertEqual(commands[:3], [
            ('init', '--quiet', '--template='),
            ('fetch', '--quiet', '--depth=1', '--no-tags', staging.REPO, SHA),
            ('checkout', '--quiet', '--detach', 'FETCH_HEAD'),
        ])
        self.assert_isolated(commands)

    def test_production_guard_reads_only_and_staging_lock_refuses_contention(self):
        staging = self.staging
        directory = self.root / 'deployment-fixture'
        directory.mkdir()
        lock = directory / 'deployment.lock'
        lock.touch()
        with patch.object(staging, 'PRODUCTION_STATE', directory), lock.open('rb') as held:
            fcntl.flock(held, fcntl.LOCK_EX | fcntl.LOCK_NB)
            with self.assertRaisesRegex(ValueError, 'production deploy'):
                staging.production_idle()
            fcntl.flock(held, fcntl.LOCK_UN)
            (directory / 'portal-pending.json').touch()
            with self.assertRaisesRegex(ValueError, 'pending'):
                staging.production_idle()
            (directory / 'portal-pending.json').unlink()
            staging.production_idle()
            fcntl.flock(held, fcntl.LOCK_EX | fcntl.LOCK_NB)
        self.assertEqual(sorted(p.name for p in directory.iterdir()), ['deployment.lock'])
        with staging.deploy_lock(), self.assertRaisesRegex(ValueError, 'staging deploy'):
            staging.deploy_lock()

    def test_build_uses_staging_cache_and_checks_provenance(self):
        staging = self.staging
        dockerfile = (HERE / 'staging.Dockerfile').read_text()
        for cache in ('oteryn-staging-cargo-registry', 'oteryn-staging-target'):
            self.assertIn('id=' + cache + ',', dockerfile)
        self.assertIn('ARG CARGO_BUILD_JOBS=2', dockerfile)
        with patch.object(staging, 'run', side_effect=[
            '', json.dumps([{'Architecture': 'arm64', 'Config': {'Labels': {staging.REVISION: SHA}}}]), '',
        ]) as run:
            staging.build(self.root / 'src' / SHA, SHA, 'runtime')
        command = run.call_args_list[0].args[0]
        self.assertIn(str(self.root / 'staging.Dockerfile'), command)
        self.assertIn('CARGO_BUILD_JOBS=2', command)
        self.assertIn('oteryn-staging:aaaaaaaa-arm64', command)
        self.assertEqual(run.call_args.args[0][-2:], ['oteryn-staging:aaaaaaaa-arm64', 'check-image'])
        self.assert_isolated([item.args[0] for item in run.call_args_list])
        for image in ({'Architecture': 'amd64', 'Config': {'Labels': {staging.REVISION: SHA}}},
                      {'Architecture': 'arm64', 'Config': {'Labels': {staging.REVISION: 'b' * 40}}}):
            with patch.object(staging, 'run', return_value=json.dumps([image])):
                with self.assertRaisesRegex(ValueError, 'provenance'):
                    staging.verify_image('oteryn-staging:aaaaaaaa-arm64', SHA)

    def test_daemon_queues_before_parent_returns_and_preserves_pending_status(self):
        staging = self.staging
        with patch.object(staging, 'production_idle'), patch.object(staging.os, 'fork', return_value=123), \
                patch('sys.stdout', new=io.StringIO()) as stdout:
            self.assertEqual(staging.launch(SHA, '', []), 0)
            self.assertIn(SHA, stdout.getvalue())
            self.assertEqual(staging.observe('status'), 2)
            self.assertIn('"phase": "queued"', stdout.getvalue())
            self.assertEqual(staging.pointer('latest'), SHA)
        with staging.deploy_lock():
            pass

    def test_daemon_child_detaches_and_reports_runner_result(self):
        staging = self.staging
        with patch.object(staging, 'production_idle'), patch.object(staging.os, 'fork', return_value=0), \
                patch.object(staging.os, 'setsid') as detach, patch.object(staging.os, 'dup2') as redirect, \
                patch.object(staging.os, 'chdir') as move, \
                patch.object(staging, 'deploy', return_value=True) as deploy, \
                patch.object(staging.os, '_exit', side_effect=SystemExit) as exit_process:
            with self.assertRaises(SystemExit):
                staging.launch(SHA, 'spec', ['oteryn-db'])
        detach.assert_called_once()
        move.assert_called_once_with(self.root)
        self.assertEqual([item.args[1] for item in redirect.call_args_list], [0, 1, 2])
        deploy.assert_called_once_with(self.root / 'run' / SHA, SHA, 'spec', ['oteryn-db'])
        self.assertEqual(exit_process.call_args_list[0], call(0))

    def test_compose_is_scoped_and_confined(self):
        staging = self.staging
        directory = self.root / 'run'
        directory.mkdir()
        staging.configure(directory, SHA)
        (self.root / 'staging.compose.yml').write_bytes((HERE / 'staging.compose.yml').read_bytes())
        raw = staging.compose(directory, '--profile', 'operations', 'config', '--format', 'json',
                              output=True)
        config = json.loads(raw)
        self.assertEqual(config['name'], 'oteryn-staging')
        self.assertEqual(config['networks']['net']['name'], 'oteryn-staging_net')
        self.assertEqual(config['networks']['net']['driver'], 'bridge')
        self.assertTrue(config['networks']['net']['internal'])
        services = config['services']
        self.assertEqual([(name, port['host_ip'], str(port['published']))
                          for name, service in services.items() for port in service.get('ports', [])],
                         [('ingress', '127.0.0.1', '18080')])
        core = ('portal', 'portal-public', 'worker', 'postgres', 'mail', 'ingress')
        for name, service in services.items():
            with self.subTest(service=name):
                self.assertTrue(service['read_only'])
                self.assertFalse(service.get('privileged', False))
                self.assertEqual(service['cap_drop'], ['ALL'])
                self.assertFalse(service.get('cap_add'))
                self.assertIn('no-new-privileges:true', service['security_opt'])
                self.assertGreater(float(service['cpus']), 0)
                self.assertGreater(int(service['mem_limit']), 0)
                for volume in service.get('volumes', []):
                    self.assertEqual(volume['type'], 'volume')
                if name != 'ingress':
                    self.assertEqual(set(service['networks']), {'net'})
        for job in ('browser', 'pg-tests'):
            self.assertAlmostEqual(sum(float(services[name]['cpus']) for name in (*core, job)), 2)
            self.assertLess(sum(int(services[name]['mem_limit']) for name in (*core, job)), 5 * 1024**3)
        self.assertIn('postgres:18.6@sha256:e3d24f6906d0760bf4da0dae872415391ededefad23fd471c47dc53b0ddf3c42',
                      services['postgres']['image'])
        self.assertEqual(services['mail']['environment']['MP_SMTP_REQUIRE_STARTTLS'], 'true')
        # The ingress exposes only a GET view of the catcher's API, never its UI.
        ingress = services['ingress']['command'][0]
        self.assertIn("const MAIL = '/__mailpit/api/v1/';", ingress)
        self.assertIn("request.method !== 'GET'", ingress)
        self.assertIn("hostname: 'mail', port: 8025, method: 'GET'", ingress)
        with patch.object(staging, 'run') as command:
            staging.compose(directory, 'ps')
        self.assertEqual(command.call_args.args[0][:4], ['docker', 'compose', '-p', 'oteryn-staging'])

    def test_env_is_fresh_private_and_partitioned(self):
        staging = self.staging
        _, _, _, first = staging.configure(self.root, SHA)
        _, _, _, second = staging.configure(self.root, SHA)
        self.assertFalse(set(first) & set(second))
        for path in self.root.glob('*.env'):
            self.assertEqual(path.stat().st_mode & 0o777, 0o600)
        portal = (self.root / 'portal.env').read_text()
        for flag in staging.OFF:
            self.assertIn(flag + '=false\n', portal)
        self.assertIn('OTERYN_ENV=testing\n', portal)
        self.assertIn('OTERYN_SMTP_HOST=mail\n', portal)
        self.assertNotIn('OTERYN_ORACLE_DEPLOY_IMAGE', portal)
        self.assertIn('/' + staging.PORTAL_DB, portal)
        self.assertIn('/' + staging.TEST_DB + '\n', (self.root / 'pg-tests.env').read_text())
        self.assertNotIn('/' + staging.PORTAL_DB, (self.root / 'pg-tests.env').read_text())

    def test_public_origin_defaults_to_loopback_and_rejects_bad_values(self):
        staging = self.staging
        staging.configure(self.root, SHA)
        public = (self.root / 'public.env').read_text()
        self.assertIn('OTERYN_PUBLIC_ORIGIN=' + staging.ORIGIN + '\n', public)
        self.assertIn('OTERYN_WEBAUTHN_RP_ID=localhost\n', public)
        (self.root / 'public-origin').write_text('https://staging.example.com\n')
        staging.configure(self.root, SHA)
        public = (self.root / 'public.env').read_text()
        self.assertIn('OTERYN_PUBLIC_ORIGIN=https://staging.example.com\n', public)
        self.assertIn('OTERYN_WEBAUTHN_RP_ID=staging.example.com\n', public)
        self.assertIn('OTERYN_NOTIFY_UNSUBSCRIBE_URL=https://staging.example.com/en/unsubscribe\n', public)
        self.assertIn('OTERYN_PUBLIC_ORIGIN=' + staging.ORIGIN + '\n',
                      (self.root / 'portal.env').read_text())
        for bad in ('http://staging.example.com', 'https://staging.example.com/x',
                    'https://user@evil.example', 'https://staging.example.com\nX=1', 'https://localhost'):
            with self.subTest(bad=bad):
                (self.root / 'public-origin').write_text(bad)
                with self.assertRaises(staging.Refusal):
                    staging.configure(self.root, SHA)

    def test_stack_replaces_only_exact_volumes_and_migrates_seeds_serves(self):
        staging = self.staging
        directory = self.root / 'run'
        directory.mkdir()
        with patch.object(staging, 'run', return_value=subprocess.CompletedProcess([], 0)) as command, \
                patch.object(staging, 'install_tls'), patch.object(staging, 'wait_ready') as ready:
            staging.replace_stack(directory, SHA, 'smtp', 'password')
        commands = [entry.args[0] for entry in command.call_args_list]
        compose = [item[10:] for item in commands if item[:2] == ['docker', 'compose']]
        self.assertEqual(compose[0][0], 'down')
        self.assertEqual([item[-1] for item in compose[2:4]], ['migrate', 'install-legal-pages'])
        self.assertEqual(compose[4][5:7], ['seed', 'account'])
        self.assertEqual(compose[-1], ['up', '-d', 'portal', 'portal-public', 'worker', 'ingress'])
        removals = [item for item in commands if item[:3] == ['docker', 'volume', 'rm']]
        self.assertEqual([item[-1] for item in removals], [
            'oteryn-staging_data', 'oteryn-staging_trust', 'oteryn-staging_mail_tls',
            'oteryn-staging_browser_tls',
        ])
        self.assertEqual(staging.pointer('current'), SHA)
        ready.assert_called_once()
        self.assert_isolated(commands)

    def assert_isolated(self, commands):
        denied = ('/opt/oteryn-portal', '/opt/oteryn-oracle', 'oteryn-oracle_private',
                  'oteryn-portal', 'platform.env', 'platform-migrate.env', 'prune')
        for args in commands:
            for arg in args:
                if arg == '--no-prune':
                    continue
                for forbidden in denied:
                    self.assertNotIn(forbidden, str(arg))
            if args[:2] == ['docker', 'compose']:
                self.assertEqual(args[2:4], ['-p', 'oteryn-staging'])

    def test_readiness_deadline_and_recovery(self):
        staging = self.staging
        with patch.object(staging, 'ready', side_effect=[False, True]) as probe, \
                patch.object(staging.time, 'sleep'):
            staging.wait_ready()
        self.assertEqual(probe.call_count, 2)
        with patch.object(staging, 'ready', return_value=False), \
                patch.object(staging.time, 'monotonic', side_effect=[0, 0, 2]), \
                patch.object(staging.time, 'sleep'), self.assertRaisesRegex(ValueError, 'deadline'):
            staging.wait_ready(1)

    def test_playwright_filters_literal_path_and_reports_assertions(self):
        staging = self.staging
        failed = {'status': 'unexpected', 'projectName': 'en-390',
                  'results': [{'error': {'message': 'Expected redaction\nsecond line'}}]}
        report = {'stats': {'expected': 3, 'unexpected': 1, 'flaky': 0, 'skipped': 2},
                  'suites': [{'suites': [{'specs': [{'title': 'can register', 'tests': [failed]}]}]}],
                  'errors': []}
        result = subprocess.CompletedProcess([], 1, stdout=json.dumps(report).encode())
        with patch.object(staging, 'run', return_value=result) as command:
            ok, summary = staging.playwright(self.root, 'specs/a.spec.mjs', ['redaction'])
        self.assertFalse(ok)
        self.assertEqual(summary, ['Playwright passed 3, failed 1, skipped 2',
                                   'can register [en-390] — Expected [redacted]'])
        self.assertEqual(command.call_args.args[0][-2:], ['--', r'specs/a\.spec\.mjs'])
        self.assert_isolated([command.call_args.args[0]])
        report.update(suites=[], stats={'expected': 1, 'unexpected': 0, 'flaky': 0, 'skipped': 0})
        self.assertTrue(staging.playwright_summary(
            subprocess.CompletedProcess([], 0, stdout=json.dumps(report).encode()), [])[0])
        self.assertFalse(staging.playwright_summary(
            subprocess.CompletedProcess([], 1, stdout=b'invalid JSON'), [])[0])
        dockerfile = (HERE / 'staging.Dockerfile').read_text()
        self.assertIn(f'mcr.microsoft.com/playwright:v{staging.PLAYWRIGHT_VERSION}-noble@sha256:', dockerfile)
        staging.build_context(HERE.parents[1])

    def test_pg_crates_run_separately_report_failures_and_drop_test_database(self):
        staging = self.staging

        def postgres(args, **kwargs):
            if 'pg-tests' in args:
                return subprocess.CompletedProcess(args, 1 if args[-1] == 'oteryn-db' else 0,
                                                   stdout=b'test migrations::locks ... FAILED\n')
            return subprocess.CompletedProcess(args, 0, stdout=b'')

        with patch.object(staging, 'run', side_effect=postgres) as command:
            ok, lines = staging.pg_tests(self.root, ['oteryn-db', 'oteryn-web'], 'private', ['private'])
        self.assertFalse(ok)
        self.assertEqual(lines[:3], ['PG oteryn-db FAIL', '  migrations::locks', 'PG oteryn-web PASS'])
        commands = [entry.args[0] for entry in command.call_args_list]
        create_role = command.call_args_list[0].kwargs['data']
        self.assertIn(b'LOGIN CREATEDB', create_role)
        self.assertIn(b'REVOKE CONNECT ON DATABASE portal_e2e_staging FROM PUBLIC', create_role)
        self.assertEqual([item[-1] for item in commands if 'dropdb' in item], [staging.TEST_DB] * 2)
        self.assertEqual([item[-3:] for item in commands if 'pg-tests' in item],
                         [['--locked', '-p', 'oteryn-db'], ['--locked', '-p', 'oteryn-web']])
        self.assertNotIn('private', repr(commands))
        self.assert_isolated(commands)

    def test_timeout_removes_only_the_named_test_container(self):
        staging = self.staging
        timeout = subprocess.TimeoutExpired(['docker'], 3600)
        with patch.object(staging, 'run', side_effect=[timeout, None]) as command:
            self.assertEqual(staging.playwright(self.root, '', []),
                             (False, ['Playwright timed out']))
        self.assertEqual(command.call_args.args[0],
                         ['docker', 'container', 'rm', '--force', 'oteryn-staging_browser_job'])
        self.assert_isolated([item.args[0] for item in command.call_args_list])

    def test_deploy_rechecks_boundaries_before_replacement_and_saves_both_results(self):
        staging = self.staging
        parent = self.root / 'run'
        directory = parent / SHA
        directory.mkdir(parents=True)
        checkout = self.root / 'src' / SHA
        with patch.multiple(staging, qualify=DEFAULT, production_idle=DEFAULT, source=DEFAULT,
                            build_context=DEFAULT, require_capacity=DEFAULT, build=DEFAULT,
                            replace_stack=DEFAULT, playwright=DEFAULT, pg_tests=DEFAULT) as methods:
            sequence = Mock()
            for name, method in methods.items():
                sequence.attach_mock(method, name)
            methods['source'].return_value = checkout
            methods['playwright'].return_value = (False, ['Playwright failed'])
            methods['pg_tests'].return_value = (True, ['PG oteryn-db PASS'])
            self.assertFalse(staging.deploy(directory, SHA, 'spec', ['oteryn-db']))
        self.assertEqual([item[0] for item in sequence.mock_calls], [
            'qualify', 'source', 'build_context', 'require_capacity', 'production_idle',
            'build', 'build', 'build', 'qualify', 'production_idle', 'replace_stack',
            'playwright', 'pg_tests',
        ])
        self.assertEqual((directory / 'summary.txt').read_text(), 'Playwright failed\nPG oteryn-db PASS\n')
        self.assertEqual(json.loads((directory / 'status.json').read_text())['phase'], 'failed')

    def test_owner_install_documents_the_direct_host_cli(self):
        staging_section = (HERE / 'README.md').read_text().split('## Staging\n', 1)[1]
        self.assertIn('install -m 0755 deploy/oracle/staging-deploy.py /opt/oteryn-staging/', staging_section)
        self.assertIn('install -m 0644 deploy/oracle/staging.Dockerfile deploy/oracle/staging.compose.yml', staging_section)
        for action in ('deploy', 'status', 'logs'):
            self.assertIn('python3 /opt/oteryn-staging/staging-deploy.py ' + action, staging_section)

    def test_retention_removes_only_older_verified_tags_and_sources(self):
        staging = self.staging
        (self.root / 'run').mkdir()
        (self.root / 'src').mkdir()
        heads = [char * 40 for char in 'abcde']
        for index, sha in enumerate(heads):
            directory = self.root / 'run' / sha
            directory.mkdir()
            (self.root / 'src' / sha).mkdir()
            status = directory / 'status.json'
            status.touch()
            os.utime(status, (index + 1, index + 1))
        (self.root / 'run' / 'current').write_text(heads[1])
        with patch.object(staging, 'run', return_value=subprocess.CompletedProcess([], 0)) as command, \
                patch.object(staging, 'verify_image') as verify:
            staging.retain(heads[-1])
        images = [staging.image_tag(heads[0], target) for target in ('runtime', 'browser', 'builder')]
        commands = [entry.args[0] for entry in command.call_args_list]
        self.assertEqual([item[-1] for item in commands if 'rm' in item], images)
        self.assertEqual(verify.call_args_list, [call(image, heads[0]) for image in images])
        self.assertEqual(sorted(p.name for p in (self.root / 'src').iterdir()), heads[1:])
        self.assert_isolated(commands)


if __name__ == '__main__':
    unittest.main()
