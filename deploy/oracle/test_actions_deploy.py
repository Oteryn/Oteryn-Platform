import importlib.util
from pathlib import Path
import unittest
import json
import tempfile
import subprocess
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('oracle_deploy', Path(__file__).with_name('actions-deploy.py'))
deploy = importlib.util.module_from_spec(spec)
spec.loader.exec_module(deploy)


class AuthorityBoundary(unittest.TestCase):
    def test_receipt_cannot_accept_a_different_healthy_image(self):
        sha = 'a' * 40
        with tempfile.TemporaryDirectory() as temporary:
            state = Path(temporary)
            (state / 'game-current.json').write_text(json.dumps({'sha': sha, 'phase': 'ready', 'image': 'expected'}))
            with patch.object(deploy, 'STATE', state), patch.object(deploy, 'inspect', return_value={'Image': 'other'}):
                with self.assertRaisesRegex(ValueError, 'runtime images'):
                    deploy.verify_current_release('game', sha)

    def test_running_edge_must_acquire_the_candidate_after_recreation(self):
        sha = 'a' * 40
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / 'deploy/oracle'
            source.mkdir(parents=True)
            (source / 'Caddyfile').write_text('qualified configuration\n')
            calls = []
            def command(args, **kw):
                calls.append(args)
                if args[-2:] == ['cat', '/etc/caddy/Caddyfile']:
                    return 'old configuration\n'
            info = {'Config': {'Labels': {'com.docker.compose.project.config_files': str(root / 'edge/compose.yml')}}}
            with patch.object(deploy, 'ROOT', root), patch.object(deploy, 'run', side_effect=command), patch.object(deploy, 'inspect', return_value=info), patch.object(deploy, 'atomic'), patch.object(deploy, 'journal') as journal, patch.object(deploy, 'reauthorize'):
                with self.assertRaisesRegex(ValueError, 'did not acquire'):
                    deploy.deploy_edge(root, sha, 100, 1)
                journal.assert_not_called()
            self.assertTrue(any('validate' in args for args in calls))
            self.assertTrue(any('--force-recreate' in args for args in calls))

    def test_platform_rechecks_authority_after_backup_before_disruption(self):
        sha = 'a' * 40
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / 'platform.env').write_text('PLATFORM_IMAGE=old\nGATEWAY_IMAGE=old\nGATEWAY_VERSION=old\n')
            calls = []
            old = {'Image': 'old-id', 'Config': {'Labels': {'com.docker.compose.project.config_files': '/old/deploy/synology/compose.yml'}}}
            with patch.object(deploy, 'ROOT', root), patch.object(deploy, 'inspect', return_value=old), patch.object(deploy, 'inspect_image', return_value={'Architecture': 'arm64', 'Id': 'new-id'}), patch.object(deploy, 'contract', return_value='schema-v1'), patch.object(deploy, 'run', side_effect=lambda args, **kw: calls.append(args)), patch.object(deploy, 'reauthorize', side_effect=[None, ValueError('workflow canceled')]):
                with self.assertRaisesRegex(ValueError, 'canceled'):
                    deploy.platform_deploy(root, sha, 100, 1)
            self.assertTrue(any('dump' in args for args in calls))
            self.assertFalse(any('stop' in args or 'down' in args for args in calls))

    def test_migration_failure_retains_unknown_schema_and_no_success_record(self):
        sha = 'a' * 40
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            state = root / 'actions'
            state.mkdir()
            (root / 'platform.env').write_text('PLATFORM_IMAGE=old\nGATEWAY_IMAGE=old\nGATEWAY_VERSION=old\n')
            old = {'Image': 'old-id', 'Config': {'Labels': {'com.docker.compose.project.config_files': '/old/deploy/synology/compose.yml'}}}
            def command(args, **kw):
                if args[:2] == ['bash', '-euc']:
                    raise subprocess.CalledProcessError(1, args)
            with patch.object(deploy, 'ROOT', root), patch.object(deploy, 'STATE', state), patch.object(deploy, 'inspect', return_value=old), patch.object(deploy, 'inspect_image', return_value={'Architecture': 'arm64', 'Id': 'new-id'}), patch.object(deploy, 'contract', return_value='schema-v1'), patch.object(deploy, 'run', side_effect=command), patch.object(deploy, 'reauthorize'):
                with self.assertRaises(subprocess.CalledProcessError):
                    deploy.platform_deploy(root, sha, 100, 1)
                self.assertEqual(json.loads((state / 'platform-pending.json').read_text())['phase'], 'schema_unknown')
                self.assertFalse((state / 'platform-current.json').exists())
                with self.assertRaisesRegex(ValueError, 'reconciliation'):
                    deploy.status('platform')

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

    def test_failed_scheduler_blocks_platform_readiness(self):
        def container(name):
            return {'State': {'Running': True, 'Health': {'Status': 'unhealthy' if 'scheduler' in name else 'healthy'}}}
        with patch.object(deploy, 'run'), patch.object(deploy, 'inspect', side_effect=container):
            with self.assertRaisesRegex(ValueError, 'scheduler'):
                deploy.status('platform')

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


if __name__ == '__main__':
    unittest.main()
